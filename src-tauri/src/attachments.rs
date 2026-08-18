use serde_json::Value;
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::{Duration, SystemTime},
};

const ATTACHMENTS_DIRECTORY: &str = "attachments";
const MAX_ATTACHMENT_BYTES: usize = 20 * 1024 * 1024;
const ORPHAN_GRACE: Duration = Duration::from_secs(24 * 60 * 60);
const SNAPSHOT_NAMES: [&str; 3] = ["threads.json", "threads.json.bak", "threads.json.tmp"];

fn valid_attachment_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn attachment_path(directory: &Path, id: &str) -> Result<std::path::PathBuf, String> {
    if !valid_attachment_id(id) {
        return Err("Attachment IDs must be UUID-like values.".to_owned());
    }
    Ok(directory.join(ATTACHMENTS_DIRECTORY).join(id))
}

pub fn store_in_directory(directory: &Path, id: &str, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_ATTACHMENT_BYTES {
        return Err("Attachments cannot be larger than 20 MiB.".to_owned());
    }
    let target = attachment_path(directory, id)?;
    let attachments = target
        .parent()
        .expect("attachment path should have a parent directory");
    fs::create_dir_all(attachments).map_err(|error| {
        format!(
            "Could not create attachment directory {}: {error}",
            attachments.display()
        )
    })?;
    if target.exists() {
        return Err("An attachment with this ID already exists.".to_owned());
    }

    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = attachments.join(format!(".{id}.{}-{nonce}.tmp", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| format!("Could not create attachment temporary file: {error}"))?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("Could not write attachment bytes: {error}"))?;
        fs::rename(&temporary, &target)
            .map_err(|error| format!("Could not finalize attachment: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub fn read_from_directory(directory: &Path, id: &str) -> Result<Vec<u8>, String> {
    let path = attachment_path(directory, id)?;
    let metadata = path
        .metadata()
        .map_err(|error| format!("Could not inspect attachment: {error}"))?;
    if !metadata.is_file() {
        return Err("The attachment is not a regular file.".to_owned());
    }
    if metadata.len() > MAX_ATTACHMENT_BYTES as u64 {
        return Err("The attachment exceeds the 20 MiB limit.".to_owned());
    }
    let bytes = fs::read(path).map_err(|error| format!("Could not read attachment: {error}"))?;
    if bytes.len() > MAX_ATTACHMENT_BYTES {
        return Err("The attachment grew beyond the 20 MiB limit while being read.".to_owned());
    }
    Ok(bytes)
}

pub fn delete_from_directory(directory: &Path, id: &str) -> Result<(), String> {
    let path = attachment_path(directory, id)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Could not delete attachment: {error}")),
    }
}

fn collect_references(value: &Value, protected: &mut HashSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(id)) = object.get("attachmentId")
                && valid_attachment_id(id)
            {
                protected.insert(id.clone());
            }
            for child in object.values() {
                collect_references(child, protected);
            }
        }
        Value::Array(array) => {
            for child in array {
                collect_references(child, protected);
            }
        }
        _ => {}
    }
}

pub fn cleanup_in_directory(
    directory: &Path,
    pending_ids: &[String],
    now: SystemTime,
) -> Result<usize, String> {
    let mut protected = HashSet::new();
    for id in pending_ids {
        if !valid_attachment_id(id) {
            return Err("Attachment IDs must be UUID-like values.".to_owned());
        }
        protected.insert(id.clone());
    }

    for name in SNAPSHOT_NAMES {
        let path = directory.join(name);
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            // Conservative cleanup: an unreadable snapshot may still own every file.
            Err(_) => return Ok(0),
        };
        let snapshot: Value = match serde_json::from_str(&contents) {
            Ok(snapshot) => snapshot,
            Err(_) => return Ok(0),
        };
        collect_references(&snapshot, &mut protected);
    }

    let attachments = directory.join(ATTACHMENTS_DIRECTORY);
    let entries = match fs::read_dir(&attachments) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(format!("Could not inspect attachment directory: {error}")),
    };
    let mut deleted = 0;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("Could not inspect attachment entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Could not inspect attachment type: {error}"))?;
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if protected.contains(&name) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .map_err(|error| format!("Could not inspect attachment age: {error}"))?;
        if now.duration_since(modified).unwrap_or_default() <= ORPHAN_GRACE {
            continue;
        }
        fs::remove_file(entry.path())
            .map_err(|error| format!("Could not remove orphan attachment: {error}"))?;
        deleted += 1;
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const ID: &str = "123e4567-e89b-12d3-a456-426614174000";
    const OTHER_ID: &str = "123e4567-e89b-12d3-a456-426614174001";

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                "loopcode-attachment-test-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("test clock")
                    .as_nanos()
            )))
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn atomic_roundtrip_and_validation() {
        let directory = TestDirectory::new();
        store_in_directory(&directory.0, ID, b"raw bytes").expect("store attachment");
        assert_eq!(
            read_from_directory(&directory.0, ID).expect("read attachment"),
            b"raw bytes"
        );
        assert!(store_in_directory(&directory.0, "../threads.json", b"bad").is_err());
        assert!(read_from_directory(&directory.0, "not-an-id").is_err());
        assert!(
            store_in_directory(&directory.0, OTHER_ID, &vec![0; MAX_ATTACHMENT_BYTES + 1]).is_err()
        );
        delete_from_directory(&directory.0, ID).expect("delete attachment");
        assert!(!directory.0.join(ATTACHMENTS_DIRECTORY).join(ID).exists());
    }

    #[test]
    fn cleanup_protects_all_snapshots_and_pending_ids() {
        let directory = TestDirectory::new();
        let backup_id = OTHER_ID;
        let temporary_id = "123e4567-e89b-12d3-a456-426614174002";
        let pending = "123e4567-e89b-12d3-a456-426614174003".to_owned();
        let orphan = "123e4567-e89b-12d3-a456-426614174004";
        for id in [ID, backup_id, temporary_id, &pending, orphan] {
            store_in_directory(&directory.0, id, b"bytes").expect("store fixture");
        }
        fs::create_dir_all(&directory.0).expect("create snapshot directory");
        for (name, id) in [
            ("threads.json", ID),
            ("threads.json.bak", backup_id),
            ("threads.json.tmp", temporary_id),
        ] {
            fs::write(
                directory.0.join(name),
                format!(r#"{{"attachmentId":"{id}"}}"#),
            )
            .expect("write snapshot");
        }

        let future = SystemTime::now() + ORPHAN_GRACE + Duration::from_secs(1);
        assert_eq!(
            cleanup_in_directory(&directory.0, std::slice::from_ref(&pending), future)
                .expect("cleanup"),
            1
        );
        for id in [ID, backup_id, temporary_id, &pending] {
            assert!(directory.0.join(ATTACHMENTS_DIRECTORY).join(id).exists());
        }
        assert!(
            !directory
                .0
                .join(ATTACHMENTS_DIRECTORY)
                .join(orphan)
                .exists()
        );
    }

    #[test]
    fn cleanup_preserves_recent_orphans() {
        let directory = TestDirectory::new();
        store_in_directory(&directory.0, ID, b"bytes").expect("store fixture");
        assert_eq!(
            cleanup_in_directory(&directory.0, &[], SystemTime::now()).expect("cleanup"),
            0
        );
        assert!(directory.0.join(ATTACHMENTS_DIRECTORY).join(ID).exists());
    }

    #[test]
    fn corrupt_snapshot_skips_cleanup() {
        let directory = TestDirectory::new();
        store_in_directory(&directory.0, ID, b"bytes").expect("store fixture");
        fs::write(directory.0.join("threads.json.tmp"), "not json")
            .expect("write corrupt snapshot");
        let future = SystemTime::now() + ORPHAN_GRACE + Duration::from_secs(1);
        assert_eq!(
            cleanup_in_directory(&directory.0, &[], future).expect("cleanup"),
            0
        );
        assert!(directory.0.join(ATTACHMENTS_DIRECTORY).join(ID).exists());
    }
}
