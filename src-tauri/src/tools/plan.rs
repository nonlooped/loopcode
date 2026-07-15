//! Plan artifact tools — create/update named plan docs (no source mutation).

use crate::db::store::Database;
use serde_json::{json, Value};

/// `plan_write` — create plan artifact linked to chat/run.
pub fn plan_write(
    db: &Database,
    project_id: &str,
    chat_id: &str,
    run_id: Option<&str>,
    args: &Value,
) -> Result<Value, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Plan");
    let body = args
        .get("body")
        .or_else(|| args.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let art = db.create_artifact(
        project_id,
        Some(chat_id),
        run_id,
        "plan",
        title,
        Some(body),
    )?;
    Ok(json!({
        "status": "ok",
        "artifactId": art.id,
        "title": art.title,
        "version": art.version,
        "kind": "plan",
    }))
}

/// `plan_update` — creates a new artifact version for the updated plan body.
pub fn plan_update(
    db: &Database,
    project_id: &str,
    chat_id: &str,
    run_id: Option<&str>,
    args: &Value,
) -> Result<Value, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Plan (updated)");
    let body = args
        .get("body")
        .or_else(|| args.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let art = db.create_artifact(
        project_id,
        Some(chat_id),
        run_id,
        "plan",
        title,
        Some(body),
    )?;
    Ok(json!({
        "status": "ok",
        "artifactId": art.id,
        "title": art.title,
        "version": art.version,
        "kind": "plan",
        "updated": true,
    }))
}
