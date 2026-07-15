//! Expert keyboard map — binding table + registry for tests and UI.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stable command ids used by the cockpit UI and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CockpitCommand {
    NewChat,
    FocusComposer,
    ToggleLeft,
    ToggleRight,
    TabFiles,
    TabDiffs,
    ModeCycle,
    ModelPicker,
    StopRun,
    ApproveOnce,
    DenyApproval,
}

impl CockpitCommand {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NewChat => "new_chat",
            Self::FocusComposer => "focus_composer",
            Self::ToggleLeft => "toggle_left",
            Self::ToggleRight => "toggle_right",
            Self::TabFiles => "tab_files",
            Self::TabDiffs => "tab_diffs",
            Self::ModeCycle => "mode_cycle",
            Self::ModelPicker => "model_picker",
            Self::StopRun => "stop_run",
            Self::ApproveOnce => "approve_once",
            Self::DenyApproval => "deny_approval",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyBinding {
    pub command: CockpitCommand,
    /// Display label e.g. "Ctrl+K"
    pub chord: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: String,
}

/// Shipped default keyboard map (Win/Linux primary; macOS uses same chord labels in UI copy).
pub fn default_keyboard_map() -> Vec<KeyBinding> {
    vec![
        bind(CockpitCommand::NewChat, "Ctrl+N", true, false, false, "n"),
        bind(CockpitCommand::FocusComposer, "Ctrl+L", true, false, false, "l"),
        bind(CockpitCommand::ToggleLeft, "Ctrl+B", true, false, false, "b"),
        bind(CockpitCommand::ToggleRight, "Ctrl+Alt+B", true, true, false, "b"),
        bind(CockpitCommand::TabFiles, "Ctrl+1", true, false, false, "1"),
        bind(CockpitCommand::TabDiffs, "Ctrl+2", true, false, false, "2"),
        bind(CockpitCommand::ModeCycle, "Ctrl+Shift+M", true, false, true, "m"),
        bind(CockpitCommand::ModelPicker, "Ctrl+Shift+.", true, false, true, "."),
        bind(CockpitCommand::StopRun, "Escape", false, false, false, "escape"),
        bind(CockpitCommand::ApproveOnce, "Alt+A", false, true, false, "a"),
        bind(CockpitCommand::DenyApproval, "Alt+D", false, true, false, "d"),
    ]
}

fn bind(
    command: CockpitCommand,
    chord: &str,
    ctrl: bool,
    alt: bool,
    shift: bool,
    key: &str,
) -> KeyBinding {
    KeyBinding {
        command,
        chord: chord.into(),
        ctrl,
        alt,
        shift,
        key: key.into(),
    }
}

/// Registry that maps key events to commands (pure; no DOM).
#[derive(Debug, Default)]
pub struct KeyboardRegistry {
    by_chord: HashMap<String, CockpitCommand>,
}

impl KeyboardRegistry {
    pub fn with_defaults() -> Self {
        let mut reg = Self::default();
        for b in default_keyboard_map() {
            reg.register(&b);
        }
        reg
    }

    pub fn register(&mut self, binding: &KeyBinding) {
        let id = chord_id(binding.ctrl, binding.alt, binding.shift, &binding.key);
        self.by_chord.insert(id, binding.command);
    }

    pub fn resolve(&self, ctrl: bool, alt: bool, shift: bool, key: &str) -> Option<CockpitCommand> {
        let id = chord_id(ctrl, alt, shift, &key.to_ascii_lowercase());
        self.by_chord.get(&id).copied()
    }

    pub fn registered_commands(&self) -> Vec<CockpitCommand> {
        let mut v: Vec<_> = self.by_chord.values().copied().collect();
        v.sort_by_key(|c| c.as_str());
        v.dedup();
        v
    }

    pub fn len(&self) -> usize {
        self.by_chord.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_chord.is_empty()
    }
}

fn chord_id(ctrl: bool, alt: bool, shift: bool, key: &str) -> String {
    format!(
        "c{}-a{}-s{}-{}",
        ctrl as u8,
        alt as u8,
        shift as u8,
        key.to_ascii_lowercase()
    )
}

/// Required expert bindings from the closed prototype (minimum set).
pub fn required_expert_commands() -> &'static [CockpitCommand] {
    &[
        CockpitCommand::NewChat,
        CockpitCommand::FocusComposer,
        CockpitCommand::ToggleLeft,
        CockpitCommand::ToggleRight,
        CockpitCommand::ModeCycle,
        CockpitCommand::StopRun,
        CockpitCommand::ApproveOnce,
        CockpitCommand::DenyApproval,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_register_required_expert_bindings() {
        let reg = KeyboardRegistry::with_defaults();
        assert!(reg.len() >= 7);
        for cmd in required_expert_commands() {
            assert!(
                reg.registered_commands().contains(cmd),
                "missing {:?}",
                cmd
            );
        }
        assert_eq!(
            reg.resolve(true, false, false, "n"),
            Some(CockpitCommand::NewChat)
        );
        assert_eq!(
            reg.resolve(false, false, false, "escape"),
            Some(CockpitCommand::StopRun)
        );
    }
}
