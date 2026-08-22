use std::{
    fs,
    io::{self, Write},
    path::Path,
    sync::Mutex,
};

use atomic_write_file::AtomicWriteFile;

pub const THREADS_FILE_NAME: &str = "threads.json";
const BACKUP_FILE_NAME: &str = "threads.json.bak";
static SAVE_LOCK: Mutex<()> = Mutex::new(());

pub fn load_from_directory<T>(
    directory: &Path,
    parse: impl Fn(&str) -> Result<T, String>,
) -> Result<Option<T>, String> {
    let candidates = [
        directory.join(THREADS_FILE_NAME),
        directory.join(BACKUP_FILE_NAME),
    ];
    let mut failures = Vec::new();
    let mut found_file = false;

    for path in candidates {
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => {
                found_file = true;
                contents
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                found_file = true;
                failures.push(format!("Could not read {}: {error}", path.display()));
                continue;
            }
        };

        match parse(&contents) {
            Ok(workspace) => return Ok(Some(workspace)),
            Err(error) => failures.push(format!("Could not parse {}: {error}", path.display())),
        }
    }

    if found_file {
        Err(failures.join("; "))
    } else {
        Ok(None)
    }
}

pub fn save_to_directory(directory: &Path, workspace_json: &str) -> Result<(), String> {
    let _save = SAVE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    fs::create_dir_all(directory).map_err(|error| {
        format!(
            "Could not create the LoopCode data directory {}: {error}",
            directory.display()
        )
    })?;

    let target = directory.join(THREADS_FILE_NAME);
    let backup = directory.join(BACKUP_FILE_NAME);

    let mut file = AtomicWriteFile::open(&target)
        .map_err(|error| format!("Could not open {}: {error}", target.display()))?;
    file.write_all(workspace_json.as_bytes())
        .and_then(|()| file.write_all(b"\n"))
        .map_err(|error| format!("Could not write {}: {error}", target.display()))?;

    if target.exists() {
        let mut previous = fs::File::open(&target)
            .map_err(|error| format!("Could not read {}: {error}", target.display()))?;
        let mut backup_file = AtomicWriteFile::open(&backup)
            .map_err(|error| format!("Could not open {}: {error}", backup.display()))?;
        io::copy(&mut previous, &mut backup_file)
            .map_err(|error| format!("Could not write {}: {error}", backup.display()))?;
        backup_file.commit().map_err(|error| {
            format!("Could not publish {} atomically: {error}", backup.display())
        })?;
    }

    file.commit()
        .map_err(|error| format!("Could not publish {} atomically: {error}", target.display()))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{BACKUP_FILE_NAME, THREADS_FILE_NAME, load_from_directory, save_to_directory};

    fn load(directory: &Path) -> Result<Option<String>, String> {
        load_from_directory(directory, |contents| {
            let trimmed = contents.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                Ok(trimmed.to_owned())
            } else {
                Err("invalid JSON fixture".to_owned())
            }
        })
    }

    #[test]
    fn round_trips_workspace_json() {
        let directory = tempfile::tempdir().expect("test directory should be created");
        let workspace = r#"{"version":1,"selectedThreadId":"thread-1","threads":[{"id":"thread-1","messages":[{"text":"hello"}]}]}"#;

        save_to_directory(&directory.path(), workspace).expect("workspace should save");

        assert_eq!(
            load(&directory.path()).expect("workspace should load"),
            Some(workspace.to_owned())
        );
        assert!(directory.path().join(THREADS_FILE_NAME).is_file());
    }

    #[test]
    fn a_second_save_replaces_the_previous_snapshot() {
        let directory = tempfile::tempdir().expect("test directory should be created");
        save_to_directory(&directory.path(), r#"{"version":1,"threads":[]}"#)
            .expect("first workspace should save");
        let latest = r#"{"version":1,"threads":[{"id":"latest"}]}"#;

        save_to_directory(&directory.path(), latest).expect("latest workspace should save");

        assert_eq!(
            load(&directory.path()).expect("workspace should load"),
            Some(latest.to_owned())
        );
        let leftovers: Vec<_> = fs::read_dir(&directory.path())
            .expect("directory should list")
            .map(|entry| entry.expect("entry should read").file_name())
            .filter(|name| name != THREADS_FILE_NAME && name != BACKUP_FILE_NAME)
            .collect();
        assert!(leftovers.is_empty(), "stray files: {leftovers:?}");
    }

    #[test]
    fn concurrent_saves_leave_valid_primary_and_backup_files() {
        let directory = tempfile::tempdir().expect("test directory should be created");
        save_to_directory(directory.path(), r#"{"seed":true}"#).expect("seed should save");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let saves: Vec<_> = (0..8)
            .map(|index| {
                let path = directory.path().to_owned();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    save_to_directory(&path, &format!(r#"{{"save":{index}}}"#))
                })
            })
            .collect();

        for save in saves {
            save.join()
                .expect("save thread should finish")
                .expect("concurrent save should succeed");
        }
        for name in [THREADS_FILE_NAME, BACKUP_FILE_NAME] {
            let contents = fs::read_to_string(directory.path().join(name))
                .expect("snapshot should remain readable");
            assert!(
                contents.trim().starts_with("{\"save\":"),
                "invalid {name}: {contents}"
            );
        }
    }

    #[test]
    fn loads_the_backup_when_the_primary_file_is_corrupt() {
        let directory = tempfile::tempdir().expect("test directory should be created");
        let workspace = r#"{"version":1,"threads":[{"id":"recoverable"}]}"#;
        save_to_directory(&directory.path(), workspace).expect("workspace should save");
        fs::write(directory.path().join(BACKUP_FILE_NAME), workspace)
            .expect("backup fixture should write");
        fs::write(directory.path().join(THREADS_FILE_NAME), "{not-json")
            .expect("primary fixture should corrupt");

        assert_eq!(
            load(&directory.path()).expect("backup should load"),
            Some(workspace.to_owned())
        );
    }
}
