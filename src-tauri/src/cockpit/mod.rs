//! Agent cockpit projection (timeline, keyboard, chrome).

pub mod keyboard;
pub mod timeline;

pub use keyboard::{
    default_keyboard_map, required_expert_commands, CockpitCommand, KeyBinding, KeyboardRegistry,
};
pub use timeline::{project_timeline, TimelineItem, TimelineItemKind, TimelineProjection, TimelineViewState};
