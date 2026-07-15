//! Domain types for LoopCode durable state.
//! Session is not durable — it is rebuilt from SQLite on launch.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Current SQLite schema version applied via `PRAGMA user_version`.
pub const SCHEMA_VERSION: u32 = 2;

/// Placeholder title used when a chat is created without an explicit title.
/// `cockpit_send` auto-titles a chat from the first user message while the
/// title is still this sentinel, so the creation default and the auto-title
/// guard share one source of truth.
pub const DEFAULT_CHAT_TITLE: &str = "New chat";

/// Minimal health report exposed to tests and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
    pub name: String,
    pub version: String,
    /// Describes the WebView privilege posture.
    pub shell: String,
    /// Present when Core has an open DB.
    pub schema_version: Option<u32>,
    pub db_path: Option<String>,
    pub journal_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub root_path: String,
    pub resolved_path: String,
    pub display_path: String,
    pub orphaned: bool,
    pub last_active_chat_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Derive a short sidebar title from the user's first message.
///
/// Uses the first line only, takes the first [`MAX_WORDS`] words, and caps the
/// result at [`MAX_CHARS`] characters truncating on a word boundary. Whitespace
/// is collapsed. Returns [`DEFAULT_CHAT_TITLE`] when the message has no usable
/// text (e.g. blank or whitespace-only).
pub fn derive_chat_title(text: &str) -> String {
    const MAX_WORDS: usize = 8;
    const MAX_CHARS: usize = 60;

    let first_line = text.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        return DEFAULT_CHAT_TITLE.to_string();
    }
    let words: Vec<&str> = first_line.split_whitespace().take(MAX_WORDS).collect();
    let mut title = words.join(" ");
    if title.chars().count() > MAX_CHARS {
        let truncated: String = title.chars().take(MAX_CHARS).collect();
        title = match truncated.rfind(' ') {
            Some(idx) if idx > 0 => truncated[..idx].to_string(),
            _ => truncated,
        };
    }
    if title.is_empty() {
        DEFAULT_CHAT_TITLE.to_string()
    } else {
        title
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: String,
    pub chat_id: String,
    pub mode: String,
    pub model: Option<String>,
    pub adapter: Option<String>,
    pub policy_version: Option<String>,
    pub status: RunStatus,
    pub predecessor_run_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

/// Durable run lifecycle (host state machine).
///
/// Active path: queued → preparing → model_active ↔ tool_active | approval_waiting
/// → cancelling → cancelled, or suspended on crash/uncertain side effects,
/// or completed | failed terminal outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    Preparing,
    ModelActive,
    ToolActive,
    ApprovalWaiting,
    Cancelling,
    /// In-flight run that did not reach a terminal durable state (restart/crash).
    Suspended,
    Completed,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Preparing => "preparing",
            Self::ModelActive => "model_active",
            Self::ToolActive => "tool_active",
            Self::ApprovalWaiting => "approval_waiting",
            Self::Cancelling => "cancelling",
            Self::Suspended => "suspended",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "queued" | "pending" => Some(Self::Queued),
            "preparing" => Some(Self::Preparing),
            "model_active" | "running" => Some(Self::ModelActive),
            "tool_active" => Some(Self::ToolActive),
            "approval_waiting" => Some(Self::ApprovalWaiting),
            "cancelling" => Some(Self::Cancelling),
            "suspended" => Some(Self::Suspended),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Non-terminal statuses that represent in-flight host work (not suspended).
    pub fn is_active(self) -> bool {
        matches!(
            self,
            Self::Queued
                | Self::Preparing
                | Self::ModelActive
                | Self::ToolActive
                | Self::ApprovalWaiting
                | Self::Cancelling
        )
    }

    /// SQL `IN (...)` list for statuses that must suspend on restart.
    pub fn inflight_sql_list() -> &'static str {
        "'queued','pending','preparing','model_active','running','tool_active','approval_waiting','cancelling'"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub chat_id: String,
    pub run_id: Option<String>,
    pub seq: i64,
    pub kind: String,
    pub payload_json: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub project_id: String,
    pub chat_id: Option<String>,
    pub run_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub body_inline: Option<String>,
    pub body_path: Option<String>,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRecord {
    pub id: String,
    pub run_id: Option<String>,
    pub chat_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cost_micros: Option<i64>,
    pub elapsed_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    pub scope: String,
    pub key: String,
    pub value_json: String,
    pub updated_at: DateTime<Utc>,
}

/// Reference to a secret in the OS store — never holds secret material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRef {
    pub id: String,
    pub kind: String,
    pub label: String,
    /// Key name in the OS secret store (not the secret value).
    pub store_key: String,
    pub provider: Option<String>,
    pub project_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundle {
    pub format: String,
    pub chat_id: String,
    pub project_id: String,
    pub redacted: bool,
    pub transcript: Vec<ExportEvent>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEvent {
    pub seq: i64,
    pub kind: String,
    pub payload_json: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditRecord {
    pub id: String,
    pub kind: String,
    pub actor: String,
    pub run_id: Option<String>,
    pub chat_id: Option<String>,
    pub project_id: Option<String>,
    pub payload_json: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustRecord {
    pub id: String,
    pub kind: String,
    pub subject_key: String,
    pub content_hash: String,
    pub project_id: Option<String>,
    pub trusted_at: String,
}

#[cfg(test)]
mod tests {
    use super::{derive_chat_title, DEFAULT_CHAT_TITLE};

    #[test]
    fn derive_title_from_simple_prompt() {
        // Eight words, 38 chars — under both caps, so it passes through whole.
        assert_eq!(
            derive_chat_title("Fix the login bug on the dashboard page"),
            "Fix the login bug on the dashboard page"
        );
    }

    #[test]
    fn derive_title_truncates_to_eight_words() {
        assert_eq!(
            derive_chat_title("one two three four five six seven eight nine"),
            "one two three four five six seven eight"
        );
    }

    #[test]
    fn derive_title_uses_first_line_only() {
        assert_eq!(
            derive_chat_title("First line summary\nsecond line detail\nthird"),
            "First line summary"
        );
    }

    #[test]
    fn derive_title_caps_long_words_on_word_boundary() {
        // Six words but over 60 chars → truncate at the last word boundary ≤60.
        let long = "abcdefghij abcdefghij abcdefghij abcdefghij abcdefghij abcdefghij extra";
        let derived = derive_chat_title(long);
        assert!(derived.chars().count() <= 60);
        assert!(!derived.contains("extra"));
        // Should end at a complete word, not mid-word.
        assert!(!derived.ends_with(' '));
    }

    #[test]
    fn derive_title_collapses_whitespace() {
        assert_eq!(
            derive_chat_title("  too\t   much   space  "),
            "too much space"
        );
    }

    #[test]
    fn derive_title_blank_falls_back_to_default() {
        assert_eq!(derive_chat_title(""), DEFAULT_CHAT_TITLE);
        assert_eq!(derive_chat_title("   \n  \t "), DEFAULT_CHAT_TITLE);
    }

    #[test]
    fn derive_title_single_long_token_is_hard_capped() {
        // One very long word with no spaces → hard cap at MAX_CHARS, no panic.
        let derived = derive_chat_title(&"x".repeat(120));
        assert_eq!(derived.chars().count(), 60);
    }
}
