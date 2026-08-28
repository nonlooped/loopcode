use crate::{diagnostics::Diagnostics, native_command, persistence};
use serde_json::Value;
use tauri::Manager;

#[cfg(windows)]
pub(crate) fn configure_native_window(window: &tauri::WebviewWindow) -> tauri::Result<()> {
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
pub(crate) fn initial_working_directory() -> Result<String, String> {
    std::env::current_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| format!("Could not resolve the initial working directory: {error}"))
}

pub(crate) fn loopcode_data_directory(
    app: &tauri::AppHandle,
) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve the application data directory: {error}"))
}

#[tauri::command]
pub(crate) async fn load_workspace(app: tauri::AppHandle) -> Result<Option<Value>, String> {
    let directory = loopcode_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || persistence::load_from_directory(&directory))
        .await
        .map_err(|error| format!("Could not join the thread-loading task: {error}"))?
}

#[tauri::command]
pub(crate) async fn save_workspace(app: tauri::AppHandle, workspace: Value) -> Result<(), String> {
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
pub(crate) fn record_diagnostic(
    diagnostics: tauri::State<'_, Diagnostics>,
    level: String,
    event_name: String,
    fields: Value,
) {
    diagnostics.record(&level, &event_name, fields);
}

#[tauri::command]
pub(crate) async fn export_diagnostics(
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
pub(crate) async fn pick_folder() -> Result<Option<String>, String> {
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
pub(crate) async fn provider_version(
    command: String,
    args: Vec<String>,
) -> Result<Option<String>, String> {
    validate_provider_command(&command, &args)?;
    let command = command.trim();

    let mut process = native_command(command);
    process
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let output =
        match tokio::time::timeout(std::time::Duration::from_secs(4), process.output()).await {
            Ok(Ok(output)) if output.status.success() => output,
            _ => return Ok(None),
        };
    Ok(first_version_line(&output.stdout, &output.stderr))
}

fn validate_provider_command(command: &str, args: &[String]) -> Result<(), String> {
    if command.trim().is_empty() || command.len() > 4096 {
        return Err("Enter a valid provider executable".into());
    }
    if args.len() > 16 || args.iter().any(|arg| arg.len() > 4096) {
        return Err("Provider command arguments are too long".into());
    }
    Ok(())
}

fn first_version_line(stdout: &[u8], stderr: &[u8]) -> Option<String> {
    let bytes = if stdout.iter().any(|byte| !byte.is_ascii_whitespace()) {
        stdout
    } else {
        stderr
    };
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.chars().take(512).collect())
}

#[cfg(test)]
mod tests {
    use super::{first_version_line, validate_provider_command};

    #[test]
    fn provider_version_output_prefers_stdout_and_is_bounded() {
        assert_eq!(
            first_version_line(b"\ncodex-cli 0.149.0\n", b"ignored"),
            Some("codex-cli 0.149.0".into())
        );
        assert_eq!(
            first_version_line(b"", format!("  {}  ", "x".repeat(600)).as_bytes())
                .map(|line| line.len()),
            Some(512)
        );
    }

    #[test]
    fn provider_metadata_commands_are_bounded() {
        assert!(validate_provider_command("codex", &["--version".into()]).is_ok());
        assert!(validate_provider_command("", &[]).is_err());
        assert!(validate_provider_command("codex", &vec!["x".into(); 17]).is_err());
    }
}
