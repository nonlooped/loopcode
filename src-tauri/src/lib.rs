mod broker;
mod persistence;

use broker::{Broker, launch_harness, send_rpc, stop_all_harnesses, stop_harness};
use serde_json::Value;
use std::process::Command;
use tauri::Manager;

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

    let hwnd = window.hwnd()?;
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
        .home_dir()
        .map(|home| home.join(".loopcode"))
        .map_err(|error| format!("Could not resolve the user home directory: {error}"))
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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Broker::default())
        .setup(|app| {
            #[cfg(windows)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    configure_native_window(&window)?;
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            launch_harness,
            send_rpc,
            stop_harness,
            stop_all_harnesses,
            initial_working_directory,
            load_workspace,
            save_workspace,
            pick_folder,
            get_git_branch,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run LoopCode");
}
