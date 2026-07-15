//! `edit` — exact string replacement (OpenCode-style), with optional pre-image hash.

use crate::security::path::{classify_path, PathClass};
use crate::tools::hash::{content_hash_bytes, content_hash_str};
use crate::tools::linediff::{line_counts, lines_of};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

fn resolve_in_root(root: &Path, rel: &str) -> Result<(std::path::PathBuf, PathClass), String> {
    let c = classify_path(root, rel);
    match c.class {
        PathClass::Rejected => Err(c.reason),
        PathClass::OutsideWorkspace => Err(format!("path outside workspace: {}", c.reason)),
        other => {
            let p = c
                .normalized
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| root.join(rel));
            Ok((p, other))
        }
    }
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn detect_ending(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn to_ending(text: &str, ending: &str) -> String {
    if ending == "\n" {
        normalize_newlines(text)
    } else {
        normalize_newlines(text).replace('\n', "\r\n")
    }
}

/// Apply exact string replacement.
///
/// Args:
/// - `path`: relative path
/// - `oldString` / `old_string`: text to find
/// - `newString` / `new_string`: replacement (must differ)
/// - `replaceAll` / `replace_all`: replace every occurrence (default false)
/// - `expectedHash` (optional): fail closed on intervening edits when file exists
pub fn edit_file(root: &Path, args: &Value) -> Result<Value, String> {
    let rel = args
        .get("path")
        .or_else(|| args.get("filePath"))
        .or_else(|| args.get("file_path"))
        .and_then(|v| v.as_str())
        .ok_or("path required")?;
    let old = args
        .get("oldString")
        .or_else(|| args.get("old_string"))
        .and_then(|v| v.as_str())
        .ok_or("oldString required")?;
    let new = args
        .get("newString")
        .or_else(|| args.get("new_string"))
        .and_then(|v| v.as_str())
        .ok_or("newString required")?;
    let replace_all = args
        .get("replaceAll")
        .or_else(|| args.get("replace_all"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let expected = args
        .get("expectedHash")
        .or_else(|| args.get("expected_hash"))
        .and_then(|v| v.as_str());

    if old == new {
        return Err("No changes to apply: oldString and newString are identical.".into());
    }

    // Runtime policy has already classified and approved protected paths before
    // dispatching to this executor; rejecting them here made approved writes
    // impossible to complete.
    let (path, _) = resolve_in_root(root, rel)?;

    // Empty oldString → create new file (OpenCode convention).
    if old.is_empty() {
        if path.exists() {
            return Err(
                "oldString cannot be empty when editing an existing file. Provide the exact text to replace, or use write for a full-file replacement."
                    .into(),
            );
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, new).map_err(|e| e.to_string())?;
        return Ok(json!({
            "status": "ok",
            "path": rel,
            "hash": content_hash_str(new),
            "bytesWritten": new.len(),
            "replacements": 1,
            "created": true,
            "linesAdded": lines_of(new),
            "linesRemoved": 0,
        }));
    }

    if !path.is_file() {
        return Err(format!("file not found: {rel}"));
    }

    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let current_hash = content_hash_bytes(&bytes);
    if let Some(exp) = expected {
        if exp != current_hash {
            return Err(format!(
                "hash_conflict: expected {exp}, current {current_hash} (intervening edit or stale base)"
            ));
        }
    }

    let content = String::from_utf8(bytes)
        .map_err(|_| format!("file is not valid UTF-8: {rel}"))?;
    let ending = detect_ending(&content);
    let old_n = to_ending(old, ending);
    let new_n = to_ending(new, ending);

    let count = content.matches(&old_n).count();
    if count == 0 {
        return Err(format!(
            "oldString not found in {rel}. Ensure the exact text matches (including whitespace)."
        ));
    }
    if count > 1 && !replace_all {
        return Err(format!(
            "oldString found {count} times in {rel}. Provide a more unique string, or set replaceAll=true."
        ));
    }

    let next = if replace_all {
        content.replace(&old_n, &new_n)
    } else {
        content.replacen(&old_n, &new_n, 1)
    };
    let replacements = if replace_all { count } else { 1 };

    fs::write(&path, &next).map_err(|e| e.to_string())?;
    let (added, removed) = line_counts(&content, &next);
    Ok(json!({
        "status": "ok",
        "path": rel,
        "hash": content_hash_str(&next),
        "bytesWritten": next.len(),
        "replacements": replacements,
        "created": false,
        "linesAdded": added,
        "linesRemoved": removed,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "lc-edit-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn replaces_unique_string() {
        let root = tmp();
        fs::write(root.join("a.rs"), "fn foo() {}\nfn bar() {}\n").unwrap();
        let r = edit_file(
            &root,
            &json!({
                "path": "a.rs",
                "oldString": "fn foo() {}",
                "newString": "fn foo() { 1 }"
            }),
        )
        .unwrap();
        assert_eq!(r["replacements"], 1);
        let body = fs::read_to_string(root.join("a.rs")).unwrap();
        assert!(body.contains("fn foo() { 1 }"));
        assert!(body.contains("fn bar() {}"));
    }

    #[test]
    fn rejects_ambiguous_without_replace_all() {
        let root = tmp();
        fs::write(root.join("a.txt"), "x\nx\n").unwrap();
        let err = edit_file(
            &root,
            &json!({
                "path": "a.txt",
                "oldString": "x",
                "newString": "y"
            }),
        )
        .unwrap_err();
        assert!(err.contains("found 2 times"), "{err}");
    }
}
