//! Surfaces: files explorer, diffs, checkpoints.

pub mod checkpoint;
pub mod conflict;
pub mod diff;
pub mod editor;
pub mod explorer;

pub use checkpoint::{
    create_checkpoint, preview_intervening, restore_both, restore_conversation, restore_files,
    CheckpointManifest, RestoreAxis, RestoreReport,
};
pub use conflict::{
    evaluate_agent_patch_preflight, ConflictKind, ConflictOutcome, DirtyBufferState,
};
pub use diff::{project_diffs, Attribution, BufferSnapshot, DiffChange, DiffModel, DiffScope};
pub use editor::{open_text_file, save_text_file, OpenFileDto, SaveFileResult};
pub use explorer::{is_noise_name, list_project_tree, open_path_external, open_url_external, TreeEntry};
