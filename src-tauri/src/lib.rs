mod broker;
mod diagnostics;
mod persistence;

use broker::{
    Broker, launch_harness, register_frontend, send_rpc, stop_all_harnesses, stop_harness,
};
use diagnostics::Diagnostics;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tauri::Manager;
use tauri::ipc::Channel;
use tauri_plugin_opener::OpenerExt;

#[derive(Default)]
struct ProjectFileWatchers {
    next_id: AtomicU64,
    watchers: Mutex<HashMap<u64, RecommendedWatcher>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectFileEntry {
    name: String,
    path: String,
    is_directory: bool,
    is_symlink: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectFileChange {
    paths: Vec<String>,
}

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

fn resolve_project_path(project_root: &str, candidate: &str) -> Result<PathBuf, String> {
    let root = Path::new(project_root)
        .canonicalize()
        .map_err(|error| format!("Could not resolve project folder: {error}"))?;
    let path = Path::new(candidate)
        .canonicalize()
        .map_err(|error| format!("Could not resolve project path: {error}"))?;
    if !path.starts_with(&root) {
        return Err("The requested path is outside the project folder.".to_owned());
    }
    Ok(path)
}

#[tauri::command]
async fn read_project_directory(
    project_root: String,
    directory: String,
) -> Result<Vec<ProjectFileEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let directory = resolve_project_path(&project_root, &directory)?;
        if !directory.is_dir() {
            return Err("The requested project path is not a folder.".to_owned());
        }

        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&directory)
            .map_err(|error| format!("Could not read project folder: {error}"))?
        {
            let entry = entry.map_err(|error| format!("Could not read project entry: {error}"))?;
            let file_type = entry
                .file_type()
                .map_err(|error| format!("Could not inspect project entry: {error}"))?;
            entries.push(ProjectFileEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path().to_string_lossy().into_owned(),
                is_directory: file_type.is_dir(),
                is_symlink: file_type.is_symlink(),
            });
        }
        entries.sort_by(|left, right| {
            right
                .is_directory
                .cmp(&left.is_directory)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(entries)
    })
    .await
    .map_err(|error| format!("Could not join project folder task: {error}"))?
}

fn project_image_media_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "avif" => Some("image/avif"),
        "bmp" => Some("image/bmp"),
        "gif" => Some("image/gif"),
        "ico" => Some("image/x-icon"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "svg" => Some("image/svg+xml"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

#[tauri::command]
async fn read_project_file(
    project_root: String,
    path: String,
) -> Result<tauri::ipc::Response, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = resolve_project_path(&project_root, &path)?;
        if !path.is_file() {
            return Err("The requested project path is not a file.".to_owned());
        }

        let media_type = project_image_media_type(&path);
        let limit = if media_type.is_some() {
            10 * 1024 * 1024
        } else {
            1024 * 1024
        };
        let size = path
            .metadata()
            .map_err(|error| format!("Could not inspect file: {error}"))?
            .len();
        if size > limit {
            return Err(if media_type.is_some() {
                "Images larger than 10 MB cannot be previewed.".to_owned()
            } else {
                "Text files larger than 1 MB cannot be previewed.".to_owned()
            });
        }

        let bytes =
            std::fs::read(&path).map_err(|error| format!("Could not read file: {error}"))?;
        if bytes.len() as u64 > limit {
            return Err(
                "The file grew beyond the preview limit while it was being read.".to_owned(),
            );
        }
        Ok(tauri::ipc::Response::new(bytes))
    })
    .await
    .map_err(|error| format!("Could not join project file task: {error}"))?
}

#[tauri::command]
async fn open_project_file(
    app: tauri::AppHandle,
    project_root: String,
    path: String,
) -> Result<(), String> {
    let path = tauri::async_runtime::spawn_blocking(move || {
        let path = resolve_project_path(&project_root, &path)?;
        if !path.is_file() {
            return Err("The requested project path is not a file.".to_owned());
        }
        Ok(path)
    })
    .await
    .map_err(|error| format!("Could not join project file task: {error}"))??;

    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<String>)
        .map_err(|error| format!("Could not open file: {error}"))
}

#[tauri::command]
async fn open_project_path(
    app: tauri::AppHandle,
    project_root: String,
    path: String,
) -> Result<(), String> {
    let path =
        tauri::async_runtime::spawn_blocking(move || resolve_project_path(&project_root, &path))
            .await
            .map_err(|error| format!("Could not join project path task: {error}"))??;

    app.opener()
        .open_path(path.to_string_lossy().into_owned(), None::<String>)
        .map_err(|error| format!("Could not open path: {error}"))
}

#[tauri::command]
async fn reveal_project_path(
    app: tauri::AppHandle,
    project_root: String,
    path: String,
) -> Result<(), String> {
    let path =
        tauri::async_runtime::spawn_blocking(move || resolve_project_path(&project_root, &path))
            .await
            .map_err(|error| format!("Could not join project path task: {error}"))??;

    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|error| format!("Could not reveal path: {error}"))
}

#[tauri::command]
fn start_project_file_watcher(
    state: tauri::State<'_, ProjectFileWatchers>,
    project_root: String,
    on_change: Channel<ProjectFileChange>,
) -> Result<u64, String> {
    let root = Path::new(&project_root)
        .canonicalize()
        .map_err(|error| format!("Could not resolve project folder: {error}"))?;
    if !root.is_dir() {
        return Err("The project path is not a folder.".to_owned());
    }

    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        let Ok(event) = result else { return };
        let _ = on_change.send(ProjectFileChange {
            paths: event
                .paths
                .into_iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
        });
    })
    .map_err(|error| format!("Could not create project watcher: {error}"))?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|error| format!("Could not watch project folder: {error}"))?;

    let watcher_id = state.next_id.fetch_add(1, Ordering::Relaxed) + 1;
    state
        .watchers
        .lock()
        .map_err(|_| "The project watcher lock was poisoned.".to_owned())?
        .insert(watcher_id, watcher);
    Ok(watcher_id)
}

#[tauri::command]
fn stop_project_file_watcher(
    state: tauri::State<'_, ProjectFileWatchers>,
    watcher_id: u64,
) -> Result<(), String> {
    state
        .watchers
        .lock()
        .map_err(|_| "The project watcher lock was poisoned.".to_owned())?
        .remove(&watcher_id);
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::project_image_media_type;
    use std::path::Path;

    #[test]
    fn recognizes_previewable_image_extensions() {
        assert_eq!(
            project_image_media_type(Path::new("image.PNG")),
            Some("image/png")
        );
        assert_eq!(project_image_media_type(Path::new("component.ts")), None);
    }
}

pub fn run() {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let shutdown_started = Arc::new(AtomicBool::new(false));
    let shutdown_completed = Arc::new(AtomicBool::new(false));
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Broker::default())
        .manage(Diagnostics::new(home.join(".loopcode").join("logs")))
        .manage(ProjectFileWatchers::default())
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
            read_project_directory,
            read_project_file,
            open_project_file,
            open_project_path,
            reveal_project_path,
            start_project_file_watcher,
            stop_project_file_watcher,
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
                    let diagnostics = app.state::<Diagnostics>();
                    if broker.shutdown(&diagnostics).await.is_ok() {
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
