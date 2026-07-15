//! Project-root path boundary: normalize, classify, reject escapes.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// Classification of a path relative to the project root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathClass {
    /// Inside project root, not protected.
    InWorkspace,
    /// Inside root but always-prompt (e.g. `.git`, credential filenames).
    ProtectedInWorkspace,
    /// Resolves outside the project root (always prompt / never free allow).
    OutsideWorkspace,
    /// Path is invalid or escape attempt that cannot be classified safely.
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathClassification {
    pub class: PathClass,
    /// Absolute/normalized path when known.
    pub normalized: Option<String>,
    pub relative: Option<String>,
    pub reason: String,
}

/// Filenames that are credential-like (protected even inside workspace).
const CREDENTIAL_NAMES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    ".env.development",
    ".env.test",
    ".npmrc",
    ".netrc",
    ".pypirc",
    ".awscredentials",
    "credentials",
    "id_rsa",
    "id_ed25519",
    "id_dsa",
    "credentials.json",
    "secrets.json",
    "service-account.json",
];

const CREDENTIAL_SUFFIXES: &[&str] = &[".pem", ".key", ".p12", ".pfx", ".jks"];

/// Normalize a user/tool path: reject `..` components before join when relative
/// and resolve the deepest existing ancestor. Resolving the ancestor is important:
/// a new target below an in-workspace symlink must inherit that symlink's real
/// destination before policy decides whether it is safe.
pub fn classify_path(project_root: &Path, candidate: &str) -> PathClassification {
    let root = match canonicalize_or_abs(project_root) {
        Ok(r) => r,
        Err(e) => {
            return PathClassification {
                class: PathClass::Rejected,
                normalized: None,
                relative: None,
                reason: format!("invalid project root: {e}"),
            };
        }
    };

    let raw = candidate.trim();
    if raw.is_empty() {
        return PathClassification {
            class: PathClass::Rejected,
            normalized: None,
            relative: None,
            reason: "empty path".into(),
        };
    }

    // Reject explicit .. segments in the candidate string before resolution.
    let path = Path::new(raw);
    if has_parent_component(path) {
        // Still attempt join+canonicalize to detect escapes; never free-allow.
        return classify_after_resolve(&root, path, "path contains '..' component");
    }

    classify_after_resolve(&root, path, "classified")
}

fn classify_after_resolve(root: &Path, path: &Path, note: &str) -> PathClassification {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };

    let resolved = match resolve_through_existing_ancestor(&joined) {
        Ok(p) => p,
        Err(e) => {
            return PathClassification {
                class: PathClass::Rejected,
                normalized: Some(joined.display().to_string()),
                relative: None,
                reason: e,
            };
        }
    };

    let root_norm = strip_unc(root.to_path_buf());
    if !is_within_root(&root_norm, &resolved) {
        return PathClassification {
            class: PathClass::OutsideWorkspace,
            normalized: Some(resolved.display().to_string()),
            relative: None,
            reason: format!("outside project root ({note})"),
        };
    }

    let rel = pathdiff_relative(&root_norm, &resolved);
    if is_protected_relative(&rel) {
        return PathClassification {
            class: PathClass::ProtectedInWorkspace,
            normalized: Some(resolved.display().to_string()),
            relative: Some(rel),
            reason: "protected path (always prompt)".into(),
        };
    }

    PathClassification {
        class: PathClass::InWorkspace,
        normalized: Some(resolved.display().to_string()),
        relative: Some(rel),
        reason: "inside workspace".into(),
    }
}

fn resolve_through_existing_ancestor(joined: &Path) -> Result<PathBuf, String> {
    let mut ancestor = joined.to_path_buf();
    let mut suffix = Vec::new();
    while ancestor.symlink_metadata().is_err() {
        let name = ancestor
            .file_name()
            .ok_or_else(|| "path has no existing ancestor".to_string())?
            .to_os_string();
        suffix.push(name);
        if !ancestor.pop() {
            return Err("path has no existing ancestor".into());
        }
    }
    let mut resolved = ancestor
        .canonicalize()
        .map(strip_unc)
        .map_err(|e| format!("canonicalize failed: {e}"))?;
    for component in suffix.iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

fn has_parent_component(path: &Path) -> bool {
    path.components().any(|c| matches!(c, Component::ParentDir))
}

fn canonicalize_or_abs(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        path.canonicalize()
            .map(strip_unc)
            .map_err(|e| e.to_string())
    } else if path.is_absolute() {
        Ok(lexical_normalize(path))
    } else {
        let abs = std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path);
        Ok(lexical_normalize(&abs))
    }
}

fn strip_unc(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        p
    }
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn is_within_root(root: &Path, candidate: &Path) -> bool {
    #[cfg(windows)]
    {
        // Windows paths are case-insensitive, unlike Unix paths.
        let root_s = root.to_string_lossy().to_ascii_lowercase();
        let candidate_s = candidate.to_string_lossy().to_ascii_lowercase();
        let root = PathBuf::from(root_s.replace('/', "\\"));
        let candidate = PathBuf::from(candidate_s.replace('/', "\\"));
        candidate.starts_with(root)
    }
    #[cfg(not(windows))]
    {
        candidate.starts_with(root)
    }
}

fn pathdiff_relative(root: &Path, full: &Path) -> String {
    full.strip_prefix(root)
        .unwrap_or(full)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Protected relative paths: `.git/**` writes/access classification and credential names.
pub fn is_protected_relative(rel: &str) -> bool {
    let rel = rel.replace('\\', "/");
    let lower = rel.to_ascii_lowercase();
    if lower == ".git" || lower.starts_with(".git/") {
        return true;
    }
    let name = Path::new(&rel)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.starts_with(".env.") || CREDENTIAL_NAMES.iter().any(|n| name == *n) {
        return true;
    }
    CREDENTIAL_SUFFIXES.iter().any(|sfx| name.ends_with(sfx))
}

/// Convenience: true if path is free for ordinary in-workspace ops (read or non-protected write).
pub fn is_free_in_workspace(project_root: &Path, candidate: &str) -> bool {
    matches!(
        classify_path(project_root, candidate).class,
        PathClass::InWorkspace
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_root() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "loopcode-path-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn inside_root_is_in_workspace() {
        let root = temp_root();
        fs::write(root.join("src/main.rs"), "x").ok();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "x").unwrap();
        let c = classify_path(&root, "src/main.rs");
        assert_eq!(c.class, PathClass::InWorkspace, "{c:?}");
    }

    #[test]
    fn parent_escape_is_outside_or_rejected() {
        let root = temp_root();
        let c = classify_path(&root, "../outside.txt");
        assert!(
            matches!(c.class, PathClass::OutsideWorkspace | PathClass::Rejected),
            "{c:?}"
        );
    }

    #[test]
    fn git_path_is_protected() {
        let root = temp_root();
        fs::create_dir_all(root.join(".git")).unwrap();
        let c = classify_path(&root, ".git/config");
        assert_eq!(c.class, PathClass::ProtectedInWorkspace, "{c:?}");
    }

    #[test]
    fn env_file_is_protected() {
        let root = temp_root();
        let c = classify_path(&root, ".env");
        assert_eq!(c.class, PathClass::ProtectedInWorkspace, "{c:?}");
    }
}
