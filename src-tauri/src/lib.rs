//! LoopCode Rust core.
//!

pub mod a11y;
pub mod cockpit;
mod commands;
pub mod db;
pub mod domain;
pub mod extensibility;
pub mod providers;
pub mod reliability;
pub mod runtime;
pub mod security;
pub mod surfaces;
pub mod tools;

pub use cockpit::{project_timeline, KeyboardRegistry, TimelineProjection, TimelineViewState};
pub use domain::{HealthStatus, RunStatus, SCHEMA_VERSION};
pub use providers::{hero_provider_ids, load_bundled_catalog, list_hero_cards};
pub use runtime::{
    mode_allows_tool_effect, AgentRuntime, Mode, MockToolRunner, ToolEffect, ToolProposal,
};
pub use security::{
    approval_decision, classify_path, ActionClass, ApprovalDecision, PathClass,
    SECURITY_POLICY_VERSION,
};

use commands::AppState;
use db::store::{default_db_path, Database};
use std::sync::Mutex;

/// Application identity for the shell (used by smoke tests and UI).
pub const APP_NAME: &str = "LoopCode";
/// Semantic version of this package.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns a stable health payload (DB fields filled by the `health` command).
pub fn health_status() -> HealthStatus {
    HealthStatus {
        name: APP_NAME.to_string(),
        version: APP_VERSION.to_string(),
        shell: "capability-locked".to_string(),
        schema_version: None,
        db_path: None,
        journal_mode: None,
    }
}

/// Open the application database at the default path, or `LOOPCODE_DB_PATH` if set.
pub fn open_app_database() -> Result<Database, String> {
    let path = std::env::var("LOOPCODE_DB_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or(default_db_path()?);
    let db = Database::open(&path)?;
    // Restart reconciliation: inflight runs become suspended (never guessed completed).
    runtime::AgentRuntime::reconcile_after_restart(&db)?;
    db.refresh_orphan_flags()?;
    Ok(db)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Single-instance: second process should exit (focus existing is OS/window layer).
    let data = db::store::default_data_dir().unwrap_or_else(|_| {
        std::env::temp_dir().join("loopcode-data")
    });
    let lock_path = a11y::single_instance::default_lock_path(&data);
    let (instance, _instance_guard) =
        a11y::acquire_single_instance(&lock_path).unwrap_or((
            a11y::SingleInstanceDecision::Primary,
            None,
        ));
    if matches!(
        instance,
        a11y::SingleInstanceDecision::SecondaryAlreadyRunning
    ) {
        // Best-effort: leave lock alone; exit without opening a second Core writer.
        eprintln!("LoopCode is already running (single-instance).");
        return;
    }

    let db = open_app_database().expect("failed to open LoopCode database");
    // Persist API keys only through the OS credential manager. If it is
    // unavailable, the store is deliberately memory-only rather than plaintext.
    let secrets = security::secrets::default_secret_store(&data);
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            db: Mutex::new(db),
            runtime: Mutex::new(AgentRuntime::new()),
            secrets,
            run_cancel_flags: Mutex::new(std::collections::HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::health,
            commands::open_project,
            commands::rebind_project,
            commands::list_projects,
            commands::delete_project_record,
            commands::create_chat,
            commands::list_chats,
            commands::get_chat,
            commands::update_chat,
            commands::delete_chat,
            commands::runtime_cancel_run,
            commands::onboarding_list_heroes,
            commands::onboarding_list_catalog,
            commands::onboarding_connect_provider,
            commands::onboarding_create_custom_profile,
            commands::onboarding_get_ready_provider,
            commands::cockpit_set_provider_selection,
            commands::cockpit_timeline,
            commands::cockpit_grant_approval,
            commands::cockpit_keyboard_map,
            commands::cockpit_send,
            commands::surfaces_list_tree,
            commands::surfaces_open_external,
            commands::open_external_url,
            commands::surfaces_project_diffs,
            commands::surfaces_create_checkpoint,
            commands::ext_list_skills,
            commands::ext_load_skill,
            commands::ext_mcp_register,
            commands::ext_mcp_list,
            commands::ext_mcp_set_enabled,
            commands::ext_mcp_remove,
            commands::ext_mcp_grant_trust,
            commands::reliability_integrity_check,
            commands::reliability_backup_db,
            commands::reliability_update_check,
            commands::reliability_settings,
            commands::a11y_focus_order,
            commands::a11y_apply_theme,
            commands::a11y_single_instance_decision,
            commands::a11y_build_approval_notification,
            commands::a11y_menu_actions,
            commands::clipboard_write_text,
            commands::clipboard_read_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running LoopCode");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_status_reports_loopcode_shell() {
        let status = health_status();
        assert_eq!(status.name, "LoopCode");
        assert_eq!(status.shell, "capability-locked");
        assert!(!status.version.is_empty());
    }

    #[test]
    fn health_status_version_matches_cargo_package() {
        let status = health_status();
        assert_eq!(status.version, env!("CARGO_PKG_VERSION"));
    }
}
