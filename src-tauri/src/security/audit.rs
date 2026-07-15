//! Append-only audit log (approvals, secret-ref access, MCP/skill trust).

use crate::security::redact_json_value;
use crate::db::store::Database;
use crate::domain::AuditRecord;
use serde_json::{json, Value};

pub mod kinds {
    pub const APPROVAL: &str = "audit.approval";
    pub const SECRET_REF_ACCESS: &str = "audit.secret_ref_access";
    pub const MCP_TRUST: &str = "audit.mcp_trust";
    pub const SKILL_TRUST: &str = "audit.skill_trust";
    pub const EXPORT: &str = "audit.export";
}

/// Append a redacted audit row. Payload must not contain secret plaintext.
pub fn append_audit(
    db: &Database,
    kind: &str,
    actor: &str,
    run_id: Option<&str>,
    chat_id: Option<&str>,
    project_id: Option<&str>,
    payload: Value,
) -> Result<AuditRecord, String> {
    let redacted = redact_json_value(&payload);
    let payload_json = redacted.to_string();
    db.insert_audit(kind, actor, run_id, chat_id, project_id, &payload_json)
}

pub struct ApprovalAudit<'a> {
    pub chat_id: &'a str,
    pub run_id: &'a str,
    pub project_id: Option<&'a str>,
    pub action_class: &'a str,
    pub decision: &'a str,
    pub grant: Option<&'a str>,
    pub tool_name: &'a str,
}

pub fn audit_approval(db: &Database, a: ApprovalAudit<'_>) -> Result<AuditRecord, String> {
    append_audit(
        db,
        kinds::APPROVAL,
        "core",
        Some(a.run_id),
        Some(a.chat_id),
        a.project_id,
        json!({
            "actionClass": a.action_class,
            "decision": a.decision,
            "grant": a.grant,
            "toolName": a.tool_name,
            "policyVersion": crate::security::approval::SECURITY_POLICY_VERSION,
        }),
    )
}

pub fn audit_secret_access(
    db: &Database,
    store_key: &str,
    chat_id: Option<&str>,
    run_id: Option<&str>,
    project_id: Option<&str>,
    operation: &str,
) -> Result<AuditRecord, String> {
    append_audit(
        db,
        kinds::SECRET_REF_ACCESS,
        "core",
        run_id,
        chat_id,
        project_id,
        json!({
            "storeKey": store_key,
            "operation": operation,
        }),
    )
}
