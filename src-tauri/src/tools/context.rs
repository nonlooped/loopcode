//! Lazy host-compiled context + durable manifest.

use crate::tools::skills::{discover_skills, SkillMeta};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Default approximate token budget for auto-included file bodies.
pub const DEFAULT_BODY_TOKEN_BUDGET: usize = 6000;
/// Rough chars-per-token estimate for budgeting.
const CHARS_PER_TOKEN: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextItemStatus {
    Included,
    Summarized,
    Truncated,
    Omitted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextItem {
    pub source: String,
    pub path: Option<String>,
    pub status: ContextItemStatus,
    pub reason: String,
    pub approx_tokens: usize,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextManifest {
    pub items: Vec<ContextItem>,
    pub body_token_budget: usize,
    pub body_tokens_used: usize,
    pub agents_md_paths: Vec<String>,
    pub skills_metadata: Vec<SkillMeta>,
}

/// Discover AGENTS.md from project root (and optional nested active path).
pub fn discover_agents_md(project_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let root_agents = project_root.join("AGENTS.md");
    if root_agents.is_file() {
        out.push(root_agents);
    }
    out
}

/// Compile context for a run: mandatory markers, AGENTS.md, optional attaches, skill metadata.
pub fn compile_context(
    project_root: &Path,
    user_turn: &str,
    attach_paths: &[String],
    body_token_budget: usize,
) -> ContextManifest {
    let mut items = Vec::new();
    let mut body_tokens_used = 0usize;

    items.push(ContextItem {
        source: "host.safety".into(),
        path: None,
        status: ContextItemStatus::Included,
        reason: "mandatory host safety/mode/tool protocol".into(),
        approx_tokens: 50,
        content: None, // real protocol lives in host_system_prompt
    });
    body_tokens_used += 50;

    let agents = discover_agents_md(project_root);
    let agents_paths: Vec<String> = agents
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();

    for path in &agents {
        let text = fs::read_to_string(path).unwrap_or_default();
        let tokens = approx_tokens(&text);
        if body_tokens_used + tokens <= body_token_budget {
            body_tokens_used += tokens;
            items.push(ContextItem {
                source: "agents.md".into(),
                path: Some(path.to_string_lossy().replace('\\', "/")),
                status: ContextItemStatus::Included,
                reason: "project instructions".into(),
                approx_tokens: tokens,
                content: Some(text),
            });
        } else if body_tokens_used + 100 <= body_token_budget {
            let summary = summarize_head(&text, 500);
            let t = approx_tokens(&summary);
            body_tokens_used += t;
            items.push(ContextItem {
                source: "agents.md".into(),
                path: Some(path.to_string_lossy().replace('\\', "/")),
                status: ContextItemStatus::Summarized,
                reason: "budget: included head summary".into(),
                approx_tokens: t,
                content: Some(summary),
            });
        } else {
            items.push(ContextItem {
                source: "agents.md".into(),
                path: Some(path.to_string_lossy().replace('\\', "/")),
                status: ContextItemStatus::Omitted,
                reason: "budget exhausted".into(),
                approx_tokens: tokens,
                content: None,
            });
        }
    }

    // Current user turn always preferred
    let turn_tokens = approx_tokens(user_turn);
    items.push(ContextItem {
        source: "user.turn".into(),
        path: None,
        status: ContextItemStatus::Included,
        reason: "current user turn".into(),
        approx_tokens: turn_tokens,
        content: Some(user_turn.to_string()),
    });
    body_tokens_used += turn_tokens;

    for rel in attach_paths {
        let path = project_root.join(rel);
        if !path.is_file() {
            items.push(ContextItem {
                source: "attach".into(),
                path: Some(rel.clone()),
                status: ContextItemStatus::Omitted,
                reason: "file not found".into(),
                approx_tokens: 0,
                content: None,
            });
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        let tokens = approx_tokens(&text);
        if body_tokens_used + tokens <= body_token_budget {
            body_tokens_used += tokens;
            items.push(ContextItem {
                source: "attach".into(),
                path: Some(rel.clone()),
                status: ContextItemStatus::Included,
                reason: "user-attached".into(),
                approx_tokens: tokens,
                content: Some(text),
            });
        } else {
            let summary = summarize_head(&text, 400);
            let t = approx_tokens(&summary);
            if body_tokens_used + t <= body_token_budget {
                body_tokens_used += t;
                items.push(ContextItem {
                    source: "attach".into(),
                    path: Some(rel.clone()),
                    status: ContextItemStatus::Summarized,
                    reason: "budget: summarized large attach".into(),
                    approx_tokens: t,
                    content: Some(format!(
                        "[summary; full file omitted for budget]\n{summary}"
                    )),
                });
            } else {
                items.push(ContextItem {
                    source: "attach".into(),
                    path: Some(rel.clone()),
                    status: ContextItemStatus::Omitted,
                    reason: "budget exhausted".into(),
                    approx_tokens: tokens,
                    content: None,
                });
            }
        }
    }

    // Skill metadata only (bodies on demand)
    let skills = discover_skills(project_root);
    for s in &skills {
        items.push(ContextItem {
            source: "skill.meta".into(),
            path: Some(s.path.clone()),
            status: ContextItemStatus::Included,
            reason: "skill metadata (body not loaded)".into(),
            approx_tokens: 20,
            content: Some(format!("{}: {}", s.name, s.description)),
        });
        body_tokens_used += 20;
    }

    // Fixed ignore-list noise never auto-included as bodies
    items.push(ContextItem {
        source: "policy.ignore".into(),
        path: None,
        status: ContextItemStatus::Omitted,
        reason: "noise paths (.git, node_modules, target, …) excluded from auto context".into(),
        approx_tokens: 0,
        content: None,
    });

    ContextManifest {
        items,
        body_token_budget,
        body_tokens_used,
        agents_md_paths: agents_paths,
        skills_metadata: skills,
    }
}

pub fn approx_tokens(text: &str) -> usize {
    text.len().div_ceil(CHARS_PER_TOKEN).max(1)
}

fn summarize_head(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        format!("{}…", &text[..max_chars])
    }
}

/// `context.attach` — load a workspace file body into the tool result (budgeted).
pub fn context_attach(root: &Path, args: &Value) -> Result<Value, String> {
    let rel = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("path required")?;
    let class = crate::security::path::classify_path(root, rel);
    match class.class {
        crate::security::path::PathClass::Rejected => return Err(class.reason),
        crate::security::path::PathClass::OutsideWorkspace => {
            return Err(format!("path outside workspace: {}", class.reason));
        }
        _ => {}
    }
    let path = class
        .normalized
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(rel));
    if !path.is_file() {
        return Err(format!("not a file: {rel}"));
    }
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    const ATTACH_MAX_BYTES: u64 = 96 * 1024;
    if meta.len() > ATTACH_MAX_BYTES {
        let text = fs::read_to_string(&path).unwrap_or_default();
        let summary = summarize_head(&text, 2_000);
        return Ok(json!({
            "status": "ok",
            "attached": rel,
            "included": true,
            "truncated": true,
            "content": summary,
            "note": "file exceeded attach budget; head summary included",
        }));
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(json!({
        "status": "ok",
        "attached": rel,
        "included": true,
        "truncated": false,
        "content": content,
        "approxTokens": approx_tokens(&content),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_omits_large_attaches() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "lc-ctx-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("AGENTS.md"), "Be careful.\n").unwrap();
        let big = "x".repeat(50_000);
        fs::write(root.join("big.txt"), &big).unwrap();
        fs::write(root.join("small.txt"), "tiny").unwrap();

        let m = compile_context(
            &root,
            "hello",
            &["big.txt".into(), "small.txt".into()],
            500, // tight budget
        );
        assert!(m.body_tokens_used <= m.body_token_budget + 200); // turn may push slightly
        let big_item = m
            .items
            .iter()
            .find(|i| i.path.as_deref() == Some("big.txt"))
            .unwrap();
        assert!(
            matches!(
                big_item.status,
                ContextItemStatus::Summarized | ContextItemStatus::Omitted
            ),
            "{:?}",
            big_item.status
        );
        assert!(!m.agents_md_paths.is_empty());
    }
}
