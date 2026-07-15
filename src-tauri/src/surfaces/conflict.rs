//! Manual/agent conflict decisions for apply_patch + dirty editor buffers.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    /// On-disk hash matches expected pre-image; safe to apply.
    None,
    /// Disk diverged from agent's expected pre-image.
    InterveningManualChange,
    /// Editor has unsaved dirty buffer for the path.
    DirtyBuffer,
    /// Both intervening disk change and dirty buffer.
    DirtyAndIntervening,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictOutcome {
    Apply,
    KeepDisk,
    OverwriteWithAgent,
    DiscardBufferAndApply,
    SaveFirst,
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirtyBufferState {
    pub path: String,
    pub dirty: bool,
    pub buffer_hash: Option<String>,
}

/// Evaluate preflight before applying an agent workspace patch.
///
/// Fail-closed when `kind != None` unless the caller supplies an explicit
/// user decision via [`resolve_conflict`].
pub fn evaluate_agent_patch_preflight(
    disk_hash: Option<&str>,
    expected_hash: Option<&str>,
    dirty: Option<&DirtyBufferState>,
) -> ConflictKind {
    let intervening = match (disk_hash, expected_hash) {
        (Some(disk), Some(exp)) if disk != exp => true,
        (None, Some(exp)) if exp != "absent" && exp != "new" && !exp.is_empty() => true,
        _ => false,
    };
    let is_dirty = dirty.map(|d| d.dirty).unwrap_or(false);
    match (intervening, is_dirty) {
        (false, false) => ConflictKind::None,
        (true, false) => ConflictKind::InterveningManualChange,
        (false, true) => ConflictKind::DirtyBuffer,
        (true, true) => ConflictKind::DirtyAndIntervening,
    }
}

/// Map a user choice to whether the agent patch may proceed.
pub fn resolve_conflict(kind: ConflictKind, choice: ConflictOutcome) -> Result<(), String> {
    if kind == ConflictKind::None {
        return Ok(());
    }
    match (kind, choice) {
        (_, ConflictOutcome::Cancel) => Err("conflict: cancelled by user".into()),
        (ConflictKind::InterveningManualChange, ConflictOutcome::KeepDisk) => {
            Err("conflict: keep disk — agent patch not applied".into())
        }
        (ConflictKind::InterveningManualChange, ConflictOutcome::OverwriteWithAgent)
        | (ConflictKind::DirtyBuffer, ConflictOutcome::DiscardBufferAndApply)
        | (ConflictKind::DirtyAndIntervening, ConflictOutcome::OverwriteWithAgent)
        | (ConflictKind::DirtyAndIntervening, ConflictOutcome::DiscardBufferAndApply)
        | (_, ConflictOutcome::Apply) => Ok(()),
        (ConflictKind::DirtyBuffer, ConflictOutcome::SaveFirst)
        | (ConflictKind::DirtyAndIntervening, ConflictOutcome::SaveFirst) => {
            Err("conflict: save buffer first before agent patch".into())
        }
        _ => Err(format!(
            "conflict: outcome {choice:?} not valid for kind {kind:?}"
        )),
    }
}

/// High-level decision used by Core/UI: fail-closed unless user explicitly allows.
pub fn decide_patch_allowed(
    kind: ConflictKind,
    user_choice: Option<ConflictOutcome>,
) -> Result<(), String> {
    match kind {
        ConflictKind::None => Ok(()),
        other => match user_choice {
            None => Err(format!(
                "conflict: {other:?} — fail-closed without user choice"
            )),
            Some(choice) => resolve_conflict(other, choice),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervening_and_dirty_matrix() {
        assert_eq!(
            evaluate_agent_patch_preflight(Some("a:1"), Some("a:1"), None),
            ConflictKind::None
        );
        assert_eq!(
            evaluate_agent_patch_preflight(Some("b:1"), Some("a:1"), None),
            ConflictKind::InterveningManualChange
        );
        let dirty = DirtyBufferState {
            path: "x".into(),
            dirty: true,
            buffer_hash: Some("c:1".into()),
        };
        assert_eq!(
            evaluate_agent_patch_preflight(Some("a:1"), Some("a:1"), Some(&dirty)),
            ConflictKind::DirtyBuffer
        );
        assert_eq!(
            evaluate_agent_patch_preflight(Some("b:1"), Some("a:1"), Some(&dirty)),
            ConflictKind::DirtyAndIntervening
        );
        assert!(decide_patch_allowed(ConflictKind::InterveningManualChange, None).is_err());
        assert!(decide_patch_allowed(
            ConflictKind::InterveningManualChange,
            Some(ConflictOutcome::OverwriteWithAgent)
        )
        .is_ok());
    }
}
