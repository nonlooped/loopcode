//! Codex's ACP adapter keeps plan limits to itself (it answers the app-server's
//! `account/rateLimits/updated` notification with `null`), so they never reach an ACP client.
//! Codex does persist them verbatim in its own session rollout files, which is the same data
//! its `/status` command prints, in a structured form we can read without running anything.

use serde_json::Value;
use std::path::{Path, PathBuf};
use tauri::Manager;

/// Rollouts are grouped `sessions/YYYY/MM/DD`, so the newest limits sit in a recent day's files.
const MAX_SCANNED_DAYS: usize = 7;
const MAX_SCANNED_FILES: usize = 24;
/// A rollout that outgrows this is a transcript we have no reason to page through.
const MAX_ROLLOUT_BYTES: u64 = 32 * 1024 * 1024;

#[tauri::command]
pub(crate) async fn codex_rate_limits(app: tauri::AppHandle) -> Result<Option<Value>, String> {
    let home = codex_home(&app)?;
    tauri::async_runtime::spawn_blocking(move || latest_rate_limits(&home.join("sessions")))
        .await
        .map_err(|error| format!("Could not join the Codex usage task: {error}"))
}

fn codex_home(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Some(home) = std::env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(home));
    }
    app.path()
        .home_dir()
        .map(|home| home.join(".codex"))
        .map_err(|error| format!("Could not resolve the home directory: {error}"))
}

/// Returns `{ capturedAt, rateLimits }` from the most recent rollout entry that carries limits.
fn latest_rate_limits(sessions: &Path) -> Option<Value> {
    for file in recent_rollouts(sessions) {
        if let Some(limits) = last_rate_limits(&file) {
            return Some(limits);
        }
    }
    None
}

/// Descends the newest `YYYY/MM/DD` directories rather than walking every session ever recorded.
fn recent_rollouts(sessions: &Path) -> Vec<PathBuf> {
    let mut days = Vec::new();
    for year in newest_first(sessions) {
        for month in newest_first(&year) {
            for day in newest_first(&month) {
                days.push(day);
                if days.len() >= MAX_SCANNED_DAYS {
                    break;
                }
            }
            if days.len() >= MAX_SCANNED_DAYS {
                break;
            }
        }
        if days.len() >= MAX_SCANNED_DAYS {
            break;
        }
    }

    let mut files = Vec::new();
    for day in days {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&day)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "jsonl")
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("rollout-"))
            })
            .collect();
        // Rollout names embed an ISO timestamp, so a reverse sort is newest-first.
        entries.sort_unstable();
        entries.reverse();
        files.extend(entries);
        if files.len() >= MAX_SCANNED_FILES {
            break;
        }
    }
    files.truncate(MAX_SCANNED_FILES);
    files
}

fn newest_first(directory: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect();
    entries.sort_unstable();
    entries.reverse();
    entries
}

fn last_rate_limits(rollout: &Path) -> Option<Value> {
    if std::fs::metadata(rollout).ok()?.len() > MAX_ROLLOUT_BYTES {
        return None;
    }
    let contents = std::fs::read_to_string(rollout).ok()?;
    contents.lines().rev().find_map(|line| {
        // Cheap reject: most rollout lines are transcript content, not token accounting.
        if !line.contains("\"rate_limits\"") {
            return None;
        }
        let entry: Value = serde_json::from_str(line).ok()?;
        let payload = entry.get("payload")?;
        if payload.get("type")?.as_str()? != "token_count" {
            return None;
        }
        let limits = payload.get("rate_limits")?;
        if limits.is_null() {
            return None;
        }
        Some(serde_json::json!({
            "capturedAt": entry.get("timestamp").cloned().unwrap_or(Value::Null),
            "rateLimits": limits.clone(),
        }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    #[test]
    fn reads_the_newest_rollout_entry_that_carries_limits() {
        let root = tempfile::tempdir().unwrap();
        let sessions = root.path().join("sessions");
        write(
            &sessions.join("2026/08/28/rollout-2026-08-28T10-00-00-old.jsonl"),
            "{\"timestamp\":\"2026-08-28T10:00:00.000Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"rate_limits\":{\"limit_id\":\"stale\"}}}\n",
        );
        write(
            &sessions.join("2026/08/29/rollout-2026-08-29T09-00-00-newer.jsonl"),
            concat!(
                "{\"timestamp\":\"2026-08-29T09:00:00.000Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"rate_limits\":{\"limit_id\":\"first\"}}}\n",
                "{\"timestamp\":\"2026-08-29T09:05:00.000Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"agent_message\",\"text\":\"no limits here\"}}\n",
                "{\"timestamp\":\"2026-08-29T09:10:00.000Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"rate_limits\":{\"limit_id\":\"latest\",\"primary\":{\"used_percent\":27.0,\"window_minutes\":10080,\"resets_at\":1788453754}}}}\n",
            ),
        );

        let found = latest_rate_limits(&sessions).expect("rate limits");
        assert_eq!(found["capturedAt"], "2026-08-29T09:10:00.000Z");
        assert_eq!(found["rateLimits"]["limit_id"], "latest");
        assert_eq!(found["rateLimits"]["primary"]["used_percent"], 27.0);
    }

    #[test]
    fn token_counts_without_limits_are_skipped() {
        let root = tempfile::tempdir().unwrap();
        let sessions = root.path().join("sessions");
        write(
            &sessions.join("2026/08/29/rollout-2026-08-29T09-00-00-empty.jsonl"),
            "{\"timestamp\":\"2026-08-29T09:00:00.000Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"rate_limits\":null}}\n",
        );

        assert!(latest_rate_limits(&sessions).is_none());
    }

    #[test]
    fn a_missing_sessions_directory_reports_no_limits() {
        let root = tempfile::tempdir().unwrap();
        assert!(latest_rate_limits(&root.path().join("sessions")).is_none());
    }
}
