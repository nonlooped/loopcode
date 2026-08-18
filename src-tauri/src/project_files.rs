use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tauri::ipc::Channel;
use tauri_plugin_opener::OpenerExt;

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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFileChange {
    paths: Vec<String>,
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
pub async fn open_project_file(
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
    use super::{project_directory_entries, project_image_media_type, resolve_project_path};
    use std::{fs, path::Path, path::PathBuf};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "loopcode-project-files-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock should be after the Unix epoch")
                    .as_nanos()
            ));
            fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn rejects_paths_outside_the_project() {
        let directory = TestDirectory::new();
        let project = directory.0.join("project");
        let outside = directory.0.join("outside.txt");
        fs::create_dir(&project).expect("project should be created");
        fs::write(&outside, "outside").expect("outside file should be created");

        assert_eq!(
            resolve_project_path(&project.to_string_lossy(), &outside.to_string_lossy()),
            Err("The requested path is outside the project folder.".to_owned())
        );
    }

    #[test]
    fn orders_directories_before_files_by_name() {
        let directory = TestDirectory::new();
        fs::create_dir(directory.0.join("z-folder")).expect("folder should be created");
        fs::create_dir(directory.0.join("A-folder")).expect("folder should be created");
        fs::write(directory.0.join("z.txt"), "z").expect("file should be created");
        fs::write(directory.0.join("A.txt"), "a").expect("file should be created");

        let path = directory.0.to_string_lossy();
        let names = project_directory_entries(&path, &path)
            .expect("directory should be read")
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();

        assert_eq!(names, ["A-folder", "z-folder", "A.txt", "z.txt"]);
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
