//! Content-addressed workspace checkpoints outside the project tree (and outside Git).

use crate::db::store::Database;
use crate::security::{classify_path, PathClass};
use crate::surfaces::explorer::{is_noise_name, resolve_in_workspace};
use crate::tools::hash::content_hash_bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_FILES: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreAxis {
    FilesOnly,
    ConversationOnly,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub hash: String,
    pub skipped: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointManifest {
    pub id: String,
    pub project_id: String,
    pub chat_id: Option<String>,
    pub run_id: Option<String>,
    pub event_cursor_seq: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub store_path: String,
    pub files: Vec<FileEntry>,
    /// Absolute path to checkpoint root (outside project).
    pub root_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterveningChange {
    pub path: String,
    pub checkpoint_hash: String,
    pub live_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    pub axis: RestoreAxis,
    pub checkpoint_id: String,
    pub files_restored: Vec<String>,
    pub files_skipped: Vec<String>,
    pub conversation_cursor: Option<i64>,
    pub intervening: Vec<InterveningChange>,
    pub warned: bool,
    pub applied: bool,
    pub message: String,
}

/// Default store root: `{data_dir}/checkpoints/{project_id}`.
pub fn default_checkpoint_store(data_dir: &Path, project_id: &str) -> PathBuf {
    data_dir.join("checkpoints").join(project_id)
}

/// Create a content-addressed snapshot of the workspace under `store_root`.
/// Never writes under the project tree or into `.git`.
pub fn create_checkpoint(
    workspace_root: &Path,
    store_root: &Path,
    project_id: &str,
    chat_id: Option<&str>,
    run_id: Option<&str>,
    event_cursor_seq: Option<i64>,
) -> Result<CheckpointManifest, String> {
    if !store_is_outside_project(store_root, workspace_root) {
        return Err("checkpoint store must be outside the workspace".into());
    }
    let id = Uuid::new_v4().to_string();
    let cp_dir = store_root.join(&id);
    let blobs = cp_dir.join("blobs");
    fs::create_dir_all(&blobs).map_err(|e| e.to_string())?;

    let mut files = Vec::new();
    walk_collect(workspace_root, workspace_root, &blobs, &mut files)?;

    let manifest = CheckpointManifest {
        id: id.clone(),
        project_id: project_id.into(),
        chat_id: chat_id.map(str::to_string),
        run_id: run_id.map(str::to_string),
        event_cursor_seq,
        created_at: Utc::now(),
        store_path: store_root.display().to_string(),
        files,
        root_dir: cp_dir.display().to_string(),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(cp_dir.join("manifest.json"), manifest_json).map_err(|e| e.to_string())?;

    // Never create .git under checkpoint store
    assert_no_git_refs(&cp_dir)?;
    Ok(manifest)
}

fn walk_collect(
    workspace_root: &Path,
    dir: &Path,
    blobs: &Path,
    files: &mut Vec<FileEntry>,
) -> Result<(), String> {
    if files.len() >= MAX_FILES {
        return Ok(());
    }
    let rd = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    for ent in rd.flatten() {
        if files.len() >= MAX_FILES {
            break;
        }
        let name = ent.file_name().to_string_lossy().to_string();
        if is_noise_name(&name) {
            continue;
        }
        let path = ent.path();
        let file_type = ent.file_type().map_err(|e| e.to_string())?;
        // Never traverse links: they can escape the workspace or create cycles.
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            walk_collect(workspace_root, &path, blobs, files)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let rel = path
            .strip_prefix(workspace_root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        // Credentials are deliberately absent from checkpoints: blobs are ordinary
        // durable storage, not a secret store.
        if classify_path(workspace_root, &rel).class == PathClass::ProtectedInWorkspace {
            continue;
        }
        let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
        if meta.len() > MAX_FILE_BYTES {
            files.push(FileEntry {
                path: rel,
                hash: String::new(),
                skipped: true,
                reason: Some("too_large".into()),
            });
            continue;
        }
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        let hash = content_hash_bytes(&bytes);
        // Hash strings may contain ':' (length suffix); sanitize for Windows paths.
        let blob_path = blobs.join(blob_file_name(&hash));
        if !blob_path.exists() {
            fs::write(&blob_path, &bytes).map_err(|e| e.to_string())?;
        }
        files.push(FileEntry {
            path: rel,
            hash,
            skipped: false,
            reason: None,
        });
    }
    Ok(())
}

fn assert_no_git_refs(cp_dir: &Path) -> Result<(), String> {
    if cp_dir.join(".git").exists() {
        return Err("checkpoint store must not contain .git".into());
    }
    Ok(())
}

/// Load manifest from checkpoint directory.
pub fn load_manifest(cp_dir: &Path) -> Result<CheckpointManifest, String> {
    let raw = fs::read_to_string(cp_dir.join("manifest.json")).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

/// Load only a checkpoint owned by `project_id` from its canonical Core store.
/// The caller may select a checkpoint id, but never redirect restore to an
/// arbitrary manifest or blob directory.
pub fn load_manifest_for_project(
    data_dir: &Path,
    project_id: &str,
    checkpoint_root: &Path,
) -> Result<(CheckpointManifest, PathBuf), String> {
    let store = default_checkpoint_store(data_dir, project_id);
    let store = store
        .canonicalize()
        .map_err(|_| "checkpoint store does not exist".to_string())?;
    let checkpoint = checkpoint_root
        .canonicalize()
        .map_err(|_| "checkpoint directory does not exist".to_string())?;
    if checkpoint.parent() != Some(store.as_path()) {
        return Err("checkpoint is not in this project's Core checkpoint store".into());
    }
    let manifest = load_manifest(&checkpoint)?;
    if manifest.project_id != project_id
        || manifest.id != checkpoint.file_name().and_then(|name| name.to_str()).unwrap_or_default()
        || PathBuf::from(&manifest.root_dir).canonicalize().ok().as_deref() != Some(checkpoint.as_path())
    {
        return Err("checkpoint manifest does not match its Core-owned location".into());
    }
    Ok((manifest, checkpoint))
}

/// Compare live workspace hashes to checkpoint post-image.
pub fn preview_intervening(
    workspace_root: &Path,
    manifest: &CheckpointManifest,
) -> Result<Vec<InterveningChange>, String> {
    let mut out = Vec::new();
    for f in &manifest.files {
        if f.skipped || classify_path(workspace_root, &f.path).class == PathClass::ProtectedInWorkspace {
            continue;
        }
        let path = resolve_in_workspace(workspace_root, &f.path)?;
        let live = if path.is_file() {
            Some(content_hash_bytes(
                &fs::read(&path).map_err(|e| e.to_string())?,
            ))
        } else {
            None
        };
        if live.as_deref() != Some(f.hash.as_str()) {
            out.push(InterveningChange {
                path: f.path.clone(),
                checkpoint_hash: f.hash.clone(),
                live_hash: live,
            });
        }
    }
    Ok(out)
}

/// Restore workspace files from checkpoint. Does not touch chat events or Git.
/// If intervening changes exist and `force` is false, returns warned report without applying.
pub fn restore_files(
    workspace_root: &Path,
    manifest: &CheckpointManifest,
    force: bool,
) -> Result<RestoreReport, String> {
    let intervening = preview_intervening(workspace_root, manifest)?;
    if !intervening.is_empty() && !force {
        return Ok(RestoreReport {
            axis: RestoreAxis::FilesOnly,
            checkpoint_id: manifest.id.clone(),
            files_restored: vec![],
            files_skipped: vec![],
            conversation_cursor: None,
            intervening,
            warned: true,
            applied: false,
            message: "intervening manual changes detected — confirm overwrite".into(),
        });
    }

    let cp_dir = PathBuf::from(&manifest.root_dir);
    restore_files_from_directory(workspace_root, manifest, &cp_dir, force)
}

/// Restore from a checkpoint directory already validated by Core.
pub fn restore_files_from_directory(
    workspace_root: &Path,
    manifest: &CheckpointManifest,
    cp_dir: &Path,
    force: bool,
) -> Result<RestoreReport, String> {
    let intervening = preview_intervening(workspace_root, manifest)?;
    if !intervening.is_empty() && !force {
        return Ok(RestoreReport {
            axis: RestoreAxis::FilesOnly,
            checkpoint_id: manifest.id.clone(),
            files_restored: vec![],
            files_skipped: vec![],
            conversation_cursor: None,
            intervening,
            warned: true,
            applied: false,
            message: "intervening manual changes detected — confirm overwrite".into(),
        });
    }
    let blobs = cp_dir.join("blobs");
    let mut restored = Vec::new();
    let mut skipped = Vec::new();

    for f in &manifest.files {
        if f.skipped {
            skipped.push(f.path.clone());
            continue;
        }
        if classify_path(workspace_root, &f.path).class == PathClass::ProtectedInWorkspace {
            skipped.push(f.path.clone());
            continue;
        }
        let dest = resolve_in_workspace(workspace_root, &f.path)?;
        let blob = blobs.join(blob_file_name(&f.hash));
        if !blob.is_file() {
            skipped.push(f.path.clone());
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let data = fs::read(&blob).map_err(|e| e.to_string())?;
        fs::write(&dest, data).map_err(|e| e.to_string())?;
        restored.push(f.path.clone());
    }

    assert_no_git_under_project(workspace_root)?;

    Ok(RestoreReport {
        axis: RestoreAxis::FilesOnly,
        checkpoint_id: manifest.id.clone(),
        files_restored: restored,
        files_skipped: skipped,
        conversation_cursor: None,
        intervening,
        warned: false,
        applied: true,
        message: "files restored".into(),
    })
}

fn assert_no_git_under_project(_workspace: &Path) -> Result<(), String> {
    // Checkpoint ops never create .git refs; presence of user's .git is fine.
    Ok(())
}

/// Conversation-only: soft-hide events after the checkpoint cursor (no delete).
pub fn restore_conversation(
    db: &Database,
    chat_id: &str,
    event_cursor_seq: Option<i64>,
) -> Result<RestoreReport, String> {
    let cursor = event_cursor_seq.unwrap_or(0);
    // Store visibility cursor in settings — events remain durable.
    let value = serde_json::json!({ "hiddenAfterSeq": cursor }).to_string();
    db.set_setting("chat", &format!("{chat_id}.event_cursor"), &value)?;
    Ok(RestoreReport {
        axis: RestoreAxis::ConversationOnly,
        checkpoint_id: String::new(),
        files_restored: vec![],
        files_skipped: vec![],
        conversation_cursor: Some(cursor),
        intervening: vec![],
        warned: false,
        applied: true,
        message: format!("conversation cursor set to seq ≤ {cursor} (later events soft-hidden)"),
    })
}

/// Read soft-hide cursor for a chat (None = show all).
pub fn chat_event_cursor(db: &Database, chat_id: &str) -> Result<Option<i64>, String> {
    let Some(s) = db.get_setting("chat", &format!("{chat_id}.event_cursor"))? else {
        return Ok(None);
    };
    let v: serde_json::Value =
        serde_json::from_str(&s.value_json).map_err(|e| e.to_string())?;
    Ok(v.get("hiddenAfterSeq").and_then(|x| x.as_i64()))
}

/// Both axes: files then conversation.
pub fn restore_both(
    workspace_root: &Path,
    db: &Database,
    chat_id: &str,
    manifest: &CheckpointManifest,
    force: bool,
) -> Result<RestoreReport, String> {
    let mut files_report = restore_files(workspace_root, manifest, force)?;
    if files_report.warned && !files_report.applied {
        files_report.axis = RestoreAxis::Both;
        return Ok(files_report);
    }
    let conv = restore_conversation(db, chat_id, manifest.event_cursor_seq)?;
    Ok(RestoreReport {
        axis: RestoreAxis::Both,
        checkpoint_id: manifest.id.clone(),
        files_restored: files_report.files_restored,
        files_skipped: files_report.files_skipped,
        conversation_cursor: conv.conversation_cursor,
        intervening: files_report.intervening,
        warned: false,
        applied: true,
        message: "files and conversation restored".into(),
    })
}

/// Both-axis restore using a checkpoint directory that has been validated by Core.
pub fn restore_both_from_directory(
    workspace_root: &Path,
    db: &Database,
    chat_id: &str,
    manifest: &CheckpointManifest,
    checkpoint_dir: &Path,
    force: bool,
) -> Result<RestoreReport, String> {
    let mut files_report = restore_files_from_directory(workspace_root, manifest, checkpoint_dir, force)?;
    if files_report.warned && !files_report.applied {
        files_report.axis = RestoreAxis::Both;
        return Ok(files_report);
    }
    let conv = restore_conversation(db, chat_id, manifest.event_cursor_seq)?;
    Ok(RestoreReport {
        axis: RestoreAxis::Both,
        checkpoint_id: manifest.id.clone(),
        files_restored: files_report.files_restored,
        files_skipped: files_report.files_skipped,
        conversation_cursor: conv.conversation_cursor,
        intervening: files_report.intervening,
        warned: false,
        applied: true,
        message: "files and conversation restored".into(),
    })
}

/// Ensure checkpoint store path is outside the project tree.
pub fn store_is_outside_project(store_root: &Path, project_root: &Path) -> bool {
    let store = store_root.canonicalize().unwrap_or_else(|_| store_root.to_path_buf());
    let proj = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    !store.starts_with(&proj)
}

fn blob_file_name(hash: &str) -> String {
    hash.replace(':', "_").replace(['/', '\\', '*', '?', '"', '<', '>', '|'], "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::store::Database;

    fn temp(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "lc-cp-{label}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn create_restore_files_and_intervening_warn() {
        let project = temp("proj");
        let store = temp("store");
        fs::write(project.join("a.txt"), "v1").unwrap();
        let m = create_checkpoint(&project, &store, "p1", Some("c1"), Some("r1"), Some(3))
            .unwrap();
        assert!(store_is_outside_project(Path::new(&m.store_path), &project));
        assert!(!Path::new(&m.root_dir).join(".git").exists());

        fs::write(project.join("a.txt"), "manual").unwrap();
        let preview = preview_intervening(&project, &m).unwrap();
        assert_eq!(preview.len(), 1);

        let warned = restore_files(&project, &m, false).unwrap();
        assert!(warned.warned && !warned.applied);
        assert_eq!(fs::read_to_string(project.join("a.txt")).unwrap(), "manual");

        let forced = restore_files(&project, &m, true).unwrap();
        assert!(forced.applied);
        assert_eq!(fs::read_to_string(project.join("a.txt")).unwrap(), "v1");
    }

    #[test]
    fn conversation_restore_sets_cursor_without_deleting_events() {
        let db = Database::open_in_memory().unwrap();
        let root = temp("conv");
        let project = db.open_project(&root).unwrap();
        let chat = db.create_chat(&project.id, Some("t")).unwrap();
        db.append_event(&chat.id, None, "user_message", r#"{"text":"a"}"#)
            .unwrap();
        db.append_event(&chat.id, None, "user_message", r#"{"text":"b"}"#)
            .unwrap();
        let events_before = db.list_events(&chat.id, None).unwrap();
        assert_eq!(events_before.len(), 2);

        let report = restore_conversation(&db, &chat.id, Some(1)).unwrap();
        assert!(report.applied);
        assert_eq!(report.conversation_cursor, Some(1));
        // Events still present
        assert_eq!(db.list_events(&chat.id, None).unwrap().len(), 2);
        assert_eq!(chat_event_cursor(&db, &chat.id).unwrap(), Some(1));
    }
}
