//! Skill discovery — metadata free; body load-only on demand.
//! Script invoke is approval-gated (skill_script) with content-hash trust invalidation.

use crate::db::store::Database;
use crate::security::approval::{approval_decision, ActionClass, ApprovalDecision};
use crate::security::trust::{self, content_hash, TrustKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub path: String,
    pub origin: String,
    /// Script file names under the skill dir (paths only — not executed on list).
    #[serde(default)]
    pub scripts: Vec<String>,
}

/// Discover SKILL.md under known project skill roots (metadata only — never bodies).
///
/// Search order (later duplicates of the same absolute path are skipped):
/// - `skills/`
/// - `.loopcode/skills/`
/// - `.agents/skills/` (Agent Skills / Claude Code convention)
pub fn discover_skills(project_root: &Path) -> Vec<SkillMeta> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (subdir, origin) in [
        ("skills", "project"),
        (".loopcode/skills", "loopcode"),
        (".agents/skills", "agents"),
    ] {
        let base = project_root.join(subdir);
        if !base.is_dir() {
            continue;
        }
        if let Ok(rd) = fs::read_dir(&base) {
            for ent in rd.flatten() {
                let p = ent.path();
                if p.is_dir() {
                    let skill_md = p.join("SKILL.md");
                    if skill_md.is_file() {
                        let key = skill_md.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
                        if !seen.insert(key) {
                            continue;
                        }
                        if let Some(meta) = parse_skill_meta(&skill_md, origin) {
                            out.push(meta);
                        }
                    }
                }
            }
        }
    }
    out.sort_by_key(|a| a.name.to_ascii_lowercase());
    out
}

fn parse_skill_meta(path: &Path, origin: &str) -> Option<SkillMeta> {
    let text = fs::read_to_string(path).ok()?;
    let (name, description) = parse_frontmatter(&text);
    let name = name.unwrap_or_else(|| {
        path.parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "skill".into())
    });
    let scripts = list_scripts_in_skill_dir(path.parent().unwrap_or(path));
    Some(SkillMeta {
        name,
        description: description.unwrap_or_default(),
        path: path.to_string_lossy().replace('\\', "/"),
        origin: origin.into(),
        scripts,
    })
}

fn list_scripts_in_skill_dir(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return out;
    };
    for ent in rd.flatten() {
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.eq_ignore_ascii_case("SKILL.md") {
            continue;
        }
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(
            ext.as_str(),
            "sh" | "bash" | "ps1" | "py" | "js" | "mjs" | "ts" | "rb" | "exe" | "cmd" | "bat"
        ) || name.starts_with("run")
        {
            out.push(name.to_string());
        }
    }
    out.sort();
    out
}

/// Subject key for skill-script trust rows.
pub fn skill_script_subject(skill_dir_rel: &str, script_name: &str) -> String {
    format!(
        "{}::{}",
        skill_dir_rel.replace('\\', "/"),
        script_name.replace('\\', "/")
    )
}

/// Strip Windows verbatim (`\\?\`) prefixes so child processes (node) get normal paths.
pub fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else if let Some(rest) = s.strip_prefix("//?/") {
        PathBuf::from(rest)
    } else {
        path.to_path_buf()
    }
}

fn path_under(child: &Path, parent: &Path) -> bool {
    let c = strip_verbatim_prefix(child);
    let p = strip_verbatim_prefix(parent);
    c.starts_with(&p)
}

/// Resolve skill script to (host_path, subject_key, content_hash, display_command).
/// `host_path` is free of `\\?\` so spawners work on Windows.
pub fn resolve_skill_script(
    root: &Path,
    skill_md_path: &str,
    script_name: &str,
) -> Result<(PathBuf, String, String, String), String> {
    let skill_md = PathBuf::from(skill_md_path);
    let full_md = if skill_md.is_absolute() {
        skill_md
    } else {
        root.join(&skill_md)
    };
    let root_c = strip_verbatim_prefix(
        &root
            .canonicalize()
            .unwrap_or_else(|_| root.to_path_buf()),
    );
    let full_md_c = strip_verbatim_prefix(
        &full_md
            .canonicalize()
            .map_err(|e| format!("skill path: {e}"))?,
    );
    if !path_under(&full_md_c, &root_c) {
        return Err("skill path outside project".into());
    }
    let skill_dir = full_md_c
        .parent()
        .ok_or("skill dir missing")?
        .to_path_buf();
    // Prefer joining non-verbatim paths for process spawn
    let script_joined = skill_dir.join(script_name);
    let script_c = if script_joined.is_file() {
        strip_verbatim_prefix(
            &script_joined
                .canonicalize()
                .unwrap_or_else(|_| script_joined.clone()),
        )
    } else {
        return Err(format!("script not found: {script_name}"));
    };
    if !path_under(&script_c, &skill_dir) || !path_under(&script_c, &root_c) {
        return Err("script path outside skill directory".into());
    }
    if !script_c.is_file() {
        return Err(format!("script not found: {script_name}"));
    }
    let rel_dir = skill_dir
        .strip_prefix(&root_c)
        .unwrap_or(&skill_dir)
        .to_string_lossy()
        .replace('\\', "/");
    let bytes = fs::read(&script_c).map_err(|e| e.to_string())?;
    let hash = content_hash(&bytes);
    let command = script_c.display().to_string();
    Ok((
        script_c,
        skill_script_subject(&rel_dir, script_name),
        hash,
        command,
    ))
}

/// Preview exact command + hash for approval cards (no execution).
pub fn preview_skill_script(
    root: &Path,
    skill_md_path: &str,
    script_name: &str,
) -> Result<Value, String> {
    let (path, subject, hash, command) = resolve_skill_script(root, skill_md_path, script_name)?;
    Ok(json!({
        "command": command,
        "scriptPath": path.display().to_string(),
        "subjectKey": subject,
        "contentHash": hash,
        "actionClass": ActionClass::SkillScript.as_str(),
    }))
}

fn parse_frontmatter(text: &str) -> (Option<String>, Option<String>) {
    if !text.starts_with("---") {
        return (None, None);
    }
    let rest = text.trim_start_matches("---");
    let Some(end) = rest.find("---") else {
        return (None, None);
    };
    let fm = &rest[..end];
    let mut name = None;
    let mut description = None;
    for line in fm.lines() {
        if let Some(v) = line.strip_prefix("name:") {
            name = Some(v.trim().trim_matches('"').to_string());
        }
        if let Some(v) = line.strip_prefix("description:") {
            description = Some(v.trim().trim_matches('"').to_string());
        }
    }
    (name, description)
}

pub fn skills_list(root: &Path, _args: &Value) -> Result<Value, String> {
    let skills = discover_skills(root);
    Ok(json!({
        "status": "ok",
        "skills": skills,
        "loadOnly": true,
    }))
}

/// Load full SKILL.md body on demand (still not executing scripts).
pub fn skills_load(root: &Path, args: &Value) -> Result<Value, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("path required")?;
    let p = PathBuf::from(path);
    let full = if p.is_absolute() {
        p
    } else {
        root.join(p)
    };
    // Must stay under root
    let root_c = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let full_c = full.canonicalize().map_err(|e| e.to_string())?;
    if !full_c.starts_with(&root_c) {
        return Err("skill path outside project".into());
    }
    if !full_c.ends_with("SKILL.md") {
        return Err("path must be a SKILL.md file".into());
    }
    let body = fs::read_to_string(&full_c).map_err(|e| e.to_string())?;
    let (name, description) = parse_frontmatter(&body);
    Ok(json!({
        "status": "ok",
        "path": full_c.to_string_lossy(),
        "name": name,
        "description": description,
        "body": body,
        "scriptsNotExecuted": true,
    }))
}

/// Grant trust for a skill script at its current content hash.
pub fn grant_skill_script_trust(
    db: &Database,
    root: &Path,
    skill_md_path: &str,
    script_name: &str,
    project_id: Option<&str>,
) -> Result<Value, String> {
    let (_script, subject, hash, command) = resolve_skill_script(root, skill_md_path, script_name)?;
    let rec = trust::grant_trust(db, TrustKind::SkillScript, &subject, &hash, project_id)?;
    Ok(json!({
        "status": "ok",
        "subjectKey": subject,
        "contentHash": hash,
        "command": command,
        "trustId": rec.id,
    }))
}

/// Grant trust when the host already has subject+hash from a pending approval card.
pub fn grant_skill_script_trust_at_hash(
    db: &Database,
    subject_key: &str,
    content_hash: &str,
    project_id: Option<&str>,
) -> Result<(), String> {
    trust::grant_trust(
        db,
        TrustKind::SkillScript,
        subject_key,
        content_hash,
        project_id,
    )?;
    Ok(())
}

/// Evaluate whether a skill script may run: approval class + hash trust.
///
/// Always `require_approval` unless `invocation_granted` (once/this-chat grant)
/// **and** the script is trusted at the current content hash. Hash change
/// invalidates prior trust → re-prompt.
///
/// After matrix approval, the runtime calls [`grant_skill_script_trust_at_hash`]
/// so re-propose can execute.
pub fn evaluate_skill_script_run(
    db: &Database,
    root: &Path,
    skill_md_path: &str,
    script_name: &str,
    project_id: Option<&str>,
    invocation_granted: bool,
) -> Result<Value, String> {
    let (script_path, subject, hash, command) =
        resolve_skill_script(root, skill_md_path, script_name)?;
    let trusted = trust::is_trusted(db, TrustKind::SkillScript, &subject, &hash, project_id)?;
    let matrix = approval_decision(ActionClass::SkillScript);

    if !trusted {
        return Ok(json!({
            "status": "require_approval",
            "actionClass": ActionClass::SkillScript.as_str(),
            "reason": "skill script not trusted at current content hash (or hash changed)",
            "subjectKey": subject,
            "contentHash": hash,
            "command": command,
            "matrix": matrix.as_str(),
            "executed": false,
        }));
    }

    if !invocation_granted && matrix == ApprovalDecision::RequireApproval {
        return Ok(json!({
            "status": "require_approval",
            "actionClass": ActionClass::SkillScript.as_str(),
            "reason": "skill script invoke requires approval (same class as shell)",
            "subjectKey": subject,
            "contentHash": hash,
            "command": command,
            "trusted": true,
            "executed": false,
        }));
    }

    // Trusted + granted: run (bounded). Never auto-runs on folder open.
    match run_script_process(&script_path) {
        Ok(output) => Ok(json!({
            "status": "ok",
            "actionClass": ActionClass::SkillScript.as_str(),
            "subjectKey": subject,
            "contentHash": hash,
            "command": command,
            "executed": true,
            "output": output,
        })),
        Err(e) => Ok(json!({
            "status": "tool_error",
            "actionClass": ActionClass::SkillScript.as_str(),
            "subjectKey": subject,
            "contentHash": hash,
            "command": command,
            "executed": false,
            "error": e,
            "reason": e,
        })),
    }
}

fn run_script_process(script: &Path) -> Result<String, String> {
    // Always use non-verbatim path for child process argv.
    let script = strip_verbatim_prefix(script);
    let ext = script
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut cmd = match ext.as_str() {
        "ps1" => {
            let mut c = Command::new("powershell");
            c.arg("-NoProfile").arg("-File").arg(&script);
            c
        }
        "py" => {
            let mut c = Command::new("python");
            c.arg(&script);
            c
        }
        "js" | "mjs" => {
            let mut c = Command::new("node");
            c.arg(&script);
            c
        }
        "sh" | "bash" => {
            let mut c = Command::new("bash");
            c.arg(&script);
            c
        }
        _ => Command::new(&script),
    };
    let out = cmd.output().map_err(|e| e.to_string())?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.stderr.is_empty() {
        s.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    if s.len() > 16_384 {
        s.truncate(16_384);
        s.push_str("…[truncated]");
    }
    if !out.status.success() {
        return Err(format!(
            "script exited with code {:?}: {s}",
            out.status.code()
        ));
    }
    Ok(s)
}

/// Invalidate skill-script trust (e.g. after detected hash change).
pub fn invalidate_skill_script_trust(
    db: &Database,
    subject_key: &str,
    project_id: Option<&str>,
) -> Result<(), String> {
    trust::invalidate_trust(db, TrustKind::SkillScript, subject_key, project_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::store::Database;

    fn temp_skill_project() -> PathBuf {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "lc-skill-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let skill_dir = root.join("skills/demo");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: demo\ndescription: A demo skill\n---\n# Body secret details\n",
        )
        .unwrap();
        fs::write(skill_dir.join("run.js"), "console.log('skill-v1');\n").unwrap();
        root
    }

    #[test]
    fn discovers_skill_metadata_not_body() {
        let root = temp_skill_project();
        let list = discover_skills(&root);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "demo");
        assert_eq!(list[0].description, "A demo skill");
        assert!(list[0].scripts.iter().any(|s| s == "run.js"));
        let v = skills_list(&root, &json!({})).unwrap();
        let s = v.to_string();
        assert!(!s.contains("Body secret details"));
        assert!(!s.contains("skill-v1"));
    }

    #[test]
    fn discovers_agents_skills_convention() {
        let root = std::env::temp_dir().join(format!(
            "loopcode-agents-skills-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let skill_dir = root.join(".agents/skills/polish");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: polish\ndescription: UI polish skill\n---\n# Body stays hidden\n",
        )
        .unwrap();
        let list = discover_skills(&root);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "polish");
        assert_eq!(list[0].origin, "agents");
        assert!(!skills_list(&root, &json!({})).unwrap().to_string().contains("Body stays hidden"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn script_requires_approval_and_hash_invalidates_trust() {
        let db = Database::open_in_memory().unwrap();
        let root = temp_skill_project();
        let skill_md = root.join("skills/demo/SKILL.md");
        let skill_md_s = skill_md.to_string_lossy().replace('\\', "/");

        // No trust → require_approval
        let r = evaluate_skill_script_run(
            &db,
            &root,
            &skill_md_s,
            "run.js",
            Some("p1"),
            false,
        )
        .unwrap();
        assert_eq!(r["status"], "require_approval");
        assert_eq!(r["executed"], false);
        assert_eq!(r["actionClass"], "skill_script");
        let hash1 = r["contentHash"].as_str().unwrap().to_string();
        let subject = r["subjectKey"].as_str().unwrap().to_string();

        grant_skill_script_trust(&db, &root, &skill_md_s, "run.js", Some("p1")).unwrap();

        // Trusted but no invocation grant → still require_approval (shell-class)
        let r = evaluate_skill_script_run(
            &db,
            &root,
            &skill_md_s,
            "run.js",
            Some("p1"),
            false,
        )
        .unwrap();
        assert_eq!(r["status"], "require_approval");
        assert_eq!(r["trusted"], true);

        // Trusted + granted
        let r = evaluate_skill_script_run(
            &db,
            &root,
            &skill_md_s,
            "run.js",
            Some("p1"),
            true,
        )
        .unwrap();
        // May execute if node present, or fail process — but not require_approval
        if r["status"] == "ok" {
            assert_eq!(r["executed"], true);
        }

        // Change script → hash invalidates trust
        fs::write(root.join("skills/demo/run.js"), "console.log('skill-v2');\n").unwrap();
        let r = evaluate_skill_script_run(
            &db,
            &root,
            &skill_md_s,
            "run.js",
            Some("p1"),
            true,
        )
        .unwrap();
        assert_eq!(r["status"], "require_approval");
        assert_ne!(r["contentHash"].as_str().unwrap(), hash1);
        assert!(!trust::is_trusted(
            &db,
            TrustKind::SkillScript,
            &subject,
            r["contentHash"].as_str().unwrap(),
            Some("p1"),
        )
        .unwrap());
    }
}
