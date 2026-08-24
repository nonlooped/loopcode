mod app_commands;
mod broker;
mod diagnostics;
mod git;
mod persistence;
mod project_files;
mod terminal;

#[cfg(windows)]
use app_commands::configure_native_window;
use app_commands::{
    export_diagnostics, initial_working_directory, load_workspace, pick_folder,
    provider_auth_status, provider_version, record_diagnostic, save_workspace,
};
use broker::{
    Broker, FrontendGeneration, launch_harness, register_frontend, send_rpc, stop_all_harnesses,
    stop_harness,
};
use diagnostics::Diagnostics;
use git::{
    create_git_worktree, get_git_branch, get_git_file_diff, list_git_branches, list_git_changes,
    switch_git_branch,
};
use project_files::{
    ProjectFileWatchers, list_composer_completions, open_project_path, read_project_directory,
    read_project_file, reveal_project_path, start_project_file_watcher, stop_project_file_watcher,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::Manager;
use terminal::{
    TerminalManager, resize_terminal, start_terminal, stop_all_terminals, stop_terminal,
    stop_terminal_for_thread, write_terminal,
};

pub fn run() {
    let shutdown_started = Arc::new(AtomicBool::new(false));
    let shutdown_completed = Arc::new(AtomicBool::new(false));
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Broker::default())
        .manage(FrontendGeneration::default())
        .manage(ProjectFileWatchers::default())
        .manage(TerminalManager::default())
        .setup(|app| {
            #[cfg(windows)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    configure_native_window(&window)?;
                }
            }

            let log_dir = app
                .path()
                .app_log_dir()
                .map_err(|error| format!("Could not resolve the log directory: {error}"))?;
            app.manage(Diagnostics::new(log_dir));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            launch_harness,
            register_frontend,
            send_rpc,
            stop_harness,
            stop_all_harnesses,
            initial_working_directory,
            load_workspace,
            save_workspace,
            record_diagnostic,
            export_diagnostics,
            pick_folder,
            get_git_branch,
            list_git_branches,
            list_git_changes,
            get_git_file_diff,
            switch_git_branch,
            create_git_worktree,
            provider_version,
            provider_auth_status,
            list_composer_completions,
            read_project_directory,
            read_project_file,
            open_project_path,
            reveal_project_path,
            start_project_file_watcher,
            stop_project_file_watcher,
            start_terminal,
            write_terminal,
            resize_terminal,
            stop_terminal,
            stop_terminal_for_thread,
            stop_all_terminals,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build LoopCode");

    app.run(move |app, event| {
        if let tauri::RunEvent::ExitRequested { api, code, .. } = event
            && !shutdown_completed.load(Ordering::Acquire)
        {
            api.prevent_exit();
            if !shutdown_started.swap(true, Ordering::AcqRel) {
                let app = app.clone();
                let shutdown_completed = Arc::clone(&shutdown_completed);
                tauri::async_runtime::spawn(async move {
                    let broker = app.state::<Broker>();
                    let terminals = app.state::<TerminalManager>();
                    let diagnostics = app.state::<Diagnostics>();
                    let _ = broker.shutdown(&diagnostics).await;
                    let _ = terminals.shutdown(&diagnostics).await;
                    shutdown_completed.store(true, Ordering::Release);
                    app.exit(code.unwrap_or(0));
                });
            }
        }
    });
}
