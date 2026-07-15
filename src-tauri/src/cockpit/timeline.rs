//! Timeline projection — pure mapping from Core semantic events to UI items.

use crate::domain::{Event, Run, RunStatus};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineItemKind {
    UserMessage,
    AssistantMessage,
    ToolProposal,
    ToolResult,
    ToolRejected,
    Approval,
    Error,
    Usage,
    Lifecycle,
    System,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineViewState {
    /// No chat selected / no project.
    EmptyStart,
    /// Chat exists but no events yet.
    EmptyChat,
    Loading,
    Populated,
    Error,
    Suspended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineItem {
    pub id: String,
    pub seq: i64,
    pub kind: TimelineItemKind,
    pub title: String,
    pub body: String,
    pub run_id: Option<String>,
    pub event_kind: String,
    /// For approval cards: action class if present in payload.
    pub action_class: Option<String>,
    pub tool_name: Option<String>,
    pub needs_approval: bool,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineProjection {
    pub state: TimelineViewState,
    pub items: Vec<TimelineItem>,
    pub status_label: String,
    pub cost_display: String,
    pub tokens_display: String,
    pub active_run_id: Option<String>,
    pub active_run_status: Option<String>,
}

/// Project persisted events (+ optional runs) into timeline UI state.
///
/// This is the shipped projection function tests must call.
pub fn project_timeline(
    events: &[Event],
    runs: &[Run],
    chat_selected: bool,
    loading: bool,
    load_error: Option<&str>,
) -> TimelineProjection {
    if loading {
        return TimelineProjection {
            state: TimelineViewState::Loading,
            items: vec![],
            status_label: "Loading…".into(),
            cost_display: "$—".into(),
            tokens_display: "— tok".into(),
            active_run_id: None,
            active_run_status: None,
        };
    }
    if let Some(err) = load_error {
        return TimelineProjection {
            state: TimelineViewState::Error,
            items: vec![],
            status_label: format!("Error: {err}"),
            cost_display: "$—".into(),
            tokens_display: "— tok".into(),
            active_run_id: None,
            active_run_status: None,
        };
    }
    if !chat_selected {
        return TimelineProjection {
            state: TimelineViewState::EmptyStart,
            items: vec![],
            status_label: "Open a project and start a chat".into(),
            cost_display: "$0.00".into(),
            tokens_display: "0 tok".into(),
            active_run_id: None,
            active_run_status: None,
        };
    }

    // A suspended run is durable history from an interrupted app session, not
    // live work. Reporting it as active traps the composer in its Stop state
    // after restart even though there is no runtime left to cancel.
    let active = runs.iter().find(|r| r.status.is_active());
    let suspended = runs.iter().any(|r| r.status == RunStatus::Suspended);

    if events.is_empty() {
        return TimelineProjection {
            state: if suspended {
                TimelineViewState::Suspended
            } else {
                TimelineViewState::EmptyChat
            },
            items: vec![],
            status_label: if suspended {
                "The previous run was interrupted. You can continue below.".into()
            } else {
                "Start a chat — type below".into()
            },
            cost_display: "$0.00".into(),
            tokens_display: "0 tok".into(),
            active_run_id: active.map(|r| r.id.clone()),
            active_run_status: active.map(|r| r.status.as_str().to_string()),
        };
    }

    let mut items: Vec<TimelineItem> = events.iter().map(event_to_item).collect();
    items.sort_by_key(|i| i.seq);
    // Pending cards are derived from immutable tool.rejected events. Clear
    // stale needs_approval once the run leaves approval_waiting (granted,
    // denied, completed, suspended after restart, etc.), and keep only the
    // latest pending item per waiting run so earlier decisions don't reappear.
    resolve_pending_approvals(&mut items, runs);

    let (cost, tokens) = sum_usage(events);
    let state = if suspended && active.is_none() {
        TimelineViewState::Suspended
    } else {
        TimelineViewState::Populated
    };

    let status_label = match active.map(|r| r.status) {
        Some(RunStatus::ApprovalWaiting) => "Waiting for approval".into(),
        Some(RunStatus::ModelActive) | Some(RunStatus::ToolActive) | Some(RunStatus::Preparing) => {
            "Running".into()
        }
        Some(RunStatus::Cancelling) => "Cancelling…".into(),
        // `active` deliberately excludes suspended runs; retain this arm so
        // the lifecycle mapping stays total if the selection above changes.
        Some(RunStatus::Suspended) => {
            "The previous run was interrupted. You can continue below.".into()
        }
        Some(RunStatus::Completed) => "Completed".into(),
        Some(RunStatus::Failed) => "Failed".into(),
        Some(RunStatus::Cancelled) => "Cancelled".into(),
        Some(RunStatus::Queued) => "Queued".into(),
        None => {
            if suspended {
                "The previous run was interrupted. You can continue below.".into()
            } else if let Some(last) = runs.last() {
                last.status.as_str().to_string()
            } else {
                "Idle".into()
            }
        }
    };

    TimelineProjection {
        state,
        items,
        status_label,
        cost_display: format!("${cost:.4}"),
        tokens_display: format!("{tokens} tok"),
        active_run_id: active.map(|r| r.id.clone()),
        active_run_status: active.map(|r| r.status.as_str().to_string()),
    }
}

fn event_to_item(e: &Event) -> TimelineItem {
    let payload: Value = serde_json::from_str(&e.payload_json).unwrap_or(Value::Null);
    let (kind, title, body, needs_approval, action_class, tool_name) =
        classify_kind(&e.kind, &payload);

    TimelineItem {
        id: e.id.clone(),
        seq: e.seq,
        kind,
        title,
        body,
        run_id: e.run_id.clone(),
        event_kind: e.kind.clone(),
        action_class,
        tool_name,
        needs_approval,
        payload,
    }
}

/// Drop actionable pending flags that no longer match durable run state.
///
/// User grants land in audit_log / in-memory GrantTable — not in the events
/// stream — so a raw tool.rejected with `require_approval` would otherwise
/// keep showing Accept/Deny after the decision (or after restart suspension).
fn resolve_pending_approvals(items: &mut [TimelineItem], runs: &[Run]) {
    let waiting: std::collections::HashSet<&str> = runs
        .iter()
        .filter(|r| r.status == RunStatus::ApprovalWaiting)
        .map(|r| r.id.as_str())
        .collect();

    // Highest-seq pending item per run that is still approval_waiting.
    let mut latest_pending: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    for item in items.iter() {
        if !item.needs_approval {
            continue;
        }
        let Some(run_id) = item.run_id.as_deref() else {
            continue;
        };
        if !waiting.contains(run_id) {
            continue;
        }
        let entry = latest_pending.entry(run_id.to_string()).or_insert(item.seq);
        if item.seq > *entry {
            *entry = item.seq;
        }
    }

    for item in items.iter_mut() {
        if !item.needs_approval {
            continue;
        }
        let keep = match item.run_id.as_deref() {
            Some(run_id) if waiting.contains(run_id) => {
                latest_pending.get(run_id).copied() == Some(item.seq)
            }
            // Orphan pending rows (no run id / unknown run) are not actionable.
            _ => false,
        };
        if !keep {
            item.needs_approval = false;
        }
    }
}

/// Nested `content` object from Core `tool.rejected` / `tool.result` envelopes.
fn payload_content(payload: &Value) -> Option<&Value> {
    payload.get("content").filter(|c| c.is_object())
}

fn str_field(v: &Value, camel: &str, snake: &str) -> Option<String> {
    v.get(camel)
        .or_else(|| v.get(snake))
        .and_then(|x| x.as_str())
        .map(str::to_string)
}

/// Resolve tool name from top-level envelope or nested content (real Core shape).
fn resolve_tool_name(payload: &Value) -> Option<String> {
    str_field(payload, "toolName", "tool_name").or_else(|| {
        payload_content(payload).and_then(|c| str_field(c, "toolName", "tool_name"))
    })
}

/// Resolve action class from top-level or nested `content` (engine nests under content).
fn resolve_action_class(payload: &Value) -> Option<String> {
    str_field(payload, "actionClass", "action_class").or_else(|| {
        payload_content(payload).and_then(|c| str_field(c, "actionClass", "action_class"))
    })
}

/// Status string: top-level or nested `content.status` (real tool.rejected envelope).
fn resolve_status(payload: &Value) -> Option<&str> {
    payload
        .get("status")
        .and_then(|s| s.as_str())
        .or_else(|| {
            payload_content(payload)
                .and_then(|c| c.get("status"))
                .and_then(|s| s.as_str())
        })
}

fn resolve_reason(payload: &Value) -> Option<&str> {
    payload
        .get("reason")
        .and_then(|r| r.as_str())
        .or_else(|| {
            payload_content(payload)
                .and_then(|c| c.get("reason"))
                .and_then(|r| r.as_str())
        })
        .or_else(|| payload.get("error").and_then(|e| e.as_str()))
}

fn classify_kind(
    event_kind: &str,
    payload: &Value,
) -> (
    TimelineItemKind,
    String,
    String,
    bool,
    Option<String>,
    Option<String>,
) {
    // Lifecycle first: `run.lifecycle.approval_waiting` must NOT become Approval.
    if event_kind.starts_with("run.lifecycle") || event_kind.contains("lifecycle") {
        return (
            TimelineItemKind::Lifecycle,
            "Lifecycle".into(),
            payload
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or(event_kind)
                .to_string(),
            false,
            None,
            None,
        );
    }

    let tool = resolve_tool_name(payload);
    let action = resolve_action_class(payload);

    // Explicit audit/approval decision events only (not lifecycle kinds).
    if event_kind == "audit.approval" || event_kind.starts_with("approval.") {
        let decision = payload
            .get("decision")
            .and_then(|d| d.as_str())
            .unwrap_or("");
        let needs = decision == "require_approval";
        return (
            TimelineItemKind::Approval,
            if needs {
                "Approval required".into()
            } else {
                format!("Approval: {decision}")
            },
            resolve_reason(payload).unwrap_or("").to_string(),
            needs,
            action,
            tool,
        );
    }

    if event_kind.starts_with("tool.proposal") {
        return (
            TimelineItemKind::ToolProposal,
            format!("Tool: {}", tool.as_deref().unwrap_or("?")),
            payload
                .get("arguments")
                .map(|a| a.to_string())
                .unwrap_or_default(),
            false,
            action,
            tool,
        );
    }
    if event_kind.starts_with("tool.result") {
        return (
            TimelineItemKind::ToolResult,
            format!("Tool result: {}", tool.as_deref().unwrap_or("?")),
            payload
                .get("content")
                .map(|c| c.to_string())
                .unwrap_or_else(|| payload.to_string()),
            false,
            action,
            tool,
        );
    }

    // Real Core emit shape for gated tools:
    // { toolName, content: { status: "require_approval", actionClass, reason }, error }
    let status = resolve_status(payload);
    if event_kind.starts_with("tool.rejected") || status == Some("require_approval") {
        let status_s = status.unwrap_or("rejected");
        let needs = status_s == "require_approval";
        return (
            if needs {
                TimelineItemKind::Approval
            } else {
                TimelineItemKind::ToolRejected
            },
            if needs {
                format!(
                    "Approve: {}",
                    tool.as_deref().unwrap_or(action.as_deref().unwrap_or("action"))
                )
            } else {
                format!("Rejected: {}", tool.as_deref().unwrap_or("tool"))
            },
            resolve_reason(payload).unwrap_or(status_s).to_string(),
            needs,
            action,
            tool,
        );
    }
    if event_kind.contains("user_message") || event_kind.ends_with("user") {
        let text = payload
            .get("text")
            .or_else(|| payload.get("content").filter(|c| c.is_string()))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        return (
            TimelineItemKind::UserMessage,
            "You".into(),
            text,
            false,
            None,
            None,
        );
    }
    if event_kind.contains("assistant") {
        let text = payload
            .get("text")
            .or_else(|| payload.get("content").filter(|c| c.is_string()))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        return (
            TimelineItemKind::AssistantMessage,
            "Assistant".into(),
            text,
            false,
            None,
            None,
        );
    }
    if event_kind.contains("error") || event_kind.ends_with(".error") {
        // Prefer Core RuntimeError + retryPlan fields (not hard-coded chrome strings).
        let category = payload
            .get("category")
            .and_then(|c| c.as_str())
            .unwrap_or("error");
        let message = payload
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("");
        let detail = payload
            .get("detail")
            .and_then(|d| d.as_str())
            .unwrap_or("");
        let retryability = payload
            .get("retryability")
            .and_then(|r| r.as_str())
            .or_else(|| {
                payload
                    .pointer("/retryPlan/retryability")
                    .and_then(|r| r.as_str())
            })
            .unwrap_or("");
        let next_action = payload
            .pointer("/retryPlan/nextAction")
            .or_else(|| payload.pointer("/retryPlan/next_action"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        let title = if category.is_empty() {
            "Error".into()
        } else {
            format!("Error: {category}")
        };
        let body = if detail.is_empty() {
            format!("{message}\nretryability={retryability}\nnext_action={next_action}")
        } else {
            format!(
                "{message}\n{detail}\nretryability={retryability}\nnext_action={next_action}"
            )
        };
        return (
            TimelineItemKind::Error,
            title,
            body,
            false,
            None,
            None,
        );
    }
    if event_kind == "run.retry_planned" || event_kind.contains("retry_planned") {
        let category = payload
            .get("category")
            .and_then(|c| c.as_str())
            .unwrap_or("retry");
        let next = payload
            .get("nextAction")
            .or_else(|| payload.pointer("/retryPlan/nextAction"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        let msg = payload
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("");
        return (
            TimelineItemKind::System,
            format!("Retry: {category}"),
            format!("{msg}\nnext_action={next}"),
            false,
            None,
            None,
        );
    }
    if event_kind == "run.limit_warning" || event_kind.contains("limit_warning") {
        let msg = payload
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("limit warning");
        return (
            TimelineItemKind::System,
            "Limit warning".into(),
            msg.to_string(),
            false,
            None,
            None,
        );
    }
    if event_kind.contains("usage") {
        return (
            TimelineItemKind::Usage,
            "Usage".into(),
            payload.to_string(),
            false,
            None,
            None,
        );
    }
    (
        TimelineItemKind::Unknown,
        event_kind.to_string(),
        payload.to_string(),
        false,
        None,
        None,
    )
}

fn sum_usage(events: &[Event]) -> (f64, i64) {
    let mut cost_micros: i64 = 0;
    let mut tokens: i64 = 0;
    for e in events {
        if !e.kind.contains("usage") && e.kind != "run.usage" {
            // Also scan tool results for usage fields
        }
        if let Ok(v) = serde_json::from_str::<Value>(&e.payload_json) {
            if let Some(u) = v.get("usage") {
                tokens += u
                    .get("total_tokens")
                    .or_else(|| u.get("totalTokens"))
                    .and_then(|t| t.as_i64())
                    .unwrap_or(0);
                tokens += u
                    .get("input_tokens")
                    .and_then(|t| t.as_i64())
                    .unwrap_or(0)
                    + u.get("output_tokens")
                        .and_then(|t| t.as_i64())
                        .unwrap_or(0);
            }
            if let Some(c) = v.get("costMicros").and_then(|c| c.as_i64()) {
                cost_micros += c;
            }
            if let Some(c) = v.get("cost_micros").and_then(|c| c.as_i64()) {
                cost_micros += c;
            }
            // Direct usage ledger shape in event
            if e.kind.contains("usage") {
                tokens += v.get("input_tokens").and_then(|t| t.as_i64()).unwrap_or(0)
                    + v.get("output_tokens").and_then(|t| t.as_i64()).unwrap_or(0);
                cost_micros += v.get("cost_micros").and_then(|c| c.as_i64()).unwrap_or(0);
            }
        }
    }
    (cost_micros as f64 / 1_000_000.0, tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn ev(seq: i64, kind: &str, payload: &str) -> Event {
        Event {
            id: format!("e{seq}"),
            chat_id: "c1".into(),
            run_id: Some("r1".into()),
            seq,
            kind: kind.into(),
            payload_json: payload.into(),
            created_at: Utc::now(),
        }
    }

    fn run(status: RunStatus) -> Run {
        Run {
            id: "r1".into(),
            chat_id: "c1".into(),
            mode: "build".into(),
            model: Some("m".into()),
            adapter: Some("mock".into()),
            policy_version: None,
            status,
            predecessor_run_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            finished_at: None,
        }
    }

    #[test]
    fn empty_chat_distinct_from_start() {
        let empty_start = project_timeline(&[], &[], false, false, None);
        assert_eq!(empty_start.state, TimelineViewState::EmptyStart);
        let empty_chat = project_timeline(&[], &[], true, false, None);
        assert_eq!(empty_chat.state, TimelineViewState::EmptyChat);
        assert_ne!(empty_start.status_label, empty_chat.status_label);
    }

    #[test]
    fn suspended_run_does_not_lock_the_composer_after_restart() {
        let p = project_timeline(
            &[ev(1, "user_message", r#"{"text":"hi"}"#)],
            &[run(RunStatus::Suspended)],
            true,
            false,
            None,
        );

        assert_eq!(p.state, TimelineViewState::Suspended);
        assert_eq!(p.active_run_id, None);
        assert_eq!(p.active_run_status, None);
        assert_eq!(
            p.status_label,
            "The previous run was interrupted. You can continue below."
        );
    }

    #[test]
    fn projects_messages_tools_approvals() {
        // Nested content matches real AgentRuntime::propose_tool emit envelope.
        let events = vec![
            ev(1, "user_message", r#"{"text":"hi"}"#),
            ev(
                2,
                "tool.proposal",
                r#"{"toolName":"exec","arguments":{},"actionClass":"shell"}"#,
            ),
            ev(
                3,
                "tool.rejected",
                r#"{"callId":"c1","toolName":"exec","executed":false,"content":{"status":"require_approval","actionClass":"shell","reason":"needs approval"},"error":"needs approval"}"#,
            ),
        ];
        let runs = vec![run(RunStatus::ApprovalWaiting)];
        let p = project_timeline(&events, &runs, true, false, None);
        assert_eq!(p.state, TimelineViewState::Populated);
        assert!(p.items.iter().any(|i| i.kind == TimelineItemKind::UserMessage));
        assert!(p.items.iter().any(|i| i.kind == TimelineItemKind::ToolProposal));
        let approval = p
            .items
            .iter()
            .find(|i| i.needs_approval)
            .expect("nested require_approval must set needs_approval");
        assert_eq!(approval.kind, TimelineItemKind::Approval);
        assert_eq!(approval.action_class.as_deref(), Some("shell"));
        assert_eq!(approval.tool_name.as_deref(), Some("exec"));
    }

    #[test]
    fn lifecycle_approval_waiting_is_lifecycle_not_approval_card() {
        let events = vec![ev(
            1,
            "run.lifecycle.approval_waiting",
            r#"{"runId":"r1","status":"approval_waiting","from":"tool_active"}"#,
        )];
        let p = project_timeline(&events, &[], true, false, None);
        assert_eq!(p.items.len(), 1);
        assert_eq!(p.items[0].kind, TimelineItemKind::Lifecycle);
        assert!(!p.items[0].needs_approval);
        assert!(p.items[0].action_class.is_none());
    }

    #[test]
    fn flat_require_approval_payload_still_works() {
        // Flat shape (tests / older fixtures) remains supported.
        let events = vec![ev(
            1,
            "tool.rejected",
            r#"{"status":"require_approval","actionClass":"shell","toolName":"exec","reason":"x"}"#,
        )];
        let runs = vec![run(RunStatus::ApprovalWaiting)];
        let p = project_timeline(&events, &runs, true, false, None);
        assert!(p.items[0].needs_approval);
        assert_eq!(p.items[0].action_class.as_deref(), Some("shell"));
    }

    #[test]
    fn clears_needs_approval_when_run_left_waiting() {
        let events = vec![ev(
            1,
            "tool.rejected",
            r#"{"callId":"c1","toolName":"exec","executed":false,"content":{"status":"require_approval","actionClass":"shell","reason":"needs approval"},"error":"needs approval"}"#,
        )];
        for status in [
            RunStatus::Completed,
            RunStatus::Failed,
            RunStatus::Suspended,
            RunStatus::ModelActive,
            RunStatus::Cancelled,
        ] {
            let p = project_timeline(&events, &[run(status)], true, false, None);
            assert!(
                !p.items[0].needs_approval,
                "status {status:?} must clear needs_approval"
            );
            assert_eq!(p.items[0].kind, TimelineItemKind::Approval);
        }
    }

    #[test]
    fn only_latest_pending_approval_stays_actionable() {
        let events = vec![
            ev(
                1,
                "tool.rejected",
                r#"{"callId":"c1","toolName":"exec","executed":false,"content":{"status":"require_approval","actionClass":"shell","reason":"first"},"error":"first"}"#,
            ),
            ev(
                2,
                "tool.rejected",
                r#"{"callId":"c2","toolName":"exec","executed":false,"content":{"status":"require_approval","actionClass":"shell","reason":"second"},"error":"second"}"#,
            ),
        ];
        let p = project_timeline(&events, &[run(RunStatus::ApprovalWaiting)], true, false, None);
        let pending: Vec<_> = p.items.iter().filter(|i| i.needs_approval).collect();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].seq, 2);
        assert!(!p.items[0].needs_approval);
    }
}
