//! Path-bounded open/save for Core-owned FS (agent diffs, conflict checks).

use crate::security::path::PathClass;
use crate::surfaces::explorer::resolve_in_workspace;
use crate::tools::hash::{content_hash_bytes, content_hash_str};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const MAX_EDIT_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenFileDto {
    pub path: String,
    pub content: String,
    pub hash: String,
    pub encoding: String,
    pub binary: bool,
    pub size: u64,
    pub language_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveFileResult {
    pub path: String,
    pub hash: String,
    pub bytes_written: usize,
    pub status: String,
}

/// Open a text file for the editor. Binary files return binary=true, no editable content.
pub fn open_text_file(root: &Path, rel: &str) -> Result<OpenFileDto, String> {
    let path = resolve_in_workspace(root, rel)?;
    if !path.is_file() {
        return Err(format!("not a file: {rel}"));
    }
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_EDIT_BYTES {
        return Err(format!(
            "file too large for editor ({} > {} bytes)",
            meta.len(),
            MAX_EDIT_BYTES
        ));
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let hash = content_hash_bytes(&bytes);
    let language_hint = language_from_path(rel);
    match String::from_utf8(bytes.clone()) {
        Ok(content) => Ok(OpenFileDto {
            path: rel.to_string(),
            content,
            hash,
            encoding: "utf-8".into(),
            binary: false,
            size: meta.len(),
            language_hint,
        }),
        Err(_) => Ok(OpenFileDto {
            path: rel.to_string(),
            content: format!("[binary {} bytes — cannot edit]", bytes.len()),
            hash,
            encoding: "binary".into(),
            binary: true,
            size: meta.len(),
            language_hint: "plaintext".into(),
        }),
    }
}

/// Save editor buffer through Core. Optional expected_hash detects intervening disk edits.
pub fn save_text_file(
    root: &Path,
    rel: &str,
    content: &str,
    expected_hash: Option<&str>,
) -> Result<SaveFileResult, String> {
    let path = resolve_in_workspace(root, rel)?;
    // Re-classify for protected
    let class = crate::security::path::classify_path(root, rel);
    if class.class == PathClass::ProtectedInWorkspace {
        return Err("protected path requires approval".into());
    }
    if path.exists() {
        let current = fs::read(&path).map_err(|e| e.to_string())?;
        let current_hash = content_hash_bytes(&current);
        if let Some(exp) = expected_hash {
            if exp != current_hash {
                return Err(format!(
                    "hash_conflict: expected {exp}, current {current_hash} (intervening edit)"
                ));
            }
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, content).map_err(|e| e.to_string())?;
    let hash = content_hash_str(content);
    Ok(SaveFileResult {
        path: rel.to_string(),
        hash,
        bytes_written: content.len(),
        status: "ok".into(),
    })
}

fn language_from_path(path: &str) -> String {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "json" => "json",
        "md" => "markdown",
        "py" => "python",
        "toml" => "toml",
        "css" => "css",
        "html" | "htm" => "html",
        "yml" | "yaml" => "yaml",
        "sh" | "bash" => "shell",
        "ps1" => "powershell",
        _ => "plaintext",
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "lc-ed-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn open_save_roundtrip_and_hash_conflict() {
        let root = temp();
        fs::write(root.join("a.txt"), "hello").unwrap();
        let open = open_text_file(&root, "a.txt").unwrap();
        assert_eq!(open.content, "hello");
        assert!(!open.binary);

        let saved = save_text_file(&root, "a.txt", "world", Some(&open.hash)).unwrap();
        assert_eq!(saved.status, "ok");
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "world");

        let err = save_text_file(&root, "a.txt", "x", Some("deadbeef:0")).unwrap_err();
        assert!(err.contains("hash_conflict"), "{err}");
    }
}
