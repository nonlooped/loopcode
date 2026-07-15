//! Host-side mode contracts (Ask / Plan / Build / Debug).
//! Prompt wording and model output cannot change these boundaries.

use serde::{Deserialize, Serialize};

/// User-selected run mode. Immutable for the lifetime of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Non-mutating inspection only; produces an answer.
    Ask,
    /// Same mutation boundary as Ask; produces a plan artifact.
    Plan,
    /// May invoke mutation-capable tools (subject to approval policy).
    Build,
    /// Inspection and approved diagnostics; no source-mutating tools.
    Debug,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Plan => "plan",
            Self::Build => "build",
            Self::Debug => "debug",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ask" => Some(Self::Ask),
            "plan" => Some(Self::Plan),
            "build" => Some(Self::Build),
            "debug" => Some(Self::Debug),
            _ => None,
        }
    }

    /// Whether this mode may execute tools marked as mutating (source edits / writes).
    pub fn allows_source_mutation(self) -> bool {
        matches!(self, Self::Build)
    }

    /// Whether this mode may execute tools marked as diagnostic (non-source side effects).
    pub fn allows_diagnostics(self) -> bool {
        matches!(self, Self::Build | Self::Debug)
    }

    /// Whether this mode may execute read-only inspection tools.
    pub fn allows_inspection(self) -> bool {
        true
    }
}

/// Classification of a tool's side-effect profile (mock + future real tools).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolEffect {
    /// Pure inspection / read — allowed in all modes.
    ReadOnly,
    /// Diagnostic that does not edit project source (e.g. collect logs).
    Diagnostic,
    /// Mutates project source or workspace files.
    Mutating,
}

/// Evaluate whether a tool with the given effect may execute under `mode`.
///
/// This is the shipped policy function tests must call — do not re-implement in tests.
pub fn mode_allows_tool_effect(mode: Mode, effect: ToolEffect) -> bool {
    match effect {
        ToolEffect::ReadOnly => mode.allows_inspection(),
        ToolEffect::Diagnostic => mode.allows_diagnostics(),
        ToolEffect::Mutating => mode.allows_source_mutation(),
    }
}

/// Stable reason code when a tool is rejected by mode policy.
pub fn mode_rejection_reason(mode: Mode, effect: ToolEffect) -> String {
    format!(
        "mode_policy_denied: mode={} does not allow tool_effect={}",
        mode.as_str(),
        match effect {
            ToolEffect::ReadOnly => "read_only",
            ToolEffect::Diagnostic => "diagnostic",
            ToolEffect::Mutating => "mutating",
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_only_allows_mutating() {
        assert!(!mode_allows_tool_effect(Mode::Ask, ToolEffect::Mutating));
        assert!(!mode_allows_tool_effect(Mode::Plan, ToolEffect::Mutating));
        assert!(mode_allows_tool_effect(Mode::Build, ToolEffect::Mutating));
        assert!(!mode_allows_tool_effect(Mode::Debug, ToolEffect::Mutating));
    }

    #[test]
    fn debug_allows_diagnostic_not_mutating() {
        assert!(mode_allows_tool_effect(Mode::Debug, ToolEffect::Diagnostic));
        assert!(mode_allows_tool_effect(Mode::Debug, ToolEffect::ReadOnly));
        assert!(!mode_allows_tool_effect(Mode::Debug, ToolEffect::Mutating));
    }
}
