//! Provider-neutral file diffs with turn/run and manual attribution.

use crate::domain::Event;
use crate::surfaces::checkpoint::CheckpointManifest;
use crate::surfaces::explorer::resolve_in_workspace;
use crate::tools::hash::content_hash_bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffScope {
    /// Changes attributed to a single run/turn.
    ThisRun,
    /// Cumulative agent changes since chat start.
    SinceChatStart,
    /// Unsaved editor buffer vs on-disk.
    BufferVsDisk,
    /// Live disk vs checkpoint post-image.
    VsCheckpoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attribution {
    /// Agent change linked to run (and optional tool call).
    Agent { run_id: String, tool_id: Option<String> },
    Manual,
    External,
}

impl Attribution {
    pub fn as_label(&self) -> String {
        match self {
            Self::Agent { run_id, tool_id } => match tool_id {
                Some(t) => format!("agent:{run_id}:{t}"),
                None => format!("agent:{run_id}"),
            },
            Self::Manual => "manual".into(),
            Self::External => "external".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffChange {
    pub path: String,
    pub kind: String, // add | modify | delete
    pub attribution: String,
    pub run_id: Option<String>,
    pub tool_id: Option<String>,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
    pub before_preview: Option<String>,
    pub after_preview: Option<String>,
    /// Line-level stats when computable (None for legacy events without them).
    pub lines_added: Option<u64>,
    pub lines_removed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffModel {
    pub scope: DiffScope,
    pub changes: Vec<DiffChange>,
}

#[derive(Debug, Clone)]
pub struct BufferSnapshot {
    pub path: String,
    pub content: String,
    pub dirty: bool,
}

/// Project diffs for the requested scope using events + filesystem (not git).
pub fn project_diffs(
    scope: DiffScope,
    events: &[Event],
    workspace_root: Option<&Path>,
    run_id: Option<&str>,
    buffers: &[BufferSnapshot],
    checkpoint: Option<&CheckpointManifest>,
) -> DiffModel {
    match scope {
        DiffScope::ThisRun => {
            let rid = run_id.unwrap_or("");
            let filtered: Vec<_> = events
                .iter()
                .filter(|e| e.run_id.as_deref() == Some(rid))
                .cloned()
                .collect();
            DiffModel {
                scope,
                changes: changes_from_agent_events(&filtered),
            }
        }
        DiffScope::SinceChatStart => DiffModel {
            scope,
            changes: changes_from_agent_events(events),
        },
        DiffScope::BufferVsDisk => {
            let root = match workspace_root {
                Some(r) => r,
                None => {
                    return DiffModel {
                        scope,
                        changes: vec![],
                    }
                }
            };
            let mut changes = Vec::new();
            for b in buffers.iter().filter(|b| b.dirty) {
                let disk = resolve_in_workspace(root, &b.path)
                    .ok()
                    .and_then(|p| fs::read_to_string(p).ok());
                let before_hash = disk.as_ref().map(|s| content_hash_bytes(s.as_bytes()));
                let after_hash = content_hash_bytes(b.content.as_bytes());
                if before_hash.as_deref() == Some(after_hash.as_str()) {
                    continue;
                }
                let (la, lr) = match &disk {
                    Some(d) => crate::tools::linediff::line_counts(d, &b.content),
                    None => (crate::tools::linediff::lines_of(&b.content), 0),
                };
                changes.push(DiffChange {
                    path: b.path.clone(),
                    kind: if disk.is_none() {
                        "add".into()
                    } else {
                        "modify".into()
                    },
                    attribution: Attribution::Manual.as_label(),
                    run_id: None,
                    tool_id: None,
                    before_hash,
                    after_hash: Some(after_hash),
                    before_preview: disk.as_ref().map(|s| truncate(s, 200)),
                    after_preview: Some(truncate(&b.content, 200)),
                    lines_added: Some(la),
                    lines_removed: Some(lr),
                });
            }
            DiffModel { scope, changes }
        }
        DiffScope::VsCheckpoint => {
            let (root, cp) = match (workspace_root, checkpoint) {
                (Some(r), Some(c)) => (r, c),
                _ => {
                    return DiffModel {
                        scope,
                        changes: vec![],
                    }
                }
            };
            let mut changes = Vec::new();
            for f in cp.files.iter().filter(|f| !f.skipped) {
                let live_path = resolve_in_workspace(root, &f.path).ok();
                let live_bytes = live_path.as_ref().and_then(|p| fs::read(p).ok());
                let live_hash = live_bytes.as_ref().map(|b| content_hash_bytes(b));
                if live_hash.as_deref() == Some(f.hash.as_str()) {
                    continue;
                }
                let kind = if live_bytes.is_none() {
                    "delete"
                } else {
                    "modify"
                };
                changes.push(DiffChange {
                    path: f.path.clone(),
                    kind: kind.into(),
                    attribution: Attribution::Manual.as_label(),
                    run_id: None,
                    tool_id: None,
                    before_hash: Some(f.hash.clone()),
                    after_hash: live_hash,
                    before_preview: None,
                    after_preview: live_bytes
                        .and_then(|b| String::from_utf8(b).ok())
                        .map(|s| truncate(&s, 200)),
                    // Checkpoint stores hashes only — no pre-image body for line stats.
                    lines_added: None,
                    lines_removed: None,
                });
            }
            DiffModel { scope, changes }
        }
    }
}

fn is_mutating_tool(tool: &str) -> bool {
    // Builtins + MCP/aliases that mutate workspace files.
    // `plan_write` only creates DB artifacts — never a workspace file change.
    if tool == "plan_write" {
        return false;
    }
    matches!(tool, "write" | "edit" | "patch")
        || tool.contains("write")
        || tool.contains("patch")
        || tool.contains("apply_patch")
}

fn changes_from_agent_events(events: &[Event]) -> Vec<DiffChange> {
    // Last write/patch per path wins; attribution from the event's run/tool.
    let mut by_path: HashMap<String, DiffChange> = HashMap::new();
    for e in events {
        // Proposals are not applied changes — only successful tool results.
        if !e.kind.starts_with("tool.result") {
            continue;
        }
        let Ok(payload) = serde_json::from_str::<Value>(&e.payload_json) else {
            continue;
        };
        let tool = payload
            .get("toolName")
            .or_else(|| payload.get("tool_name"))
            .and_then(|t| t.as_str())
            .unwrap_or("");
        // Read-only tools (read/glob/grep/…) often return path + status:ok — never
        // treat those as diffs. Only mutating tools contribute file changes.
        if !is_mutating_tool(tool) {
            continue;
        }

        let path = extract_path(&payload);
        let Some(path) = path else { continue };

        // Only record successful mutations
        let executed = payload
            .get("executed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let status_ok = payload
            .pointer("/content/status")
            .and_then(|s| s.as_str())
            == Some("ok");
        if !executed && !status_ok {
            continue;
        }

        let tool_id = payload
            .get("callId")
            .or_else(|| payload.get("call_id"))
            .and_then(|c| c.as_str())
            .map(str::to_string);
        let run_id = e.run_id.clone().unwrap_or_default();
        let after_hash = payload
            .pointer("/content/hash")
            .and_then(|h| h.as_str())
            .map(str::to_string);
        let attr = Attribution::Agent {
            run_id: run_id.clone(),
            tool_id: tool_id.clone(),
        };
        let ev_added = payload.pointer("/content/linesAdded").and_then(|v| v.as_u64());
        let ev_removed = payload
            .pointer("/content/linesRemoved")
            .and_then(|v| v.as_u64());
        let created = payload
            .pointer("/content/created")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let deleted = payload
            .pointer("/content/deleted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        // Accumulate line stats across successive edits to the same path.
        let prior = by_path.get(&path);
        let lines_added = match (prior.and_then(|p| p.lines_added), ev_added) {
            (Some(a), Some(b)) => Some(a + b),
            (None, b) => b,
            (a, None) => a,
        };
        let lines_removed = match (prior.and_then(|p| p.lines_removed), ev_removed) {
            (Some(a), Some(b)) => Some(a + b),
            (None, b) => b,
            (a, None) => a,
        };
        let kind = if deleted {
            // Created earlier in this scope and now deleted → net no-op, but still
            // surface it as a delete so the user sees the removal.
            "delete"
        } else if created || prior.is_some_and(|p| p.kind == "add") {
            "add"
        } else {
            "modify"
        };
        by_path.insert(
            path.clone(),
            DiffChange {
                path,
                kind: kind.into(),
                attribution: attr.as_label(),
                run_id: if run_id.is_empty() { None } else { Some(run_id) },
                tool_id,
                before_hash: None,
                after_hash,
                before_preview: payload
                    .pointer("/content/beforePreview")
                    .and_then(|c| c.as_str())
                    .map(|s| truncate(s, 200)),
                after_preview: payload
                    .pointer("/content/content")
                    .or_else(|| payload.get("content").and_then(|c| c.get("body")))
                    .and_then(|c| c.as_str())
                    .map(|s| truncate(s, 200)),
                lines_added,
                lines_removed,
            },
        );
    }
    let mut changes: Vec<_> = by_path.into_values().collect();
    changes.sort_by(|a, b| a.path.cmp(&b.path));
    changes
}

fn extract_path(payload: &Value) -> Option<String> {
    payload
        .get("path")
        .or_else(|| payload.pointer("/content/path"))
        .or_else(|| payload.pointer("/arguments/path"))
        .and_then(|p| p.as_str())
        .map(str::to_string)
}

/// Truncate by max **bytes**, never splitting a UTF-8 codepoint (char boundary safe).
fn truncate(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn ev(seq: i64, kind: &str, run: &str, payload: &str) -> Event {
        Event {
            id: format!("e{seq}"),
            chat_id: "c".into(),
            run_id: Some(run.into()),
            seq,
            kind: kind.into(),
            payload_json: payload.into(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn turn_and_cumulative_agent_attribution() {
        let events = vec![
            ev(
                1,
                "tool.result",
                "run-a",
                r#"{"toolName":"write","executed":true,"callId":"t1","content":{"status":"ok","path":"src/a.rs","hash":"h1","content":"agent"}}"#,
            ),
            ev(
                2,
                "tool.result",
                "run-b",
                r#"{"toolName":"write","executed":true,"callId":"t2","content":{"status":"ok","path":"src/b.rs","hash":"h2"}}"#,
            ),
        ];
        let turn = project_diffs(DiffScope::ThisRun, &events, None, Some("run-a"), &[], None);
        assert_eq!(turn.changes.len(), 1);
        assert!(turn.changes[0].attribution.contains("run-a"));
        assert!(turn.changes[0].attribution.starts_with("agent:"));

        let cum = project_diffs(DiffScope::SinceChatStart, &events, None, None, &[], None);
        assert_eq!(cum.changes.len(), 2);
    }

    #[test]
    fn read_only_tool_results_are_not_file_changes() {
        // Successful read/glob payloads carry path + status:ok — must not inflate the pill.
        let events = vec![
            ev(
                1,
                "tool.result",
                "run-a",
                r#"{"toolName":"glob","executed":true,"callId":"g1","content":{"status":"ok","path":".","pattern":"*","matchCount":3,"matches":["README.md"]}}"#,
            ),
            ev(
                2,
                "tool.result",
                "run-a",
                r##"{"toolName":"read","executed":true,"callId":"r1","content":{"status":"ok","path":"README.md","hash":"h","content":"# hi"}}"##,
            ),
            ev(
                3,
                "tool.result",
                "run-a",
                r#"{"toolName":"read","executed":true,"callId":"r2","content":{"status":"ok","path":"package.json","hash":"h2","content":"{}"}}"#,
            ),
            ev(
                4,
                "tool.result",
                "run-a",
                r#"{"toolName":"grep","executed":true,"callId":"s1","content":{"status":"ok","path":"src","matches":[]}}"#,
            ),
        ];
        let model = project_diffs(DiffScope::SinceChatStart, &events, None, None, &[], None);
        assert!(
            model.changes.is_empty(),
            "read-only tools must not produce diffs: {:?}",
            model.changes
        );
    }

    #[test]
    fn read_mixed_with_write_only_counts_mutation() {
        let events = vec![
            ev(
                1,
                "tool.result",
                "run-a",
                r##"{"toolName":"read","executed":true,"callId":"r1","content":{"status":"ok","path":"README.md","hash":"h","content":"# hi"}}"##,
            ),
            ev(
                2,
                "tool.result",
                "run-a",
                r#"{"toolName":"write","executed":true,"callId":"w1","content":{"status":"ok","path":"src/a.rs","hash":"h1","created":true,"linesAdded":1,"linesRemoved":0}}"#,
            ),
        ];
        let model = project_diffs(DiffScope::SinceChatStart, &events, None, None, &[], None);
        assert_eq!(model.changes.len(), 1);
        assert_eq!(model.changes[0].path, "src/a.rs");
        assert_eq!(model.changes[0].kind, "add");
    }

    #[test]
    fn patch_delete_result_projects_delete_kind_with_line_stats() {
        let events = vec![ev(
            1,
            "tool.result",
            "run-a",
            r#"{"toolName":"patch","executed":true,"callId":"t1","content":{"status":"ok","path":"src/old.rs","deleted":true,"beforePreview":"fn gone() {}","linesAdded":0,"linesRemoved":12}}"#,
        )];
        let model = project_diffs(DiffScope::SinceChatStart, &events, None, None, &[], None);
        assert_eq!(model.changes.len(), 1);
        let c = &model.changes[0];
        assert_eq!(c.kind, "delete");
        assert_eq!(c.lines_added, Some(0));
        assert_eq!(c.lines_removed, Some(12));
        assert_eq!(c.before_preview.as_deref(), Some("fn gone() {}"));
    }

    #[test]
    fn truncate_does_not_panic_on_multibyte_utf8() {
        // 199 ASCII + one multi-byte char → byte len > 200 mid-character if sliced naively
        let s = format!("{}{}", "a".repeat(199), "世");
        assert!(s.len() > 200);
        let t = truncate(&s, 200);
        assert!(t.ends_with('…'));
        assert!(t.is_char_boundary(t.len() - '…'.len_utf8()));
        // Must not panic and must be valid UTF-8 (already a String)
        assert!(t.chars().count() <= 201);
    }

    #[test]
    fn buffer_vs_disk_manual_with_multibyte_preview() {
        use std::fs;
        let root = std::env::temp_dir().join(format!(
            "lc-diff-utf8-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("note.txt"), "on-disk").unwrap();
        let dirty = format!("{}{}", "a".repeat(199), "世");
        let buf = BufferSnapshot {
            path: "note.txt".into(),
            content: dirty.clone(),
            dirty: true,
        };
        let model = project_diffs(
            DiffScope::BufferVsDisk,
            &[],
            Some(&root),
            None,
            &[buf],
            None,
        );
        assert_eq!(model.changes.len(), 1);
        assert_eq!(model.changes[0].attribution, "manual");
        assert_eq!(model.changes[0].path, "note.txt");
        // Preview path exercised truncate on multi-byte content without panic
        assert!(model.changes[0].after_preview.is_some());
    }
}
