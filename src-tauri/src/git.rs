use crate::{app_commands::loopcode_data_directory, native_command};
use serde::Serialize;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

fn git_command() -> tokio::process::Command {
    native_command("git")
}
const MAX_GIT_CHANGES: usize = 10_000;
const MAX_GIT_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_GIT_DIFF_BYTES: usize = 2 * 1024 * 1024;
const MAX_GIT_PATH_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum GitChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Conflicted,
    TypeChanged,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitChange {
    path: String,
    old_path: Option<String>,
    status: GitChangeStatus,
    staged: bool,
    unstaged: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitChanges {
    changes: Vec<GitChange>,
    additions: usize,
    deletions: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitFileDiff {
    hunks: Vec<String>,
    binary: bool,
    too_large: bool,
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

async fn validated_git_cwd(cwd: String) -> Result<PathBuf, String> {
    tauri::async_runtime::spawn_blocking(move || validate_git_cwd(&cwd))
        .await
        .map_err(|error| format!("Could not join Git path validation task: {error}"))?
}

fn validate_git_relative_path(path: &str) -> Result<&str, String> {
    if path.is_empty()
        || path.len() > MAX_GIT_PATH_BYTES
        || Path::new(path).is_absolute()
        || Path::new(path)
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err("Git returned an invalid changed-file path".to_owned());
    }
    Ok(path)
}

fn git_path(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > MAX_GIT_PATH_BYTES {
        return Err("A changed-file path is too long to display".to_owned());
    }
    let path = String::from_utf8_lossy(bytes).into_owned();
    validate_git_relative_path(&path)?;
    Ok(path)
}

fn ensure_git_output_size(output: &[u8]) -> Result<(), String> {
    if output.len() > MAX_GIT_OUTPUT_BYTES {
        Err("The Git change list is too large to display".to_owned())
    } else {
        Ok(())
    }
}

fn git_numstat_count(value: &[u8]) -> Result<usize, String> {
    if value == b"-" {
        return Ok(0);
    }
    std::str::from_utf8(value)
        .ok()
        .and_then(|count| count.parse().ok())
        .ok_or_else(|| "Git returned an invalid line count".to_owned())
}

fn parse_git_numstat(output: &[u8]) -> Result<(usize, usize), String> {
    ensure_git_output_size(output)?;
    let mut fields = output.split(|byte| *byte == 0);
    let mut additions = 0usize;
    let mut deletions = 0usize;
    while let Some(record) = fields.next() {
        if record.is_empty() {
            continue;
        }
        let mut parts = record.splitn(3, |byte| *byte == b'\t');
        additions = additions
            .checked_add(git_numstat_count(parts.next().unwrap_or_default())?)
            .ok_or_else(|| "Git returned too many additions".to_owned())?;
        deletions = deletions
            .checked_add(git_numstat_count(parts.next().unwrap_or_default())?)
            .ok_or_else(|| "Git returned too many deletions".to_owned())?;
        let raw_path = parts
            .next()
            .ok_or_else(|| "Git returned incomplete line statistics".to_owned())?;
        if raw_path.is_empty() {
            git_path(
                fields
                    .next()
                    .ok_or_else(|| "Git returned an incomplete renamed-file path".to_owned())?,
            )?;
            git_path(
                fields
                    .next()
                    .ok_or_else(|| "Git returned an incomplete renamed-file path".to_owned())?,
            )?;
        } else {
            git_path(raw_path)?;
        }
    }
    Ok((additions, deletions))
}

fn working_change_status(index: char, worktree: char) -> GitChangeStatus {
    if index == '?' && worktree == '?' {
        return GitChangeStatus::Untracked;
    }
    if index == 'U' || worktree == 'U' || matches!((index, worktree), ('A', 'A') | ('D', 'D')) {
        return GitChangeStatus::Conflicted;
    }
    if index == 'R' || worktree == 'R' {
        return GitChangeStatus::Renamed;
    }
    if index == 'C' || worktree == 'C' {
        return GitChangeStatus::Copied;
    }
    if index == 'A' || worktree == 'A' {
        return GitChangeStatus::Added;
    }
    if index == 'D' || worktree == 'D' {
        return GitChangeStatus::Deleted;
    }
    if index == 'T' || worktree == 'T' {
        return GitChangeStatus::TypeChanged;
    }
    GitChangeStatus::Modified
}

fn parse_working_tree_changes(output: &[u8]) -> Result<Vec<GitChange>, String> {
    ensure_git_output_size(output)?;
    let mut fields = output
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    let mut changes = Vec::new();
    while let Some(record) = fields.next() {
        if record.len() < 4 || record[2] != b' ' {
            return Err("Git returned an invalid working-tree status".to_owned());
        }
        let index = char::from(record[0]);
        let worktree = char::from(record[1]);
        let status = working_change_status(index, worktree);
        let path = git_path(&record[3..])?;
        let old_path = if matches!(status, GitChangeStatus::Renamed | GitChangeStatus::Copied) {
            Some(git_path(fields.next().ok_or_else(|| {
                "Git returned an incomplete renamed-file status".to_owned()
            })?)?)
        } else {
            None
        };
        changes.push(GitChange {
            path,
            old_path,
            status,
            staged: index != ' ' && index != '?',
            unstaged: worktree != ' ',
        });
        if changes.len() > MAX_GIT_CHANGES {
            return Err("The repository has too many changed files to display".to_owned());
        }
    }
    Ok(changes)
}

fn branch_change_status(value: u8) -> Result<GitChangeStatus, String> {
    match value {
        b'A' => Ok(GitChangeStatus::Added),
        b'M' => Ok(GitChangeStatus::Modified),
        b'D' => Ok(GitChangeStatus::Deleted),
        b'R' => Ok(GitChangeStatus::Renamed),
        b'C' => Ok(GitChangeStatus::Copied),
        b'T' => Ok(GitChangeStatus::TypeChanged),
        b'U' => Ok(GitChangeStatus::Conflicted),
        _ => Err("Git returned an unknown branch-change status".to_owned()),
    }
}

fn parse_branch_changes(output: &[u8]) -> Result<Vec<GitChange>, String> {
    ensure_git_output_size(output)?;
    let mut fields = output
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    let mut changes = Vec::new();
    while let Some(raw_status) = fields.next() {
        let status = branch_change_status(
            *raw_status
                .first()
                .ok_or_else(|| "Git returned an empty branch-change status".to_owned())?,
        )?;
        let first_path = git_path(
            fields
                .next()
                .ok_or_else(|| "Git returned an incomplete branch-change status".to_owned())?,
        )?;
        let (path, old_path) =
            if matches!(status, GitChangeStatus::Renamed | GitChangeStatus::Copied) {
                (
                    git_path(fields.next().ok_or_else(|| {
                        "Git returned an incomplete renamed-file status".to_owned()
                    })?)?,
                    Some(first_path),
                )
            } else {
                (first_path, None)
            };
        changes.push(GitChange {
            path,
            old_path,
            status,
            staged: false,
            unstaged: false,
        });
        if changes.len() > MAX_GIT_CHANGES {
            return Err("The branch comparison has too many changed files to display".to_owned());
        }
    }
    Ok(changes)
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
pub(crate) async fn get_git_branch(cwd: String) -> Result<Option<String>, String> {
    let cwd = validated_git_cwd(cwd).await?;
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

async fn git_numstat_at(
    cwd: &Path,
    comparison: &str,
    context: &str,
) -> Result<(usize, usize), String> {
    let args = [
        "diff",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--numstat",
        "-z",
        "--find-renames",
        "--relative",
        comparison,
        "--",
        ".",
    ];
    let output = git_output(cwd, args, 10, context).await?;
    parse_git_numstat(&output.stdout)
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

async fn git_branch_comparison_ref(cwd: &Path, branch: &str) -> Result<String, String> {
    check_git_branch_name(cwd, branch, "base branch").await?;
    let reference = format!("refs/heads/{branch}");
    git_output(
        cwd,
        ["rev-parse", "--verify", "--quiet", reference.as_str()],
        4,
        "Could not resolve the base branch",
    )
    .await?;
    Ok(format!("{reference}...HEAD"))
}

async fn list_git_changes_at(cwd: &Path, base_branch: Option<&str>) -> Result<GitChanges, String> {
    if let Some(base_branch) = base_branch {
        let comparison = git_branch_comparison_ref(cwd, base_branch).await?;
        let args = [
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--name-status",
            "-z",
            "--find-renames",
            "--relative",
            comparison.as_str(),
            "--",
            ".",
        ];
        let output = git_output(cwd, args, 10, "Could not compare Git branches").await?;
        let (additions, deletions) =
            git_numstat_at(cwd, &comparison, "Could not read branch line statistics").await?;
        return Ok(GitChanges {
            changes: parse_branch_changes(&output.stdout)?,
            additions,
            deletions,
        });
    }

    let output = git_output(
        cwd,
        [
            "-c",
            "status.relativePaths=true",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--",
            ".",
        ],
        10,
        "Could not read Git changes",
    )
    .await?;
    let changes = parse_working_tree_changes(&output.stdout)?;
    let has_head = git_ref(cwd, &["rev-parse", "--verify", "HEAD"])
        .await
        .is_some();
    let (additions, deletions) = if has_head {
        git_numstat_at(cwd, "HEAD", "Could not read Git line statistics").await?
    } else {
        (0, 0)
    };
    Ok(GitChanges {
        changes,
        additions,
        deletions,
    })
}

#[tauri::command]
pub(crate) async fn list_git_changes(
    cwd: String,
    base_branch: Option<String>,
) -> Result<GitChanges, String> {
    let cwd = validated_git_cwd(cwd).await?;
    list_git_changes_at(&cwd, base_branch.as_deref()).await
}

fn extract_git_hunks(patch: &[u8]) -> GitFileDiff {
    if patch.len() > MAX_GIT_DIFF_BYTES {
        return GitFileDiff {
            hunks: Vec::new(),
            binary: false,
            too_large: true,
        };
    }
    let patch = String::from_utf8_lossy(patch);
    let binary = patch
        .lines()
        .any(|line| line.starts_with("Binary files ") || line == "GIT binary patch");
    let mut hunks = Vec::new();
    let mut current = String::new();
    for line in patch.split_inclusive('\n') {
        if line.starts_with("@@ ") {
            if !current.is_empty() {
                hunks.push(std::mem::take(&mut current));
            }
            current.push_str(line);
        } else if line.starts_with("diff --git ") {
            if !current.is_empty() {
                hunks.push(std::mem::take(&mut current));
            }
        } else if !current.is_empty() {
            current.push_str(line);
        }
    }
    if !current.is_empty() {
        hunks.push(current);
    }
    GitFileDiff {
        hunks,
        binary,
        too_large: false,
    }
}

fn untracked_file_diff(cwd: &Path, path: &str) -> Result<GitFileDiff, String> {
    validate_git_relative_path(path)?;
    let cwd = cwd
        .canonicalize()
        .map_err(|error| format!("Could not resolve Git working folder: {error}"))?;
    let candidate = cwd.join(path);
    let symlink_metadata = candidate
        .symlink_metadata()
        .map_err(|error| format!("Could not inspect changed file: {error}"))?;
    if symlink_metadata.file_type().is_symlink() {
        return Ok(GitFileDiff {
            hunks: Vec::new(),
            binary: true,
            too_large: false,
        });
    }
    let resolved = candidate
        .canonicalize()
        .map_err(|error| format!("Could not resolve changed file: {error}"))?;
    if !resolved.starts_with(&cwd) || !resolved.is_file() {
        return Err("The changed file is outside the working folder".to_owned());
    }
    if symlink_metadata.len() > MAX_GIT_DIFF_BYTES as u64 {
        return Ok(GitFileDiff {
            hunks: Vec::new(),
            binary: false,
            too_large: true,
        });
    }
    let bytes =
        std::fs::read(resolved).map_err(|error| format!("Could not read changed file: {error}"))?;
    if bytes.len() > MAX_GIT_DIFF_BYTES {
        return Ok(GitFileDiff {
            hunks: Vec::new(),
            binary: false,
            too_large: true,
        });
    }
    if bytes.contains(&0) {
        return Ok(GitFileDiff {
            hunks: Vec::new(),
            binary: true,
            too_large: false,
        });
    }
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Ok(GitFileDiff {
            hunks: Vec::new(),
            binary: true,
            too_large: false,
        });
    };
    if text.is_empty() {
        return Ok(GitFileDiff {
            hunks: Vec::new(),
            binary: false,
            too_large: false,
        });
    }
    let line_count = text.split_inclusive('\n').count();
    let mut hunk = format!("@@ -0,0 +1,{line_count} @@\n");
    for line in text.split_inclusive('\n') {
        hunk.push('+');
        hunk.push_str(line);
        if !line.ends_with('\n') {
            hunk.push_str("\n\\ No newline at end of file\n");
        }
    }
    Ok(GitFileDiff {
        hunks: vec![hunk],
        binary: false,
        too_large: false,
    })
}

async fn git_file_diff_at(
    cwd: &Path,
    base_branch: Option<&str>,
    path: &str,
    old_path: Option<&str>,
) -> Result<GitFileDiff, String> {
    validate_git_relative_path(path)?;
    if let Some(old_path) = old_path {
        validate_git_relative_path(old_path)?;
    }

    let comparison = if let Some(base_branch) = base_branch {
        git_branch_comparison_ref(cwd, base_branch).await?
    } else {
        let head = git_ref(cwd, &["rev-parse", "--verify", "HEAD"]).await;
        let tracked = git_ref(cwd, &["ls-files", "--error-unmatch", "--", path]).await;
        if head.is_none() || tracked.is_none() {
            let cwd = cwd.to_owned();
            let path = path.to_owned();
            return tauri::async_runtime::spawn_blocking(move || untracked_file_diff(&cwd, &path))
                .await
                .map_err(|error| format!("Could not join changed-file task: {error}"))?;
        }
        "HEAD".to_owned()
    };

    let mut args = vec![
        "diff",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--find-renames",
        "--relative",
        "--unified=3",
        comparison.as_str(),
        "--",
    ];
    if let Some(old_path) = old_path {
        args.push(old_path);
    }
    args.push(path);
    let output = git_output(cwd, args, 15, "Could not read the file diff").await?;
    Ok(extract_git_hunks(&output.stdout))
}

#[tauri::command]
pub(crate) async fn get_git_file_diff(
    cwd: String,
    base_branch: Option<String>,
    path: String,
    old_path: Option<String>,
) -> Result<GitFileDiff, String> {
    let cwd = validated_git_cwd(cwd).await?;
    git_file_diff_at(&cwd, base_branch.as_deref(), &path, old_path.as_deref()).await
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
pub(crate) async fn list_git_branches(cwd: String) -> Result<Vec<String>, String> {
    let cwd = validated_git_cwd(cwd).await?;
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
pub(crate) async fn switch_git_branch(cwd: String, branch: String) -> Result<(), String> {
    let cwd = validated_git_cwd(cwd).await?;
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
pub(crate) struct GitWorktreeResult {
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
pub(crate) async fn create_git_worktree(
    app: tauri::AppHandle,
    cwd: String,
    base_branch: String,
    branch: String,
) -> Result<GitWorktreeResult, String> {
    let worktrees_root = loopcode_data_directory(&app)?.join("worktrees");
    let cwd = validated_git_cwd(cwd).await?;
    create_git_worktree_at(&cwd, &worktrees_root, &base_branch, &branch).await
}

#[cfg(test)]
mod tests {
    use super::{
        GitChangeStatus, create_git_worktree_at, git_file_diff_at, list_git_branches_at,
        list_git_changes_at, parse_git_numstat, switch_git_branch_at, validate_git_cwd,
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

    fn test_repository(contents: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let directory = tempfile::tempdir().expect("test directory should be created");
        let repository = directory.path().join("repository");
        std::fs::create_dir(&repository).expect("repository directory should be created");
        run_test_git(&repository, &["init", "--initial-branch=main"]);
        run_test_git(&repository, &["config", "user.email", "test@example.com"]);
        run_test_git(&repository, &["config", "user.name", "LoopCode tests"]);
        std::fs::write(repository.join("README.md"), contents)
            .expect("test file should be written");
        run_test_git(&repository, &["add", "README.md"]);
        run_test_git(&repository, &["commit", "-m", "Initial commit"]);
        (directory, repository)
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
    fn git_numstat_accepts_renames_and_binary_files() {
        let stats =
            parse_git_numstat(b"3\t2\tsrc/main.rs\0-\t-\tasset.png\0\x34\t1\t\0old.rs\0new.rs\0")
                .expect("line statistics should parse");
        assert_eq!(stats, (7, 3));
    }

    #[test]
    fn git_branches_can_be_switched_and_used_for_managed_worktrees() {
        let (directory, repository) = test_repository("test\n");
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
    fn change_summaries_skip_untracked_content_and_diffs_remain_renderable() {
        let (_directory, repository) = test_repository("before\n");
        run_test_git(&repository, &["switch", "-c", "feature"]);
        std::fs::write(repository.join("README.md"), "after\n")
            .expect("tracked file should change");
        std::fs::write(repository.join("new.txt"), "new file\n")
            .expect("untracked file should be written");
        std::fs::create_dir(repository.join("nested")).expect("nested directory should be created");
        let noncanonical_repository = repository.join("nested").join("..");

        tauri::async_runtime::block_on(async {
            let changes = list_git_changes_at(&repository, None)
                .await
                .expect("working tree changes should load");
            assert_eq!((changes.additions, changes.deletions), (1, 1));
            assert!(changes.changes.iter().any(|change| {
                change.path == "README.md" && change.status == GitChangeStatus::Modified
            }));
            assert!(changes.changes.iter().any(|change| {
                change.path == "new.txt" && change.status == GitChangeStatus::Untracked
            }));
            let tracked_diff = git_file_diff_at(&repository, None, "README.md", None)
                .await
                .expect("tracked diff should load");
            assert!(tracked_diff.hunks[0].contains("-before\n+after"));
            let untracked_diff = git_file_diff_at(&noncanonical_repository, None, "new.txt", None)
                .await
                .expect("untracked diff should load");
            assert!(untracked_diff.hunks[0].contains("+new file"));

            run_test_git(&repository, &["add", "."]);
            run_test_git(&repository, &["commit", "-m", "Feature change"]);
            let branch_changes = list_git_changes_at(&repository, Some("main"))
                .await
                .expect("branch changes should load");
            assert_eq!((branch_changes.additions, branch_changes.deletions), (2, 1));
            assert!(
                branch_changes
                    .changes
                    .iter()
                    .any(|change| change.path == "README.md")
            );
            let branch_diff = git_file_diff_at(&repository, Some("main"), "README.md", None)
                .await
                .expect("branch diff should load");
            assert!(branch_diff.hunks[0].contains("-before\n+after"));
        });
    }
}
