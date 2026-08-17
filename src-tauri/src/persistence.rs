use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

pub const THREADS_FILE_NAME: &str = "threads.json";
const BACKUP_FILE_NAME: &str = "threads.json.bak";
const TEMP_FILE_NAME: &str = "threads.json.tmp";

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
    fs::create_dir_all(directory).map_err(|error| {
        format!(
            "Could not create the LoopCode data directory {}: {error}",
            directory.display()
        )
    })?;

    let target = directory.join(THREADS_FILE_NAME);
    let backup = directory.join(BACKUP_FILE_NAME);
    let temporary = directory.join(TEMP_FILE_NAME);

    write_temporary_file(&temporary, workspace_json)?;
    if backup.exists() {
        fs::remove_file(&backup)
            .map_err(|error| format!("Could not replace backup {}: {error}", backup.display()))?;
    }
    if target.exists() {
        fs::rename(&target, &backup).map_err(|error| {
            format!(
                "Could not rotate {} to {}: {error}",
                target.display(),
                backup.display()
            )
        })?;
    }

    if let Err(error) = fs::rename(&temporary, &target) {
        restore_backup(&backup, &target);
        return Err(format!(
            "Could not move {} to {}: {error}",
            temporary.display(),
            target.display()
        ));
    }

    Ok(())
}

fn write_temporary_file(path: &Path, workspace_json: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("Could not open {}: {error}", path.display()))?;
    file.write_all(workspace_json.as_bytes())
        .and_then(|()| file.write_all(b"\n"))
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("Could not write {}: {error}", path.display()))
}

fn restore_backup(backup: &Path, target: &Path) {
    if backup.exists() && !target.exists() {
        let _ = fs::rename(backup, target);
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{BACKUP_FILE_NAME, THREADS_FILE_NAME, load_from_directory, save_to_directory};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "loopcode-persistence-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock should be after the Unix epoch")
                    .as_nanos()
            ));
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn load(directory: &PathBuf) -> Result<Option<String>, String> {
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
        let directory = TestDirectory::new();
        let workspace = r#"{"version":1,"selectedThreadId":"thread-1","threads":[{"id":"thread-1","messages":[{"text":"hello"}]}]}"#;

        save_to_directory(&directory.0, workspace).expect("workspace should save");

        assert_eq!(
            load(&directory.0).expect("workspace should load"),
            Some(workspace.to_owned())
        );
        assert!(directory.0.join(THREADS_FILE_NAME).is_file());
    }

    #[test]
    fn a_second_save_replaces_the_previous_snapshot() {
        let directory = TestDirectory::new();
        save_to_directory(&directory.0, r#"{"version":1,"threads":[]}"#)
            .expect("first workspace should save");
        let latest = r#"{"version":1,"threads":[{"id":"latest"}]}"#;

        save_to_directory(&directory.0, latest).expect("latest workspace should save");

        assert_eq!(
            load(&directory.0).expect("workspace should load"),
            Some(latest.to_owned())
        );
    }

    #[test]
    fn loads_the_backup_when_the_primary_file_is_corrupt() {
        let directory = TestDirectory::new();
        let workspace = r#"{"version":1,"threads":[{"id":"recoverable"}]}"#;
        save_to_directory(&directory.0, workspace).expect("workspace should save");
        fs::copy(
            directory.0.join(THREADS_FILE_NAME),
            directory.0.join(BACKUP_FILE_NAME),
        )
        .expect("backup fixture should copy");
        fs::write(directory.0.join(THREADS_FILE_NAME), "{not-json")
            .expect("primary fixture should corrupt");

        assert_eq!(
            load(&directory.0).expect("backup should load"),
            Some(workspace.to_owned())
        );
    }
}
