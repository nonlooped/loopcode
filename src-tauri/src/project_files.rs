use ignore::WalkBuilder;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tauri::{Manager, ipc::Channel};
use tauri_plugin_opener::OpenerExt;

const MAX_PROJECT_DIRECTORY_ENTRIES: usize = 10_000;
const MAX_PROJECT_WATCHERS: usize = 50;
const MAX_SKILL_BYTES: u64 = 1024 * 1024;

#[derive(Default)]
pub struct ProjectFileWatchers {
    next_id: AtomicU64,
    watchers: Mutex<HashMap<u64, RecommendedWatcher>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFileEntry {
    name: String,
    path: String,
    is_directory: bool,
    is_symlink: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerCompletionEntry {
    kind: String,
    name: String,
    path: String,
    relative_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFileChange {
    paths: Vec<String>,
}

fn ensure_watcher_capacity(count: usize) -> Result<(), String> {
    if count >= MAX_PROJECT_WATCHERS {
        Err("Too many project folders are being watched.".to_owned())
    } else {
        Ok(())
    }
}

fn project_file_change(event: Event) -> Option<ProjectFileChange> {
    if event.kind.is_access() {
        return None;
    }
    Some(ProjectFileChange {
        paths: event
            .paths
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    })
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

fn project_directory_entries(
    project_root: &str,
    directory: &str,
) -> Result<Vec<ProjectFileEntry>, String> {
    let directory = resolve_project_path(project_root, directory)?;
    if !directory.is_dir() {
        return Err("The requested project path is not a folder.".to_owned());
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&directory)
        .map_err(|error| format!("Could not read project folder: {error}"))?
    {
        if entries.len() == MAX_PROJECT_DIRECTORY_ENTRIES {
            return Err("Project folders cannot contain more than 10,000 entries.".to_owned());
        }
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
}

fn normalized_relative_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

const MAX_COMPOSER_PROJECT_ENTRIES: usize = 10_000;

fn composer_project_entries(project_root: &Path) -> Result<Vec<ComposerCompletionEntry>, String> {
    let root = project_root
        .canonicalize()
        .map_err(|error| format!("Could not resolve project folder: {error}"))?;
    if !root.is_dir() {
        return Err("The project path is not a folder.".to_owned());
    }

    let mut entries = Vec::new();
    let walker = WalkBuilder::new(&root)
        .hidden(false)
        .require_git(false)
        .follow_links(false)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build();
    for entry in walker.filter_map(Result::ok).skip(1) {
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() && !file_type.is_dir() {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(&root) else {
            continue;
        };
        entries.push(ComposerCompletionEntry {
            kind: if file_type.is_dir() { "folder" } else { "file" }.to_owned(),
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path().to_string_lossy().into_owned(),
            relative_path: normalized_relative_path(relative),
            description: None,
        });
        if entries.len() == MAX_COMPOSER_PROJECT_ENTRIES {
            break;
        }
    }
    Ok(entries)
}

fn frontmatter_value(contents: &str, key: &str) -> Option<String> {
    let mut lines = contents.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    for line in lines {
        let line = line.trim();
        if line == "---" {
            break;
        }
        let Some(value) = line
            .strip_prefix(key)
            .and_then(|value| value.strip_prefix(':'))
        else {
            continue;
        };
        let value = value.trim().trim_matches(['"', '\'']).trim();
        if !value.is_empty() && value != ">" && value != "|" {
            return Some(value.to_owned());
        }
    }
    None
}

fn collect_skill_entries(
    root: &Path,
    display_root: &str,
    seen_names: &mut HashSet<String>,
    entries: &mut Vec<ComposerCompletionEntry>,
) {
    if !root.is_dir() {
        return;
    }
    let mut directories = vec![root.to_owned()];
    while let Some(directory) = directories.pop() {
        let Ok(children) = std::fs::read_dir(directory) else {
            continue;
        };
        for child in children.filter_map(Result::ok) {
            let Ok(file_type) = child.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                directories.push(child.path());
                continue;
            }
            if child.file_name() != "SKILL.md" || !file_type.is_file() {
                continue;
            }
            let Ok(metadata) = child.metadata() else {
                continue;
            };
            if metadata.len() > MAX_SKILL_BYTES {
                continue;
            }
            let Ok(contents) = std::fs::read_to_string(child.path()) else {
                continue;
            };
            if contents.len() as u64 > MAX_SKILL_BYTES {
                continue;
            }
            let Some(name) = frontmatter_value(&contents, "name") else {
                continue;
            };
            if !seen_names.insert(name.to_lowercase()) {
                continue;
            }
            let relative = child
                .path()
                .strip_prefix(root)
                .ok()
                .map(normalized_relative_path);
            entries.push(ComposerCompletionEntry {
                kind: "skill".to_owned(),
                name,
                path: child.path().to_string_lossy().into_owned(),
                relative_path: relative
                    .map(|path| format!("{display_root}/{path}"))
                    .unwrap_or_else(|| display_root.to_owned()),
                description: frontmatter_value(&contents, "description"),
            });
        }
    }
}

fn composer_entries(
    home: &Path,
    project_root: &Path,
) -> Result<Vec<ComposerCompletionEntry>, String> {
    let mut entries = composer_project_entries(project_root)?;
    let mut seen_names = HashSet::new();
    for (root, display_root) in [
        (project_root.join(".agents/skills"), ".agents/skills"),
        (project_root.join(".claude/skills"), ".claude/skills"),
        (home.join(".agents/skills"), "~/.agents/skills"),
        (home.join(".claude/skills"), "~/.claude/skills"),
    ] {
        collect_skill_entries(&root, display_root, &mut seen_names, &mut entries);
    }
    Ok(entries)
}

#[tauri::command]
pub async fn list_composer_completions(
    app: tauri::AppHandle,
    project_root: String,
) -> Result<Vec<ComposerCompletionEntry>, String> {
    let home = app
        .path()
        .home_dir()
        .map_err(|error| format!("Could not resolve the user home directory: {error}"))?;
    tauri::async_runtime::spawn_blocking(move || composer_entries(&home, Path::new(&project_root)))
        .await
        .map_err(|error| format!("Could not join composer completion task: {error}"))?
}

#[tauri::command]
pub async fn read_project_directory(
    project_root: String,
    directory: String,
) -> Result<Vec<ProjectFileEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        project_directory_entries(&project_root, &directory)
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
pub async fn read_project_file(
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
pub async fn open_project_path(
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
pub async fn reveal_project_path(
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
pub fn start_project_file_watcher(
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
    let mut watchers = state
        .watchers
        .lock()
        .map_err(|_| "The project watcher lock was poisoned.".to_owned())?;
    ensure_watcher_capacity(watchers.len())?;

    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        let Ok(event) = result else { return };
        let Some(change) = project_file_change(event) else {
            return;
        };
        let _ = on_change.send(change);
    })
    .map_err(|error| format!("Could not create project watcher: {error}"))?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|error| format!("Could not watch project folder: {error}"))?;

    let watcher_id = state.next_id.fetch_add(1, Ordering::Relaxed) + 1;
    watchers.insert(watcher_id, watcher);
    Ok(watcher_id)
}

#[tauri::command]
pub fn stop_project_file_watcher(
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

#[cfg(test)]
mod tests {
    use super::{
        MAX_PROJECT_WATCHERS, composer_project_entries, ensure_watcher_capacity, frontmatter_value,
        project_directory_entries, project_file_change, project_image_media_type,
        resolve_project_path,
    };
    use notify::{Event, EventKind, event::AccessKind};
    use std::{fs, path::Path};

    #[test]
    fn caps_project_watchers() {
        assert!(ensure_watcher_capacity(MAX_PROJECT_WATCHERS - 1).is_ok());
        assert!(ensure_watcher_capacity(MAX_PROJECT_WATCHERS).is_err());
    }

    #[test]
    fn ignores_non_mutating_file_access_events() {
        assert!(project_file_change(Event::new(EventKind::Access(AccessKind::Read))).is_none());
    }

    #[test]
    fn rejects_paths_outside_the_project() {
        let directory = tempfile::tempdir().expect("test directory should be created");
        let project = directory.path().join("project");
        let outside = directory.path().join("outside.txt");
        fs::create_dir(&project).expect("project should be created");
        fs::write(&outside, "outside").expect("outside file should be created");

        assert_eq!(
            resolve_project_path(&project.to_string_lossy(), &outside.to_string_lossy()),
            Err("The requested path is outside the project folder.".to_owned())
        );
    }

    #[test]
    fn orders_directories_before_files_by_name() {
        let directory = tempfile::tempdir().expect("test directory should be created");
        fs::create_dir(directory.path().join("z-folder")).expect("folder should be created");
        fs::create_dir(directory.path().join("A-folder")).expect("folder should be created");
        fs::write(directory.path().join("z.txt"), "z").expect("file should be created");
        fs::write(directory.path().join("A.txt"), "a").expect("file should be created");

        let path = directory.path().to_string_lossy();
        let names = project_directory_entries(&path, &path)
            .expect("directory should be read")
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();

        assert_eq!(names, ["A-folder", "z-folder", "A.txt", "z.txt"]);
    }

    #[test]
    fn composer_entries_honor_gitignore_without_git_repository() {
        let directory = tempfile::tempdir().expect("test directory should be created");
        fs::create_dir(directory.path().join("ignored"))
            .expect("ignored directory should be created");
        fs::write(directory.path().join(".gitignore"), "ignored/\n")
            .expect("gitignore should be written");
        fs::write(directory.path().join("visible.txt"), "visible")
            .expect("visible file should be written");
        fs::write(directory.path().join("ignored/hidden.txt"), "hidden")
            .expect("ignored file should be written");

        let paths = composer_project_entries(directory.path())
            .expect("project entries should load")
            .into_iter()
            .map(|entry| entry.relative_path)
            .collect::<Vec<_>>();

        assert!(paths.contains(&"visible.txt".to_owned()));
        assert!(!paths.iter().any(|path| path.starts_with("ignored")));
    }

    #[test]
    fn composer_entries_omit_missing_descriptions() {
        let directory = tempfile::tempdir().expect("test directory should be created");
        fs::write(directory.path().join("visible.txt"), "visible")
            .expect("visible file should be written");

        let entries =
            composer_project_entries(directory.path()).expect("project entries should load");
        let serialized = serde_json::to_value(entries).expect("entries should serialize");

        assert!(serialized[0].get("description").is_none());
    }

    #[test]
    fn reads_skill_frontmatter() {
        let skill = "---\nname: grill-me\ndescription: Ask hard questions\n---\n";
        assert_eq!(
            frontmatter_value(skill, "name").as_deref(),
            Some("grill-me")
        );
        assert_eq!(
            frontmatter_value(skill, "description").as_deref(),
            Some("Ask hard questions")
        );
    }

    #[test]
    fn recognizes_previewable_image_extensions() {
        assert_eq!(
            project_image_media_type(Path::new("image.PNG")),
            Some("image/png")
        );
        assert_eq!(project_image_media_type(Path::new("component.ts")), None);
    }
}
