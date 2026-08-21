mod broker;
mod diagnostics;
mod persistence;
mod project_files;
mod terminal;

use broker::{
    Broker, launch_harness, register_frontend, send_rpc, stop_all_harnesses, stop_harness,
};
use diagnostics::Diagnostics;
use project_files::{
    ProjectFileWatchers, list_composer_completions, open_project_file, open_project_path,
    read_project_directory, read_project_file, reveal_project_path, start_project_file_watcher,
    stop_project_file_watcher,
};
use serde_json::Value;
use std::{
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::Manager;
use terminal::{
    TerminalManager, resize_terminal, start_terminal, stop_all_terminals, stop_terminal,
    stop_terminal_for_thread, write_terminal,
};

fn git_command() -> Command {
    let mut command = Command::new("git");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    command
}

#[cfg(windows)]
fn configure_native_window(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    use windows::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        DwmSetWindowAttribute,
    };

    let hwnd = windows::Win32::Foundation::HWND(window.hwnd()?.0);
    let corner_preference = DWMWCP_ROUND;
    let border_color = DWMWA_COLOR_NONE;

    // These attributes were introduced in Windows 11. Ignore unsupported-OS
    // errors so the app continues to start normally on older Windows versions.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::from_ref(&corner_preference).cast(),
            std::mem::size_of_val(&corner_preference) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            std::ptr::from_ref(&border_color).cast(),
            std::mem::size_of_val(&border_color) as u32,
        );
    }

    Ok(())
}

#[tauri::command]
fn initial_working_directory() -> Result<String, String> {
    std::env::current_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| format!("Could not resolve the initial working directory: {error}"))
}

fn loopcode_data_directory(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve the application data directory: {error}"))
}

#[tauri::command]
async fn load_workspace(app: tauri::AppHandle) -> Result<Option<Value>, String> {
    let directory = loopcode_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        persistence::load_from_directory(&directory, |contents| {
            serde_json::from_str(contents).map_err(|error| error.to_string())
        })
    })
    .await
    .map_err(|error| format!("Could not join the thread-loading task: {error}"))?
}

#[tauri::command]
async fn save_workspace(app: tauri::AppHandle, workspace: Value) -> Result<(), String> {
    let directory = loopcode_data_directory(&app)?;
    let workspace_json = serde_json::to_string_pretty(&workspace)
        .map_err(|error| format!("Could not serialize the LoopCode workspace: {error}"))?;
    tauri::async_runtime::spawn_blocking(move || {
        persistence::save_to_directory(&directory, &workspace_json)
    })
    .await
    .map_err(|error| format!("Could not join the thread-saving task: {error}"))?
}

#[tauri::command]
fn record_diagnostic(
    diagnostics: tauri::State<'_, Diagnostics>,
    level: String,
    event_name: String,
    fields: Value,
) {
    diagnostics.record(&level, &event_name, fields);
}

#[tauri::command]
async fn export_diagnostics(
    diagnostics: tauri::State<'_, Diagnostics>,
) -> Result<Option<String>, String> {
    let diagnostics = diagnostics.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let destination = rfd::FileDialog::new()
            .set_title("Export LoopCode diagnostics")
            .set_file_name("loopcode-acp-diagnostics.jsonl")
            .add_filter("JSON Lines", &["jsonl"])
            .save_file();
        let Some(destination) = destination else {
            return Ok(None);
        };
        match diagnostics.export_to(&destination) {
            Ok(()) => {
                diagnostics.record("info", "diagnostics.exported", serde_json::json!({}));
                Ok(Some(destination.to_string_lossy().into_owned()))
            }
            Err(error) => {
                diagnostics.record(
                    "error",
                    "diagnostics.export_failed",
                    serde_json::json!({ "message": error }),
                );
                Err(error)
            }
        }
    })
    .await
    .map_err(|error| format!("Could not join the diagnostics export task: {error}"))?
}

#[tauri::command]
async fn pick_folder() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("Choose folder")
            .pick_folder()
            .map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|error| format!("Could not join folder picker task: {error}"))
}

#[tauri::command]
async fn get_git_branch(cwd: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let branch = git_command()
            .args(["-C", &cwd, "branch", "--show-current"])
            .output();
        if let Ok(output) = branch
            && output.status.success()
        {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if !name.is_empty() {
                return Some(name);
            }
        }

        let revision = git_command()
            .args(["-C", &cwd, "rev-parse", "--short", "HEAD"])
            .output();
        if let Ok(output) = revision
            && output.status.success()
        {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if !name.is_empty() {
                return Some(name);
            }
        }

        None
    })
    .await
    .map_err(|error| format!("Could not join Git branch task: {error}"))
}

pub fn run() {
    let shutdown_started = Arc::new(AtomicBool::new(false));
    let shutdown_completed = Arc::new(AtomicBool::new(false));
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Broker::default())
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
            list_composer_completions,
            read_project_directory,
            read_project_file,
            open_project_file,
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
                let shutdown_started = Arc::clone(&shutdown_started);
                let shutdown_completed = Arc::clone(&shutdown_completed);
                tauri::async_runtime::spawn(async move {
                    let broker = app.state::<Broker>();
                    let terminals = app.state::<TerminalManager>();
                    let diagnostics = app.state::<Diagnostics>();
                    let harnesses_stopped = broker.shutdown(&diagnostics).await.is_ok();
                    let terminals_stopped = terminals.shutdown(&diagnostics).await.is_ok();
                    if harnesses_stopped && terminals_stopped {
                        shutdown_completed.store(true, Ordering::Release);
                        app.exit(code.unwrap_or(0));
                    } else {
                        shutdown_started.store(false, Ordering::Release);
                    }
                });
            }
        }
    });
}
