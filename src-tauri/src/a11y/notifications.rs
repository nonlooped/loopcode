//! OS notification payload for approvals when unfocused — no secrets in body.

use crate::security::{redact_json_value, redact_text};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalNotificationPayload {
    pub title: String,
    pub body: String,
    pub action_class: String,
    pub chat_id: Option<String>,
    pub run_id: Option<String>,
    /// Always false for approval toasts (never include tool args/secrets).
    pub includes_secrets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDto {
    pub title: String,
    pub body: String,
    pub kind: String,
}

/// Build an approval-when-unfocused notification from a real approval-shaped DTO.
/// Body never includes API keys, tokens, raw commands with secrets, or file contents.
pub fn build_approval_notification(
    action_class: &str,
    tool_name: Option<&str>,
    chat_id: Option<&str>,
    run_id: Option<&str>,
    // Raw optional detail — will be redacted; secret-shaped content stripped.
    detail: Option<&Value>,
) -> ApprovalNotificationPayload {
    let safe_tool = tool_name.unwrap_or("tool");
    let title = "LoopCode — approval needed".to_string();
    let mut body = format!("Action class “{action_class}” for {safe_tool} needs your approval.");
    if let Some(d) = detail {
        let redacted = redact_json_value(d);
        // Only allow non-sensitive summary keys
        if let Some(summary) = redacted.get("summary").and_then(|s| s.as_str()) {
            let s = redact_text(summary);
            if !s.is_empty() {
                body.push(' ');
                body.push_str(&s);
            }
        }
        // Explicitly never append command/args/token fields even if present after redact
        let _ = redacted.get("command");
        let _ = redacted.get("arguments");
        let _ = redacted.get("apiKey");
    }
    // Final pass
    let body = redact_text(&body);
    // Strip common secret patterns leftover
    let body = strip_secret_phrases(&body);

    ApprovalNotificationPayload {
        title,
        body,
        action_class: action_class.into(),
        chat_id: chat_id.map(str::to_string),
        run_id: run_id.map(str::to_string),
        includes_secrets: false,
    }
}

fn strip_secret_phrases(s: &str) -> String {
    let mut out = s.to_string();
    for needle in ["sk-", "Bearer ", "api_key=", "apiKey=", "token="] {
        if let Some(idx) = out.find(needle) {
            // Truncate at secret-looking material
            out.truncate(idx);
            out.push_str("[redacted]");
        }
    }
    out
}

/// Convert to a generic notification DTO for UI/OS bridge.
pub fn to_notification_dto(p: &ApprovalNotificationPayload) -> NotificationDto {
    NotificationDto {
        title: p.title.clone(),
        body: p.body.clone(),
        kind: "approval".into(),
    }
}

/// Preview helper used by tests — ensures payload JSON has no secret keys.
pub fn notification_payload_is_safe(payload: &ApprovalNotificationPayload) -> bool {
    if payload.includes_secrets {
        return false;
    }
    let s = format!("{} {}", payload.title, payload.body);
    let lower = s.to_ascii_lowercase();
    !lower.contains("sk-")
        && !lower.contains("bearer ")
        && !lower.contains("api_key")
        && !s.contains("BEGIN PRIVATE")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn approval_notification_redacts_secrets() {
        let raw = json!({
            "command": "curl -H 'Authorization: Bearer SECRETTOKEN' https://x",
            "apiKey": "sk-live-secret-value",
            "summary": "Shell needs approval",
            "arguments": {"token": "abc"}
        });
        let n = build_approval_notification(
            "shell",
            Some("exec"),
            Some("chat1"),
            Some("run1"),
            Some(&raw),
        );
        assert!(!n.includes_secrets);
        assert!(notification_payload_is_safe(&n));
        assert!(!n.body.contains("sk-live"));
        assert!(!n.body.contains("SECRETTOKEN"));
        assert!(!n.body.contains("Bearer SECRET"));
        assert!(n.body.contains("shell") || n.title.contains("approval"));
    }
}
