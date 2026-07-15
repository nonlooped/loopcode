//! Real workspace glob/read/grep/write/patch tools.

use crate::security::path::{classify_path, PathClass};
use crate::tools::hash::{content_hash_bytes, content_hash_str};
use crate::tools::linediff::{line_counts, lines_of};
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::collections::HashSet;

const MAX_READ_BYTES: u64 = 512 * 1024;
/// Default match cap for `grep` (callers may raise via `maxMatches`).
pub const MAX_SEARCH_MATCHES: usize = 200;
const MAX_SEARCH_MATCHES_HARD: usize = 1000;
const MAX_SEARCH_FILES: usize = 2000;
const MAX_GLOB_MATCHES: usize = 500;

const IGNORE_DIR_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".svn",
    ".hg",
    "__pycache__",
    ".next",
    "vendor",
];

fn resolve_in_root(root: &Path, rel: &str) -> Result<(PathBuf, PathClass), String> {
    let c = classify_path(root, rel);
    match c.class {
        PathClass::Rejected => Err(c.reason),
        PathClass::OutsideWorkspace => Err(format!("path outside workspace: {}", c.reason)),
        other => {
            let p = c
                .normalized
                .map(PathBuf::from)
                .unwrap_or_else(|| root.join(rel));
            Ok((p, other))
        }
    }
}

/// `glob` — find files by glob pattern (e.g. `**/*.rs`), relative to project root.
pub fn workspace_glob(root: &Path, args: &Value) -> Result<Value, String> {
    let pattern = args
        .get("pattern")
        .or_else(|| args.get("glob"))
        .and_then(|v| v.as_str())
        .ok_or("pattern required")?;
    let search_rel = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let (search_root, _) = resolve_in_root(root, search_rel)?;
    if !search_root.is_dir() {
        return Err(format!("not a directory: {search_rel}"));
    }

    let mut matches = Vec::new();
    walk_glob(root, &search_root, pattern, &mut matches)?;
    matches.sort();
    let capped = matches.len() > MAX_GLOB_MATCHES;
    matches.truncate(MAX_GLOB_MATCHES);

    Ok(json!({
        "status": "ok",
        "pattern": pattern,
        "path": search_rel,
        "matchCount": matches.len(),
        "matches": matches,
        "capped": capped,
    }))
}

fn walk_glob(
    root: &Path,
    dir: &Path,
    pattern: &str,
    matches: &mut Vec<String>,
) -> Result<(), String> {
    if matches.len() >= MAX_GLOB_MATCHES {
        return Ok(());
    }
    let rd = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    for ent in rd.flatten() {
        if matches.len() >= MAX_GLOB_MATCHES {
            break;
        }
        let name = ent.file_name().to_string_lossy().to_string();
        if IGNORE_DIR_NAMES.contains(&name.as_str()) {
            continue;
        }
        let Ok(kind) = ent.file_type() else { continue };
        // Do not let a workspace glob follow a directory symlink outside root.
        if kind.is_symlink() {
            continue;
        }
        let path = ent.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            if glob_match(pattern, &rel) || glob_match(pattern, &format!("{rel}/")) {
                // Pattern matched a directory path — still recurse for file matches.
            }
            walk_glob(root, &path, pattern, matches)?;
        } else if path.is_file() && glob_match(pattern, &rel) {
            matches.push(rel);
        }
    }
    Ok(())
}

/// Minimal glob matcher: `*` (no `/`), `?`, and `**` (across segments).
fn glob_match(pattern: &str, path: &str) -> bool {
    let pat_norm = pattern.replace('\\', "/");
    let path_norm = path.replace('\\', "/");
    let pat: Vec<&str> = pat_norm.split('/').collect();
    let path: Vec<&str> = path_norm.split('/').filter(|s| !s.is_empty()).collect();
    glob_segs(&pat, &path)
}

fn glob_segs(pat: &[&str], path: &[&str]) -> bool {
    match (pat.first(), path.first()) {
        (None, None) => true,
        (Some(&"**"), _) => {
            // `**` may consume zero or more path segments
            if glob_segs(&pat[1..], path) {
                return true;
            }
            if path.is_empty() {
                return false;
            }
            glob_segs(pat, &path[1..])
        }
        (Some(p), Some(s)) if seg_match(p, s) => glob_segs(&pat[1..], &path[1..]),
        (Some(_), Some(_)) | (Some(_), None) | (None, Some(_)) => false,
    }
}

fn seg_match(pat: &str, seg: &str) -> bool {
    let pb = pat.as_bytes();
    let sb = seg.as_bytes();
    let mut pi = 0usize;
    let mut si = 0usize;
    let mut star = None::<(usize, usize)>;
    while si < sb.len() {
        if pi < pb.len() && (pb[pi] == b'?' || pb[pi] == sb[si]) {
            pi += 1;
            si += 1;
        } else if pi < pb.len() && pb[pi] == b'*' {
            star = Some((pi + 1, si));
            pi += 1;
        } else if let Some((rp, rs)) = star {
            pi = rp;
            si = rs + 1;
            star = Some((rp, rs + 1));
        } else {
            return false;
        }
    }
    while pi < pb.len() && pb[pi] == b'*' {
        pi += 1;
    }
    pi == pb.len()
}

/// `read`
pub fn workspace_read(root: &Path, args: &Value) -> Result<Value, String> {
    let rel = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("path required")?;
    let (path, _) = resolve_in_root(root, rel)?;
    if !path.is_file() {
        return Err(format!("not a file: {rel}"));
    }
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_READ_BYTES {
        return Err(format!(
            "file too large ({} > {} bytes)",
            meta.len(),
            MAX_READ_BYTES
        ));
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let hash = content_hash_bytes(&bytes);
    match String::from_utf8(bytes.clone()) {
        Ok(text) => Ok(json!({
            "status": "ok",
            "path": rel,
            "encoding": "utf-8",
            "hash": hash,
            "content": text,
            "size": text.len(),
        })),
        Err(_) => Ok(json!({
            "status": "ok",
            "path": rel,
            "encoding": "base64",
            "hash": hash,
            "content": base64_encode(&bytes),
            "size": bytes.len(),
        })),
    }
}

/// `write` — whole-file write (Build only at mode layer).
pub fn workspace_write(root: &Path, args: &Value) -> Result<Value, String> {
    let rel = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("path required")?;
    let content = args
        .get("content")
        .or_else(|| args.get("body"))
        .and_then(|v| v.as_str())
        .ok_or("content required")?;
    let (path, _) = resolve_in_root(root, rel)?;
    let expected = args
        .get("expectedHash")
        .or_else(|| args.get("expected_hash"))
        .or_else(|| args.get("pre_image_hash"))
        .or_else(|| args.get("preImageHash"))
        .and_then(|v| v.as_str());
    // Pre-image for line stats (best-effort; binary/unreadable → additions only).
    let before = if path.is_file() {
        let current = fs::read(&path).map_err(|e| e.to_string())?;
        let current_hash = content_hash_bytes(&current);
        match expected {
            Some(exp) if exp == current_hash => {}
            None => return Err(format!("hash_conflict: file exists but expected_hash omitted (current={current_hash})")),
            Some(exp) => return Err(format!("hash_conflict: expected {exp}, current {current_hash} (intervening edit or stale base)")),
        }
        String::from_utf8(current).ok()
    } else {
        if let Some(exp) = expected {
            if !matches!(exp, "" | "new" | "absent") {
                return Err(format!("hash_conflict: file missing but expected_hash was {exp}"));
            }
        }
        None
    };
    let created = before.is_none() && !path.exists();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, content).map_err(|e| e.to_string())?;
    let hash = content_hash_str(content);
    let (added, removed) = match &before {
        Some(b) => line_counts(b, content),
        None => (lines_of(content), 0),
    };
    Ok(json!({
        "status": "ok",
        "path": rel,
        "hash": hash,
        "bytesWritten": content.len(),
        "created": created,
        "linesAdded": added,
        "linesRemoved": removed,
    }))
}

/// `patch` with pre-image hash conflict detection.
///
/// Args:
/// - `path`: relative path
/// - `content`: new full file body (not required when `delete` is true)
/// - `expected_hash` / `pre_image_hash`: required if file exists; must match current content
/// - `delete`: true to remove the file (expected_hash still required; fail-closed)
pub fn workspace_apply_patch(root: &Path, args: &Value) -> Result<Value, String> {
    // Multi-file form
    if let Some(files) = args.get("files").and_then(|v| v.as_array()) {
        // Validate every pre-image before changing the first file. This avoids
        // the old partial-apply behavior when a later file had a conflict.
        let mut paths = HashSet::new();
        for file in files {
            let path = file.get("path").and_then(|v| v.as_str()).ok_or("path required")?;
            if !paths.insert(path) {
                return Err(format!("duplicate path in patch batch: {path}"));
            }
            validate_one_patch(root, file)?;
        }
        let mut results = Vec::new();
        for f in files {
            results.push(apply_one_patch(root, f)?);
        }
        return Ok(json!({"status": "ok", "files": results}));
    }
    apply_one_patch(root, args)
}

fn validate_one_patch(root: &Path, args: &Value) -> Result<(), String> {
    let rel = args.get("path").and_then(|v| v.as_str()).ok_or("path required")?;
    let delete = args.get("delete").and_then(|v| v.as_bool()).unwrap_or(false);
    if !delete
        && args.get("content").or_else(|| args.get("body")).or_else(|| args.get("newContent")).or_else(|| args.get("new_content")).and_then(|v| v.as_str()).is_none()
    {
        return Err("content required (or newContent)".into());
    }
    let expected = args.get("expectedHash").or_else(|| args.get("expected_hash"))
        .or_else(|| args.get("pre_image_hash")).or_else(|| args.get("preImageHash"))
        .and_then(|v| v.as_str());
    let (path, _) = resolve_in_root(root, rel)?;
    if path.exists() {
        let current = fs::read(&path).map_err(|e| e.to_string())?;
        let hash = content_hash_bytes(&current);
        match expected {
            Some(exp) if exp == hash => Ok(()),
            None => Err(format!("hash_conflict: file exists but expected_hash omitted (current={hash})")),
            Some(exp) => Err(format!("hash_conflict: expected {exp}, current {hash} (intervening edit or stale base)")),
        }
    } else if delete {
        Err(format!("cannot delete: file not found: {rel}"))
    } else if matches!(expected, Some(exp) if !matches!(exp, "" | "new" | "absent")) {
        Err(format!("hash_conflict: file missing but expected_hash was {}", expected.unwrap()))
    } else {
        Ok(())
    }
}

fn apply_one_patch(root: &Path, args: &Value) -> Result<Value, String> {
    let rel = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("path required")?;
    let delete = args
        .get("delete")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // Accept executor shape (`content`) and provider wire shape (`newContent`).
    let content = args
        .get("content")
        .or_else(|| args.get("body"))
        .or_else(|| args.get("newContent"))
        .or_else(|| args.get("new_content"))
        .and_then(|v| v.as_str());
    let content = match (delete, content) {
        (false, None) => return Err("content required (or newContent)".into()),
        (_, c) => c,
    };
    let expected = args
        .get("expectedHash")
        .or_else(|| args.get("expected_hash"))
        .or_else(|| args.get("pre_image_hash"))
        .or_else(|| args.get("preImageHash"))
        .and_then(|v| v.as_str());

    let (path, _) = resolve_in_root(root, rel)?;

    let mut before: Option<String> = None;
    if path.exists() {
        let current = fs::read(&path).map_err(|e| e.to_string())?;
        let current_hash = content_hash_bytes(&current);
        before = String::from_utf8(current.clone()).ok();
        match expected {
            None => {
                return Err(format!(
                    "hash_conflict: file exists but expected_hash omitted (current={current_hash})"
                ));
            }
            Some(exp) if exp != current_hash => {
                return Err(format!(
                    "hash_conflict: expected {exp}, current {current_hash} (intervening edit or stale base)"
                ));
            }
            _ => {}
        }
    } else if delete {
        return Err(format!("cannot delete: file not found: {rel}"));
    } else if let Some(exp) = expected {
        // Creating new file with a non-empty expected hash is a conflict
        if !matches!(exp, "" | "new" | "absent") {
            return Err(format!(
                "hash_conflict: file missing but expected_hash was {exp}"
            ));
        }
    }

    if delete {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
        let removed = before.as_deref().map(lines_of).unwrap_or(0);
        // Bounded pre-image preview so the event payload stays small.
        let before_preview = before.as_ref().map(|s| s.chars().take(2000).collect::<String>());
        return Ok(json!({
            "status": "ok",
            "path": rel,
            "deleted": true,
            "beforePreview": before_preview,
            "linesAdded": 0,
            "linesRemoved": removed,
        }));
    }
    let content = content.expect("content checked above for non-delete");

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let created = !path.exists();
    fs::write(&path, content).map_err(|e| e.to_string())?;
    let new_hash = content_hash_str(content);
    let (added, removed) = match &before {
        Some(b) => line_counts(b, content),
        None => (lines_of(content), 0),
    };
    Ok(json!({
        "status": "ok",
        "path": rel,
        "hash": new_hash,
        "bytesWritten": content.len(),
        "created": created,
        "linesAdded": added,
        "linesRemoved": removed,
    }))
}

/// Bounded content search (`grep`).
pub fn workspace_search(root: &Path, args: &Value) -> Result<Value, String> {
    let query = args
        .get("query")
        .or_else(|| args.get("pattern"))
        .and_then(|v| v.as_str())
        .ok_or("query required")?;
    let case_insensitive = args
        .get("caseInsensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let max_matches = args
        .get("maxMatches")
        .and_then(|v| v.as_u64())
        .unwrap_or(MAX_SEARCH_MATCHES as u64)
        .clamp(1, MAX_SEARCH_MATCHES_HARD as u64) as usize;
    let offset = args
        .get("offset")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let needle = if case_insensitive {
        query.to_ascii_lowercase()
    } else {
        query.to_string()
    };

    // Collect past the page so we can report whether more matches exist.
    let collect_limit = max_matches
        .saturating_add(offset)
        .saturating_add(1)
        .min(MAX_SEARCH_MATCHES_HARD);

    let mut matches = Vec::new();
    let mut files_scanned = 0usize;
    let search_root = match args.get("path").and_then(|v| v.as_str()) {
        Some(path) => resolve_in_root(root, path)?.0,
        None => root.to_path_buf(),
    };
    walk_search(
        root,
        &search_root,
        &needle,
        case_insensitive,
        collect_limit,
        &mut matches,
        &mut files_scanned,
    )?;

    let total_found = matches.len();
    let page: Vec<Value> = matches
        .into_iter()
        .skip(offset)
        .take(max_matches)
        .collect();
    let has_more = total_found > offset + page.len();
    let next_offset = if has_more {
        Some(offset + page.len())
    } else {
        None
    };

    Ok(json!({
        "status": "ok",
        "query": query,
        "filesScanned": files_scanned,
        "matchCount": page.len(),
        "matches": page,
        "offset": offset,
        "nextOffset": next_offset,
        "capped": has_more || total_found >= collect_limit,
        "hint": if has_more {
            "More matches available — call again with offset=nextOffset"
        } else {
            ""
        },
    }))
}

fn walk_search(
    root: &Path,
    dir: &Path,
    needle: &str,
    case_insensitive: bool,
    max_matches: usize,
    matches: &mut Vec<Value>,
    files_scanned: &mut usize,
) -> Result<(), String> {
    if matches.len() >= max_matches || *files_scanned >= MAX_SEARCH_FILES {
        return Ok(());
    }
    let rd = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    for ent in rd.flatten() {
        if matches.len() >= max_matches || *files_scanned >= MAX_SEARCH_FILES {
            break;
        }
        let name = ent.file_name().to_string_lossy().to_string();
        if IGNORE_DIR_NAMES.contains(&name.as_str()) {
            continue;
        }
        let Ok(kind) = ent.file_type() else { continue };
        if kind.is_symlink() {
            continue;
        }
        let path = ent.path();
        if path.is_dir() {
            walk_search(
                root,
                &path,
                needle,
                case_insensitive,
                max_matches,
                matches,
                files_scanned,
            )?;
        } else if path.is_file() {
            *files_scanned += 1;
            if let Ok(meta) = path.metadata() {
                if meta.len() > MAX_READ_BYTES {
                    continue;
                }
            }
            search_file(root, &path, needle, case_insensitive, max_matches, matches)?;
        }
    }
    Ok(())
}

fn search_file(
    root: &Path,
    path: &Path,
    needle: &str,
    case_insensitive: bool,
    max_matches: usize,
    matches: &mut Vec<Value>,
) -> Result<(), String> {
    let f = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Ok(()),
    };
    let reader = BufReader::new(f);
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    for (i, line) in reader.lines().enumerate() {
        if matches.len() >= max_matches {
            break;
        }
        let Ok(line) = line else { continue };
        let hay = if case_insensitive {
            line.to_ascii_lowercase()
        } else {
            line.clone()
        };
        if hay.contains(needle) {
            matches.push(json!({
                "path": rel,
                "line": i + 1,
                "text": line.chars().take(200).collect::<String>(),
            }));
        }
    }
    Ok(())
}

fn base64_encode(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(T[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "lc-ws-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn write_read_and_hash_conflict() {
        let root = tmp();
        workspace_write(
            &root,
            &json!({"path": "a.txt", "content": "hello"}),
        )
        .unwrap();
        let r = workspace_read(&root, &json!({"path": "a.txt"})).unwrap();
        assert_eq!(r["content"], "hello");
        let h = r["hash"].as_str().unwrap().to_string();

        // Stale hash → conflict
        let err = workspace_apply_patch(
            &root,
            &json!({"path": "a.txt", "content": "world", "expected_hash": "deadbeef:0"}),
        )
        .unwrap_err();
        assert!(err.contains("hash_conflict"), "{err}");

        // Correct hash → ok
        workspace_apply_patch(
            &root,
            &json!({"path": "a.txt", "content": "world", "expected_hash": h}),
        )
        .unwrap();
        let r2 = workspace_read(&root, &json!({"path": "a.txt"})).unwrap();
        assert_eq!(r2["content"], "world");
    }

    #[test]
    fn patch_delete_requires_matching_hash_and_removes_file() {
        let root = tmp();
        workspace_write(&root, &json!({"path": "old.txt", "content": "a\nb\nc"})).unwrap();
        let h = workspace_read(&root, &json!({"path": "old.txt"}))
            .unwrap()["hash"]
            .as_str()
            .unwrap()
            .to_string();

        // No hash → fail closed
        let err = workspace_apply_patch(&root, &json!({"path": "old.txt", "delete": true}))
            .unwrap_err();
        assert!(err.contains("hash_conflict"), "{err}");

        // Wrong hash → conflict
        let err = workspace_apply_patch(
            &root,
            &json!({"path": "old.txt", "delete": true, "expected_hash": "deadbeef:0"}),
        )
        .unwrap_err();
        assert!(err.contains("hash_conflict"), "{err}");
        assert!(root.join("old.txt").exists());

        // Correct hash → deleted, all-red line stats
        let r = workspace_apply_patch(
            &root,
            &json!({"path": "old.txt", "delete": true, "expected_hash": h}),
        )
        .unwrap();
        assert_eq!(r["deleted"], true);
        assert_eq!(r["linesAdded"], 0);
        assert_eq!(r["linesRemoved"], 3);
        assert!(r["beforePreview"].as_str().unwrap().contains("a\nb\nc"));
        assert!(!root.join("old.txt").exists());

        // Deleting a missing file errors
        let err = workspace_apply_patch(
            &root,
            &json!({"path": "old.txt", "delete": true, "expected_hash": h}),
        )
        .unwrap_err();
        assert!(err.contains("not found"), "{err}");
    }

    #[test]
    fn search_finds_line() {
        let root = tmp();
        fs::write(root.join("note.md"), "alpha\nfindme please\nbeta\n").unwrap();
        let r = workspace_search(&root, &json!({"query": "findme"})).unwrap();
        assert_eq!(r["matchCount"], 1);
    }

    #[test]
    fn glob_finds_rs_files() {
        let root = tmp();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/a.rs"), "x").unwrap();
        fs::write(root.join("src/b.txt"), "y").unwrap();
        let r = workspace_glob(&root, &json!({"pattern": "**/*.rs"})).unwrap();
        assert_eq!(r["matchCount"], 1);
        assert_eq!(r["matches"][0], "src/a.rs");
    }
}
