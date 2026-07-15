//! Bounded local redacted logs (always on, no network).

use crate::security::{redact_json_value, redact_text};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalLogAppend {
    pub path: String,
    pub bytes_written: usize,
    pub redacted: bool,
}

/// Append a redacted log line to `{logs_dir}/loopcode.log`, rotating if oversized.
pub fn append_local_log(
    logs_dir: &Path,
    level: &str,
    category: &str,
    message: &str,
    detail: Option<&Value>,
) -> Result<LocalLogAppend, String> {
    fs::create_dir_all(logs_dir).map_err(|e| e.to_string())?;
    let path = logs_dir.join("loopcode.log");
    rotate_if_needed(&path)?;

    let detail_redacted = detail.map(redact_json_value);
    let line = serde_json::json!({
        "ts": Utc::now().to_rfc3339(),
        "level": redact_text(level),
        "category": redact_text(category),
        "message": redact_text(message),
        "detail": detail_redacted,
    });
    let mut text = line.to_string();
    text.push('\n');

    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    f.write_all(text.as_bytes()).map_err(|e| e.to_string())?;

    Ok(LocalLogAppend {
        path: path.display().to_string(),
        bytes_written: text.len(),
        redacted: true,
    })
}

fn rotate_if_needed(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Ok(());
    }
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() < MAX_LOG_BYTES {
        return Ok(());
    }
    let rotated = path.with_extension("log.1");
    let _ = fs::remove_file(&rotated);
    fs::rename(path, &rotated).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn default_logs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("logs")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn append_redacts_secret_shaped_detail() {
        let dir = std::env::temp_dir().join(format!(
            "lc-log-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let r = append_local_log(
            &dir,
            "error",
            "provider",
            "probe failed",
            Some(&json!({"apiKey": "sk-secret-value-here", "msg": "nope"})),
        )
        .unwrap();
        let body = fs::read_to_string(&r.path).unwrap();
        assert!(!body.contains("sk-secret-value-here"));
        assert!(body.contains("probe failed"));
    }
}
