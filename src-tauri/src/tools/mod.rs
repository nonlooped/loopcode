//! Built-in tools and context compiler.

pub mod catalog;
pub mod context;
pub mod edit;
pub mod hash;
pub mod linediff;
pub mod network;
pub mod plan;
pub mod shell;
pub mod skills;
pub mod workspace;

use crate::db::store::Database;
use crate::runtime::mode::{mode_allows_tool_effect, mode_rejection_reason, Mode};
use crate::runtime::tools::{ToolOutcomeKind, ToolProposal, ToolResult};
use crate::tools::catalog::effective_catalog;
use serde_json::{json, Value};
use std::path::Path;

/// Execute a built-in (or merged MCP) tool against the real project filesystem / Core services.
pub fn execute_builtin(
    root: Option<&Path>,
    db: Option<&Database>,
    project_id: Option<&str>,
    chat_id: Option<&str>,
    run_id: Option<&str>,
    mode: Mode,
    proposal: &ToolProposal,
) -> ToolResult {
    let catalog = effective_catalog(db, project_id);
    let def = catalog.iter().find(|d| d.name == proposal.tool_name).cloned();

    let Some(def) = def else {
        return ToolResult {
            call_id: proposal.call_id.clone(),
            tool_name: proposal.tool_name.clone(),
            kind: ToolOutcomeKind::Rejected,
            executed: false,
            content: json!({"status": "unknown_tool", "tool": proposal.tool_name}),
            error_message: Some(format!("unknown tool: {}", proposal.tool_name)),
        };
    };

    if !mode_allows_tool_effect(mode, def.effect) {
        let reason = mode_rejection_reason(mode, def.effect);
        return ToolResult {
            call_id: proposal.call_id.clone(),
            tool_name: proposal.tool_name.clone(),
            kind: ToolOutcomeKind::Rejected,
            executed: false,
            content: json!({"status": "mode_policy_denied", "reason": reason}),
            error_message: Some(reason),
        };
    }

    // Plan-only tools
    if matches!(proposal.tool_name.as_str(), "plan_write" | "plan_update")
        && !matches!(mode, Mode::Plan)
    {
        return ToolResult {
            call_id: proposal.call_id.clone(),
            tool_name: proposal.tool_name.clone(),
            kind: ToolOutcomeKind::Rejected,
            executed: false,
            content: json!({"status": "mode_policy_denied", "reason": "plan tools require Plan mode"}),
            error_message: Some("plan tools require Plan mode".into()),
        };
    }

    let result = if def.is_mcp() {
        let db = match db {
            Some(d) => d,
            None => {
                return ToolResult {
                    call_id: proposal.call_id.clone(),
                    tool_name: proposal.tool_name.clone(),
                    kind: ToolOutcomeKind::Rejected,
                    executed: false,
                    content: json!({"status": "tool_error", "error": "database required"}),
                    error_message: Some("database required".into()),
                };
            }
        };
        let server_id = def.mcp_server_id.as_deref().unwrap_or("");
        let tool = def.mcp_tool_name.as_deref().unwrap_or("");
        crate::extensibility::mcp_call(db, server_id, tool, &proposal.arguments, project_id, true)
            .map(|r| serde_json::to_value(r).unwrap_or(json!({"status": "ok"})))
    } else {
        dispatch(
            root,
            db,
            project_id,
            chat_id,
            run_id,
            &proposal.tool_name,
            &proposal.arguments,
        )
    };

    match result {
        Ok(content) => {
            let status = content.get("status").and_then(|s| s.as_str()).unwrap_or("ok");
            let timed_out = content.get("timedOut").and_then(|v| v.as_bool()) == Some(true)
                || status == "timeout";
            let executed = content
                .get("executed")
                .and_then(|v| v.as_bool())
                .unwrap_or(status == "ok" && !timed_out);
            let require = status == "require_approval"
                || status == "require_server_trust"
                || status == "timeout"
                || timed_out
                || !executed;
            let error_message = if require {
                content
                    .get("reason")
                    .or_else(|| content.get("stderr"))
                    .and_then(|r| r.as_str())
                    .map(str::to_string)
                    .or_else(|| {
                        if timed_out {
                            Some("tool timed out".into())
                        } else {
                            None
                        }
                    })
            } else {
                None
            };
            ToolResult {
                call_id: proposal.call_id.clone(),
                tool_name: proposal.tool_name.clone(),
                kind: if require {
                    ToolOutcomeKind::Rejected
                } else {
                    ToolOutcomeKind::Executed
                },
                executed: executed && !require,
                content,
                error_message,
            }
        }
        Err(e) => ToolResult {
            call_id: proposal.call_id.clone(),
            tool_name: proposal.tool_name.clone(),
            kind: ToolOutcomeKind::Rejected,
            executed: false,
            content: json!({"status": "tool_error", "error": e}),
            error_message: Some(e),
        },
    }
}

fn dispatch(
    root: Option<&Path>,
    db: Option<&Database>,
    project_id: Option<&str>,
    chat_id: Option<&str>,
    run_id: Option<&str>,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    match name {
        "glob" => {
            let root = root.ok_or("project root required")?;
            workspace::workspace_glob(root, args)
        }
        "read" => {
            let root = root.ok_or("project root required")?;
            workspace::workspace_read(root, args)
        }
        "grep" => {
            let root = root.ok_or("project root required")?;
            workspace::workspace_search(root, args)
        }
        "write" => {
            let root = root.ok_or("project root required")?;
            workspace::workspace_write(root, args)
        }
        "edit" => {
            let root = root.ok_or("project root required")?;
            edit::edit_file(root, args)
        }
        "patch" => {
            let root = root.ok_or("project root required")?;
            workspace::workspace_apply_patch(root, args)
        }
        "exec" => {
            let root = root.ok_or("project root required")?;
            shell::shell_exec(root, args)
        }
        "webfetch" => network::web_fetch(args),
        "plan_write" => {
            let db = db.ok_or("database required")?;
            let project_id = project_id.ok_or("project_id required")?;
            let chat_id = chat_id.ok_or("chat_id required")?;
            plan::plan_write(db, project_id, chat_id, run_id, args)
        }
        "plan_update" => {
            let db = db.ok_or("database required")?;
            let project_id = project_id.ok_or("project_id required")?;
            let chat_id = chat_id.ok_or("chat_id required")?;
            plan::plan_update(db, project_id, chat_id, run_id, args)
        }
        "context.attach" => context::context_attach(root.ok_or("project root required")?, args),
        "skill" => {
            let root = root.ok_or("project root required")?;
            skills::skills_load(root, args)
        }
        other => Err(format!("no real implementation for {other}")),
    }
}

/// Whether executing this tool can release the AppState db/runtime mutexes.
/// Tools that need the live `Database` connection must keep the lock.
pub fn tool_exec_releases_host_locks(name: &str) -> bool {
    !matches!(name, "plan_write" | "plan_update") && !name.starts_with("mcp_")
}

/// Whether this tool name has a real implementation (builtin or MCP-prefixed).
pub fn is_builtin_implemented(name: &str) -> bool {
    matches!(
        name,
        "glob"
            | "read"
            | "grep"
            | "write"
            | "edit"
            | "patch"
            | "exec"
            | "webfetch"
            | "plan_write"
            | "plan_update"
            | "context.attach"
            | "skill"
    ) || name.starts_with("mcp_")
}
