//! Export durable chat data with the shared Core redactor.

use crate::db::Database;
use crate::domain::{ExportBundle, ExportEvent};
pub use crate::security::{redact_json_value, redact_payload_json, redact_text};

/// Build a redacted export bundle for a chat. Secret-store values are never in
/// SQLite and therefore cannot be included here.
pub fn export_chat_redacted(db: &Database, chat_id: &str) -> Result<ExportBundle, String> {
    let chat = db
        .get_chat(chat_id)?
        .ok_or_else(|| format!("chat not found: {chat_id}"))?;
    let events = db.list_events(chat_id, None)?;

    let mut warnings = vec!["Secret-shaped fields and values are redacted from the export".into()];
    warnings.push("OS secret store material is never present in SQLite and is not exported".into());
    let secret_count = db.count_secret_refs_for_project(&chat.project_id)?;
    if secret_count > 0 {
        warnings.push(format!(
            "{secret_count} secret_ref(s) exist for project; only store_key names exist in DB (no values exported)"
        ));
    }

    let transcript = events
        .into_iter()
        .map(|e| ExportEvent {
            seq: e.seq,
            kind: e.kind,
            payload_json: redact_payload_json(&e.payload_json),
            created_at: e.created_at,
        })
        .collect();

    Ok(ExportBundle {
        format: "loopcode.chat.export.v1".into(),
        chat_id: chat.id,
        project_id: chat.project_id,
        redacted: true,
        transcript,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_api_key_field() {
        let v = json!({"message": "hi", "api_key": "sk-secretvalue12345"});
        let r = redact_json_value(&v);
        assert_eq!(r["api_key"], "[REDACTED]");
        assert_eq!(r["message"], "hi");
    }

    #[test]
    fn redacts_sk_and_bearer() {
        let t = redact_text("Bearer abc.def.ghi-token and sk-abcdefghijklmnopqrst");
        assert!(t.contains("[REDACTED]"), "got: {t}");
        assert!(!t.contains("abc.def.ghi"));
        assert!(!t.contains("sk-abcdefghij"));
    }
}
