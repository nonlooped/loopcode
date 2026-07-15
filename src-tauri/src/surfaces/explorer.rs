//! Project tree listing with noise-dir filter + show-excluded.

use crate::security::path::{classify_path, PathClass};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Common noise directory/file names hidden by default.
pub const NOISE_NAMES: &[&str] = &[
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
    ".turbo",
    ".cache",
];

const MAX_ENTRIES: usize = 2000;
const DEFAULT_MAX_DEPTH: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub excluded: bool,
    pub children: Vec<TreeEntry>,
}

pub fn is_noise_name(name: &str) -> bool {
    NOISE_NAMES.contains(&name)
}

/// List project tree from root. Noise dirs hidden unless `show_excluded`.
pub fn list_project_tree(
    root: &Path,
    show_excluded: bool,
    max_depth: Option<usize>,
) -> Result<Vec<TreeEntry>, String> {
    if !root.is_dir() {
        return Err(format!("not a directory: {}", root.display()));
    }
    let depth = max_depth.unwrap_or(DEFAULT_MAX_DEPTH);
    let mut count = 0usize;
    walk_dir(root, root, ".", show_excluded, depth, &mut count)
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    rel: &str,
    show_excluded: bool,
    remaining_depth: usize,
    count: &mut usize,
) -> Result<Vec<TreeEntry>, String> {
    if *count >= MAX_ENTRIES || remaining_depth == 0 {
        return Ok(vec![]);
    }
    let rd = fs::read_dir(dir).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    let mut names: Vec<_> = rd.flatten().collect();
    names.sort_by_key(|e| e.file_name());

    for ent in names {
        if *count >= MAX_ENTRIES {
            break;
        }
        let name = ent.file_name().to_string_lossy().to_string();
        let excluded = is_noise_name(&name);
        if excluded && !show_excluded {
            continue;
        }
        let child_rel = if rel == "." {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        // Bound path via security classifier
        let class = classify_path(root, &child_rel);
        if class.class == PathClass::Rejected || class.class == PathClass::OutsideWorkspace {
            continue;
        }
        let meta = ent.metadata().ok();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        *count += 1;
        let children = if is_dir && remaining_depth > 1 {
            walk_dir(
                root,
                &ent.path(),
                &child_rel,
                show_excluded,
                remaining_depth - 1,
                count,
            )?
        } else {
            vec![]
        };
        entries.push(TreeEntry {
            name,
            path: child_rel,
            is_dir,
            size,
            excluded,
            children,
        });
    }
    Ok(entries)
}

/// Resolve a relative path inside root (shared helper for surfaces).
pub fn resolve_in_workspace(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let c = classify_path(root, rel);
    match c.class {
        PathClass::Rejected => Err(c.reason),
        PathClass::OutsideWorkspace => Err(format!("path outside workspace: {}", c.reason)),
        _ => Ok(c
            .normalized
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join(rel))),
    }
}

/// Open a workspace-relative path with the OS default application (editor for text).
/// Path is still Core-bounded — no WebView FS/opener permissions required.
pub fn open_path_external(root: &Path, rel: &str) -> Result<(), String> {
    let path = resolve_in_workspace(root, rel)?;
    if !path.exists() {
        return Err(format!("path does not exist: {rel}"));
    }
    if !path.is_file() {
        return Err(format!("not a file: {rel}"));
    }
    open_with_default_app(&path.to_string_lossy())
}

/// Open an http(s) URL in the OS default browser (Core-side; no WebView opener perm).
pub fn open_url_external(url: &str) -> Result<(), String> {
    if url.chars().any(|c| c.is_control()) {
        return Err("url contains invalid characters".into());
    }
    let trimmed = url.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err("only http/https URLs can be opened".into());
    }
    // Reject embedded whitespace (after trim) that could break shell invocation.
    if trimmed.chars().any(|c| c.is_whitespace()) {
        return Err("url contains invalid characters".into());
    }
    open_with_default_app(trimmed)
}

fn open_with_default_app(target: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        // Do not route untrusted paths or URLs through cmd.exe: its parser treats
        // metacharacters as commands even when Rust passes separate arguments.
        std::process::Command::new("explorer.exe")
            .arg(target)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("failed to open in default app: {e}"))?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(target)
            .spawn()
            .map_err(|e| format!("failed to open in default app: {e}"))?;
        Ok(())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|e| format!("failed to open in default app: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "lc-tree-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn hides_noise_unless_show_excluded() {
        let root = temp();
        fs::write(root.join("app.rs"), "fn main() {}").unwrap();
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        fs::write(root.join("node_modules/pkg/x.js"), "1").unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "").unwrap();

        let hidden = list_project_tree(&root, false, Some(3)).unwrap();
        assert!(hidden.iter().any(|e| e.name == "src"));
        assert!(hidden.iter().any(|e| e.name == "app.rs"));
        assert!(!hidden.iter().any(|e| e.name == "node_modules"));

        let shown = list_project_tree(&root, true, Some(3)).unwrap();
        let nm = shown.iter().find(|e| e.name == "node_modules").unwrap();
        assert!(nm.excluded);
    }

    #[test]
    fn open_url_rejects_non_http() {
        assert!(open_url_external("ftp://example.com").is_err());
        assert!(open_url_external("javascript:alert(1)").is_err());
        assert!(open_url_external("file:///tmp/x").is_err());
        assert!(open_url_external("https://example.com\n").is_err());
    }
}
