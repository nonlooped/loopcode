mod broker;
mod diagnostics;
mod persistence;
mod project_files;
mod terminal;

use broker::{
    Broker, FrontendGeneration, launch_harness, register_frontend, send_rpc, stop_all_harnesses,
    stop_harness,
};
use diagnostics::Diagnostics;
use project_files::{
    ProjectFileWatchers, list_composer_completions, open_project_path, read_project_directory,
    read_project_file, reveal_project_path, start_project_file_watcher, stop_project_file_watcher,
};
use serde::Serialize;
use serde_json::Value;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
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

fn git_command() -> tokio::process::Command {
    let command = tokio::process::Command::new("git");
    #[cfg(target_os = "windows")]
    let command = {
        use std::os::windows::process::CommandExt;
        let mut command = command;
        command.as_std_mut().creation_flags(0x0800_0000);
        command
    };
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

fn validate_git_cwd(cwd: &str) -> Result<PathBuf, String> {
    let path = Path::new(cwd);
    if !path.is_absolute() {
        return Err("The Git working folder must be an absolute path".to_owned());
    }
    let path = path
        .canonicalize()
        .map_err(|error| format!("Could not resolve Git working folder: {error}"))?;
    if !path.is_dir() {
        return Err("The Git working folder must be a directory".to_owned());
    }
    Ok(path)
}

async fn git_ref(cwd: &Path, args: &[&str]) -> Option<String> {
    let mut process = git_command();
    process
        .current_dir(cwd)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let output = tokio::time::timeout(std::time::Duration::from_secs(4), process.output())
        .await
        .ok()?
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!name.is_empty()).then_some(name)
}

#[tauri::command]
async fn get_git_branch(cwd: String) -> Result<Option<String>, String> {
    let cwd = tauri::async_runtime::spawn_blocking(move || validate_git_cwd(&cwd))
        .await
        .map_err(|error| format!("Could not join Git path validation task: {error}"))??;
    if let Some(branch) = git_ref(&cwd, &["branch", "--show-current"]).await {
        return Ok(Some(branch));
    }
    Ok(git_ref(&cwd, &["rev-parse", "--short", "HEAD"]).await)
}

async fn git_output<I, S>(
    cwd: &Path,
    args: I,
    timeout_seconds: u64,
    context: &str,
) -> Result<std::process::Output, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut process = git_command();
    process
        .current_dir(cwd)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_seconds),
        process.output(),
    )
    .await
    .map_err(|_| format!("{context}: Git timed out"))?
    .map_err(|error| format!("{context}: {error}"))?;
    if output.status.success() {
        return Ok(output);
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let detail: String = detail.chars().take(2048).collect();
    Err(if detail.is_empty() {
        context.to_owned()
    } else {
        format!("{context}: {detail}")
    })
}

fn validate_git_branch_value<'a>(branch: &'a str, label: &str) -> Result<&'a str, String> {
    if branch.is_empty()
        || branch.len() > 1024
        || branch.trim() != branch
        || branch.starts_with('-')
        || branch.chars().any(char::is_control)
    {
        return Err(format!("Enter a valid {label}"));
    }
    Ok(branch)
}

async fn check_git_branch_name(cwd: &Path, branch: &str, label: &str) -> Result<(), String> {
    let branch = validate_git_branch_value(branch, label)?;
    git_output(
        cwd,
        ["check-ref-format", "--branch", branch],
        4,
        &format!("Enter a valid {label}"),
    )
    .await?;
    Ok(())
}

async fn list_git_branches_at(cwd: &Path) -> Result<Vec<String>, String> {
    let output = git_output(
        cwd,
        ["for-each-ref", "--format=%(refname:lstrip=2)", "refs/heads"],
        4,
        "Could not list Git branches",
    )
    .await?;
    let branches: Vec<_> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|branch| !branch.is_empty())
        .map(str::to_owned)
        .collect();
    if branches.len() > 10_000 || branches.iter().any(|branch| branch.len() > 1024) {
        return Err("The repository has too many Git branches to display".to_owned());
    }
    Ok(branches)
}

#[tauri::command]
async fn list_git_branches(cwd: String) -> Result<Vec<String>, String> {
    let cwd = tauri::async_runtime::spawn_blocking(move || validate_git_cwd(&cwd))
        .await
        .map_err(|error| format!("Could not join Git path validation task: {error}"))??;
    list_git_branches_at(&cwd).await
}

async fn switch_git_branch_at(cwd: &Path, branch: &str) -> Result<(), String> {
    check_git_branch_name(cwd, branch, "branch name").await?;
    git_output(
        cwd,
        ["switch", "--no-guess", "--", branch],
        15,
        "Could not change Git branch",
    )
    .await?;
    Ok(())
}

#[tauri::command]
async fn switch_git_branch(cwd: String, branch: String) -> Result<(), String> {
    let cwd = tauri::async_runtime::spawn_blocking(move || validate_git_cwd(&cwd))
        .await
        .map_err(|error| format!("Could not join Git path validation task: {error}"))??;
    switch_git_branch_at(&cwd, &branch).await
}

fn safe_path_component(value: &str) -> String {
    let component: String = value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .take(80)
        .collect();
    let component = component.trim_matches(['-', '.']);
    if component.is_empty() {
        "worktree".to_owned()
    } else {
        component.to_owned()
    }
}

fn allocate_worktree_destination(
    worktrees_root: &Path,
    repository_root: &Path,
    branch: &str,
) -> Result<PathBuf, String> {
    let repository_name = repository_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repository");
    let parent = worktrees_root.join(safe_path_component(repository_name));
    std::fs::create_dir_all(&parent)
        .map_err(|error| format!("Could not prepare the worktree directory: {error}"))?;
    let name = format!("wt-{}", safe_path_component(branch));
    for suffix in 1..=10_000 {
        let destination = if suffix == 1 {
            parent.join(&name)
        } else {
            parent.join(format!("{name}-{suffix}"))
        };
        if !destination
            .try_exists()
            .map_err(|error| format!("Could not inspect the worktree directory: {error}"))?
        {
            return Ok(destination);
        }
    }
    Err("Could not allocate a worktree directory".to_owned())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GitWorktreeResult {
    path: String,
    branch: String,
}

async fn create_git_worktree_at(
    cwd: &Path,
    worktrees_root: &Path,
    base_branch: &str,
    branch: &str,
) -> Result<GitWorktreeResult, String> {
    check_git_branch_name(cwd, base_branch, "base branch").await?;
    check_git_branch_name(cwd, branch, "new branch name").await?;
    let root_output = git_output(
        cwd,
        ["rev-parse", "--show-toplevel"],
        4,
        "Could not resolve the Git repository",
    )
    .await?;
    let repository_root = PathBuf::from(String::from_utf8_lossy(&root_output.stdout).trim());
    let worktrees_root = worktrees_root.to_owned();
    let destination_branch = branch.to_owned();
    let destination = tauri::async_runtime::spawn_blocking(move || {
        allocate_worktree_destination(&worktrees_root, &repository_root, &destination_branch)
    })
    .await
    .map_err(|error| format!("Could not join worktree path allocation task: {error}"))??;
    let args = vec![
        OsString::from("worktree"),
        OsString::from("add"),
        OsString::from("-b"),
        OsString::from(branch),
        OsString::from("--"),
        destination.as_os_str().to_owned(),
        OsString::from(base_branch),
    ];
    git_output(cwd, args, 30, "Could not create Git worktree").await?;
    let destination = tauri::async_runtime::spawn_blocking(move || {
        destination
            .canonicalize()
            .map_err(|error| format!("Could not resolve the new worktree: {error}"))
    })
    .await
    .map_err(|error| format!("Could not join worktree path validation task: {error}"))??;
    Ok(GitWorktreeResult {
        path: destination.to_string_lossy().into_owned(),
        branch: branch.to_owned(),
    })
}

#[tauri::command]
async fn create_git_worktree(
    app: tauri::AppHandle,
    cwd: String,
    base_branch: String,
    branch: String,
) -> Result<GitWorktreeResult, String> {
    let worktrees_root = loopcode_data_directory(&app)?.join("worktrees");
    let cwd = tauri::async_runtime::spawn_blocking(move || validate_git_cwd(&cwd))
        .await
        .map_err(|error| format!("Could not join Git path validation task: {error}"))??;
    create_git_worktree_at(&cwd, &worktrees_root, &base_branch, &branch).await
}

#[tauri::command]
async fn provider_version(command: String, args: Vec<String>) -> Result<Option<String>, String> {
    validate_provider_command(&command, &args)?;
    let command = command.trim();

    let mut process = tokio::process::Command::new(command);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        process.as_std_mut().creation_flags(0x0800_0000);
    }
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

#[tauri::command]
async fn provider_auth_status(command: String, args: Vec<String>) -> Result<Option<bool>, String> {
    validate_provider_command(&command, &args)?;
    let mut process = tokio::process::Command::new(command.trim());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        process.as_std_mut().creation_flags(0x0800_0000);
    }
    process
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    Ok(
        match tokio::time::timeout(std::time::Duration::from_secs(4), process.status()).await {
            Ok(Ok(status)) => Some(status.success()),
            _ => None,
        },
    )
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

#[cfg(test)]
mod tests {
    use super::{
        create_git_worktree_at, first_version_line, list_git_branches_at, switch_git_branch_at,
        validate_git_cwd, validate_provider_command,
    };
    use std::{path::Path, process::Command};

    fn run_test_git(cwd: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(cwd)
            .args(args)
            .output()
            .expect("Git should run during tests");
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    #[test]
    fn git_working_folder_must_be_an_existing_absolute_directory() {
        assert!(validate_git_cwd("relative/path").is_err());
        let directory = tempfile::tempdir().expect("test directory should be created");
        assert_eq!(
            validate_git_cwd(&directory.path().to_string_lossy())
                .expect("temporary directory should validate"),
            directory
                .path()
                .canonicalize()
                .expect("path should resolve")
        );
    }

    #[test]
    fn git_branches_can_be_switched_and_used_for_managed_worktrees() {
        let directory = tempfile::tempdir().expect("test directory should be created");
        let repository = directory.path().join("repository");
        std::fs::create_dir(&repository).expect("repository directory should be created");
        run_test_git(&repository, &["init", "--initial-branch=main"]);
        run_test_git(&repository, &["config", "user.email", "test@example.com"]);
        run_test_git(&repository, &["config", "user.name", "LoopCode tests"]);
        std::fs::write(repository.join("README.md"), "test\n")
            .expect("test file should be written");
        run_test_git(&repository, &["add", "README.md"]);
        run_test_git(&repository, &["commit", "-m", "Initial commit"]);
        run_test_git(&repository, &["branch", "other"]);
        run_test_git(&repository, &["tag", "other"]);

        tauri::async_runtime::block_on(async {
            let branches = list_git_branches_at(&repository)
                .await
                .expect("branches should be listed");
            assert!(branches.contains(&"main".to_owned()));
            assert!(branches.contains(&"other".to_owned()));
            assert!(!branches.contains(&"heads/other".to_owned()));

            switch_git_branch_at(&repository, "other")
                .await
                .expect("branch should switch");
            assert_eq!(
                run_test_git(&repository, &["branch", "--show-current"]),
                "other"
            );

            let worktree = create_git_worktree_at(
                &repository,
                &directory.path().join("worktrees"),
                "main",
                "feature/test",
            )
            .await
            .expect("worktree should be created");
            assert_eq!(worktree.branch, "feature/test");
            assert_eq!(
                run_test_git(Path::new(&worktree.path), &["branch", "--show-current"]),
                "feature/test"
            );
        });
    }

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
        assert!(validate_provider_command("claude", &["auth".into(), "status".into()]).is_ok());
        assert!(validate_provider_command("", &[]).is_err());
        assert!(validate_provider_command("claude", &vec!["x".into(); 17]).is_err());
    }
}
