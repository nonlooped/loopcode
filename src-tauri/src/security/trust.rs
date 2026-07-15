//! MCP / skill trust stubs — dual-gate without full transport.

use crate::db::store::Database;
use crate::domain::TrustRecord;
use crate::security::audit::{self, kinds as audit_kinds};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustKind {
    McpServer,
    SkillScript,
}

impl TrustKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::McpServer => "mcp_server",
            Self::SkillScript => "skill_script",
        }
    }
}

/// Collision-resistant fingerprint of config/script bytes for trust invalidation.
pub fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(7 + digest.len() * 2);
    out.push_str("sha256:");
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

/// Grant trust for a subject at a given content hash.
pub fn grant_trust(
    db: &Database,
    kind: TrustKind,
    subject_key: &str,
    content_hash: &str,
    project_id: Option<&str>,
) -> Result<TrustRecord, String> {
    let rec = db.upsert_trust(kind.as_str(), subject_key, content_hash, project_id)?;
    let _ = audit::append_audit(
        db,
        match kind {
            TrustKind::McpServer => audit_kinds::MCP_TRUST,
            TrustKind::SkillScript => audit_kinds::SKILL_TRUST,
        },
        "user",
        None,
        None,
        project_id,
        json!({
            "action": "grant",
            "kind": kind.as_str(),
            "subjectKey": subject_key,
            "contentHash": content_hash,
        }),
    )?;
    Ok(rec)
}

/// Check whether subject is trusted at exactly `content_hash`.
/// Config/hash change → prior trust invalid (returns false).
pub fn is_trusted(
    db: &Database,
    kind: TrustKind,
    subject_key: &str,
    content_hash: &str,
    project_id: Option<&str>,
) -> Result<bool, String> {
    match db.get_trust(kind.as_str(), subject_key, project_id)? {
        Some(rec) => Ok(rec.content_hash == content_hash),
        None => Ok(false),
    }
}

/// Invalidate trust when config/hash changes.
pub fn invalidate_trust(
    db: &Database,
    kind: TrustKind,
    subject_key: &str,
    project_id: Option<&str>,
) -> Result<(), String> {
    db.delete_trust(kind.as_str(), subject_key, project_id)?;
    let _ = audit::append_audit(
        db,
        match kind {
            TrustKind::McpServer => audit_kinds::MCP_TRUST,
            TrustKind::SkillScript => audit_kinds::SKILL_TRUST,
        },
        "core",
        None,
        None,
        project_id,
        json!({
            "action": "invalidate",
            "kind": kind.as_str(),
            "subjectKey": subject_key,
            "reason": "config_or_hash_change",
        }),
    )?;
    Ok(())
}
