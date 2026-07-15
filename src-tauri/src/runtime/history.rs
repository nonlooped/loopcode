//! Reconstruct provider chat history from durable Core events.

use crate::domain::Event;
use crate::providers::chat::{ChatMessage, ToolCall};
use serde_json::Value;

/// Approximate char budget for prior history bodies (tool results truncated).
pub const HISTORY_CHAR_BUDGET: usize = 120_000;
const TOOL_RESULT_MAX_CHARS: usize = 8_000;

/// Build prior `ChatMessage`s for a new run from durable chat events.
///
/// Excludes the current run (its user turn is supplied separately via
/// [`crate::providers::chat::messages_from_context`]). Reconstructs assistant
/// tool_calls from either the assistant_message payload or following
/// `tool.proposal` events.
///
/// `tool.rejected` rows with `require_approval` are UI pause signals and are
/// never fed to the model as tool failures.
pub fn history_from_events(events: &[Event], exclude_run_id: &str) -> Vec<ChatMessage> {
    history_from_events_filtered(events, Some(exclude_run_id))
}

/// Rebuild provider history including `include_run_id` (used after approval grant).
pub fn history_including_run(events: &[Event], include_run_id: &str) -> Vec<ChatMessage> {
    let _ = include_run_id;
    history_from_events_filtered(events, None)
}

fn history_from_events_filtered(
    events: &[Event],
    exclude_run_id: Option<&str>,
) -> Vec<ChatMessage> {
    let mut out: Vec<ChatMessage> = Vec::new();
    let mut chars = 0usize;

    let mut i = 0;
    while i < events.len() {
        let e = &events[i];
        if exclude_run_id.is_some_and(|id| e.run_id.as_deref() == Some(id)) {
            i += 1;
            continue;
        }

        match e.kind.as_str() {
            "user_message" => {
                if let Some(text) = payload_text(&e.payload_json) {
                    chars = push_budgeted(
                        &mut out,
                        ChatMessage {
                            role: "user".into(),
                            content: Some(text),
                            tool_calls: vec![],
                            tool_call_id: None,
                            name: None,
                        },
                        chars,
                    );
                }
                i += 1;
            }
            "assistant_message" => {
                let (text, mut tool_calls) = payload_assistant(&e.payload_json);
                let mut j = i + 1;
                // Collect tool proposals that belong to this assistant turn when
                // the message payload omitted toolCalls (legacy events).
                if tool_calls.is_empty() {
                    while j < events.len() {
                        let te = &events[j];
                        if exclude_run_id.is_some_and(|id| te.run_id.as_deref() == Some(id)) {
                            break;
                        }
                        if te.kind == "user_message" || te.kind == "assistant_message" {
                            break;
                        }
                        if te.kind == "tool.proposal" {
                            if let Some(tc) = payload_tool_proposal(&te.payload_json) {
                                tool_calls.push(tc);
                            }
                        }
                        j += 1;
                    }
                }

                chars = push_budgeted(
                    &mut out,
                    ChatMessage {
                        role: "assistant".into(),
                        content: if text.is_empty() { None } else { Some(text) },
                        tool_calls: tool_calls.clone(),
                        tool_call_id: None,
                        name: None,
                    },
                    chars,
                );

                // Emit matching tool results in proposal order.
                if !tool_calls.is_empty() {
                    let mut results: Vec<(String, String, String)> = Vec::new();
                    let mut k = i + 1;
                    while k < events.len() {
                        let te = &events[k];
                        if exclude_run_id.is_some_and(|id| te.run_id.as_deref() == Some(id)) {
                            break;
                        }
                        if te.kind == "user_message" || te.kind == "assistant_message" {
                            break;
                        }
                        if te.kind == "tool.result" || te.kind == "tool.rejected" {
                            if let Some((id, name, body)) = payload_tool_result(&te.payload_json) {
                                results.push((id, name, truncate_chars(&body, TOOL_RESULT_MAX_CHARS)));
                            }
                        }
                        k += 1;
                    }
                    for tc in &tool_calls {
                        let body = results
                            .iter()
                            .find(|(id, _, _)| id == &tc.id)
                            .map(|(_, _, b)| b.clone())
                            .or_else(|| {
                                // Match by tool name if call ids drifted across legacy rows.
                                results
                                    .iter()
                                    .find(|(_, name, _)| name == &tc.name)
                                    .map(|(_, _, b)| b.clone())
                            })
                            .unwrap_or_else(|| "{\"status\":\"missing_result\"}".into());
                        chars = push_budgeted(
                            &mut out,
                            ChatMessage {
                                role: "tool".into(),
                                content: Some(body),
                                tool_calls: vec![],
                                tool_call_id: Some(tc.id.clone()),
                                name: Some(tc.name.clone()),
                            },
                            chars,
                        );
                    }
                    i = k;
                } else {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }

        if chars >= HISTORY_CHAR_BUDGET {
            // Drop oldest non-system messages until under budget (keep recent tail).
            while chars > HISTORY_CHAR_BUDGET && out.len() > 2 {
                let removed = out.remove(0);
                chars = chars.saturating_sub(estimate_chars(&removed));
            }
            break;
        }
    }

    // Drop a leading tool message (invalid without preceding assistant tool_calls).
    while out.first().is_some_and(|m| m.role == "tool") {
        let removed = out.remove(0);
        chars = chars.saturating_sub(estimate_chars(&removed));
    }

    out
}

fn push_budgeted(out: &mut Vec<ChatMessage>, msg: ChatMessage, chars: usize) -> usize {
    let add = estimate_chars(&msg);
    out.push(msg);
    chars.saturating_add(add)
}

fn estimate_chars(m: &ChatMessage) -> usize {
    m.content.as_ref().map(|c| c.len()).unwrap_or(0)
        + m.tool_calls
            .iter()
            .map(|t| t.name.len() + t.arguments.to_string().len())
            .sum::<usize>()
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…[truncated]", &s[..max])
    }
}

fn payload_text(payload_json: &str) -> Option<String> {
    let payload = crate::security::redact_payload_json(payload_json);
    let v: Value = serde_json::from_str(&payload).ok()?;
    v.get("text")
        .and_then(|t| t.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

fn payload_assistant(payload_json: &str) -> (String, Vec<ToolCall>) {
    let payload = crate::security::redact_payload_json(payload_json);
    let Ok(v) = serde_json::from_str::<Value>(&payload) else {
        return (String::new(), vec![]);
    };
    let text = v
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let mut tool_calls = Vec::new();
    if let Some(arr) = v.get("toolCalls").or_else(|| v.get("tool_calls")).and_then(|a| a.as_array())
    {
        for item in arr {
            let id = item
                .get("id")
                .or_else(|| item.get("callId"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let name = item
                .get("name")
                .or_else(|| item.get("toolName"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let arguments = item
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));
            if !id.is_empty() && !name.is_empty() {
                tool_calls.push(ToolCall {
                    id,
                    name,
                    arguments,
                });
            }
        }
    }
    (text, tool_calls)
}

fn payload_tool_proposal(payload_json: &str) -> Option<ToolCall> {
    let payload = crate::security::redact_payload_json(payload_json);
    let v: Value = serde_json::from_str(&payload).ok()?;
    let id = v
        .get("callId")
        .or_else(|| v.get("id"))
        .and_then(|x| x.as_str())?
        .to_string();
    let name = v
        .get("toolName")
        .or_else(|| v.get("name"))
        .and_then(|x| x.as_str())?
        .to_string();
    let arguments = v
        .get("arguments")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    Some(ToolCall {
        id,
        name,
        arguments,
    })
}

fn payload_tool_result(payload_json: &str) -> Option<(String, String, String)> {
    let payload = crate::security::redact_payload_json(payload_json);
    let v: Value = serde_json::from_str(&payload).ok()?;
    let status = v
        .get("content")
        .and_then(|c| c.get("status"))
        .and_then(|s| s.as_str())
        .or_else(|| v.get("status").and_then(|s| s.as_str()));
    // Approval pauses are not model-facing tool failures — skip until a real result exists.
    if status == Some("require_approval") {
        return None;
    }
    let id = v
        .get("callId")
        .or_else(|| v.get("id"))
        .and_then(|x| x.as_str())?
        .to_string();
    let name = v
        .get("toolName")
        .or_else(|| v.get("name"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let body = if let Some(c) = v.get("content") {
        serde_json::to_string(c).unwrap_or_else(|_| c.to_string())
    } else {
        payload
    };
    Some((id, name, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn ev(kind: &str, run: &str, payload: Value, seq: i64) -> Event {
        Event {
            id: format!("e{seq}"),
            chat_id: "c1".into(),
            run_id: Some(run.into()),
            seq,
            kind: kind.into(),
            payload_json: payload.to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn reconstructs_user_assistant_tool_chain() {
        let events = vec![
            ev(
                "user_message",
                "run-a",
                serde_json::json!({"text": "list files"}),
                1,
            ),
            ev(
                "assistant_message",
                "run-a",
                serde_json::json!({
                    "text": "looking",
                    "toolCalls": [{
                        "id": "call_1",
                        "name": "glob",
                        "arguments": {"pattern": "**/*"}
                    }]
                }),
                2,
            ),
            ev(
                "tool.result",
                "run-a",
                serde_json::json!({
                    "callId": "call_1",
                    "toolName": "glob",
                    "content": {"entries": []}
                }),
                3,
            ),
            ev(
                "assistant_message",
                "run-a",
                serde_json::json!({"text": "done"}),
                4,
            ),
            ev(
                "user_message",
                "run-b",
                serde_json::json!({"text": "now edit"}),
                5,
            ),
        ];
        let prior = history_from_events(&events, "run-b");
        assert_eq!(prior.len(), 4);
        assert_eq!(prior[0].role, "user");
        assert_eq!(prior[1].role, "assistant");
        assert_eq!(prior[1].tool_calls.len(), 1);
        assert_eq!(prior[2].role, "tool");
        assert_eq!(prior[2].tool_call_id.as_deref(), Some("call_1"));
        assert_eq!(prior[3].role, "assistant");
        assert!(!prior.iter().any(|m| m.content.as_deref() == Some("now edit")));
    }

    #[test]
    fn skips_require_approval_rejects_in_model_history() {
        let events = vec![
            ev(
                "user_message",
                "run-a",
                serde_json::json!({"text": "fetch pnpm"}),
                1,
            ),
            ev(
                "assistant_message",
                "run-a",
                serde_json::json!({
                    "text": "",
                    "toolCalls": [{
                        "id": "call_1",
                        "name": "webfetch",
                        "arguments": {"url": "https://example.com"}
                    }]
                }),
                2,
            ),
            ev(
                "tool.rejected",
                "run-a",
                serde_json::json!({
                    "callId": "call_1",
                    "toolName": "webfetch",
                    "outcome": "pending_approval",
                    "content": {
                        "status": "require_approval",
                        "actionClass": "network",
                        "reason": "action_class=network requires approval"
                    },
                    "error": null
                }),
                3,
            ),
            ev(
                "tool.result",
                "run-a",
                serde_json::json!({
                    "callId": "call_1",
                    "toolName": "webfetch",
                    "content": {"body": "ok", "status": "ok"}
                }),
                4,
            ),
        ];
        let msgs = history_including_run(&events, "run-a");
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[2].role, "tool");
        let body = msgs[2].content.as_deref().unwrap_or("");
        assert!(body.contains("ok"), "{body}");
        assert!(!body.contains("require_approval"), "{body}");
    }
}
