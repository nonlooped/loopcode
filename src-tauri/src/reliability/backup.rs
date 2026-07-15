//! SQLite integrity check + safe backup (VACUUM INTO), not naive hot WAL copy.

use crate::db::migrate::{configure_connection, ensure_schema_not_newer, migrate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub ok: bool,
    pub detail: String,
}

/// Run `PRAGMA integrity_check`. Fail closed if not `ok`.
pub fn integrity_check(conn: &Connection) -> Result<IntegrityReport, String> {
    let detail: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| format!("integrity_check: {e}"))?;
    let ok = detail.eq_ignore_ascii_case("ok");
    if !ok {
        return Err(format!(
            "database integrity failure (fail-closed, no silent wipe): {detail}"
        ));
    }
    Ok(IntegrityReport {
        ok: true,
        detail,
    })
}

/// Open file DB, configure WAL, integrity-check, then migrate.
/// Integrity failure fails closed — never wipes the file.
pub fn open_with_integrity(db_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(db_path).map_err(|e| format!("open sqlite: {e}"))?;
    ensure_schema_not_newer(&conn)?;
    configure_connection(&conn)?;
    integrity_check(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

/// Safe backup via `VACUUM INTO` into `backup_path` (must not be the live DB path).
/// Never copies live WAL siblings naively.
pub fn backup_database(live_db: &Path, backup_path: &Path) -> Result<PathBuf, String> {
    if live_db == backup_path {
        return Err("backup path must differ from live database".into());
    }
    if let Some(parent) = backup_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if backup_path.exists() {
        fs::remove_file(backup_path).map_err(|e| format!("remove old backup: {e}"))?;
    }
    // Open live, VACUUM INTO destination (consistent snapshot).
    let conn = Connection::open(live_db).map_err(|e| format!("open live: {e}"))?;
    configure_connection(&conn)?;
    integrity_check(&conn)?;
    let dest = backup_path
        .to_str()
        .ok_or("backup path not utf-8")?
        .replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{dest}';"))
        .map_err(|e| format!("VACUUM INTO backup: {e}"))?;
    if !backup_path.is_file() {
        return Err("backup file missing after VACUUM INTO".into());
    }
    // Verify backup is not a WAL hot-copy sibling scheme
    let wal_sibling = format!("{}-wal", live_db.display());
    if backup_path.file_name() == Path::new(&wal_sibling).file_name() {
        return Err("refusing WAL-sibling style backup name".into());
    }
    Ok(backup_path.to_path_buf())
}

/// Open a backup as a new connection (salvage/restore source). Fail closed if corrupt.
pub fn restore_from_backup(backup_path: &Path, target_path: &Path) -> Result<(), String> {
    if !backup_path.is_file() {
        return Err(format!("backup not found: {}", backup_path.display()));
    }
    let src = Connection::open(backup_path).map_err(|e| format!("open backup: {e}"))?;
    integrity_check(&src)?;
    drop(src);

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Replace target with a fresh VACUUM INTO from backup (safe API path).
    if target_path.exists() {
        fs::remove_file(target_path).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(backup_path).map_err(|e| e.to_string())?;
    let dest = target_path
        .to_str()
        .ok_or("target path not utf-8")?
        .replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{dest}';"))
        .map_err(|e| format!("restore VACUUM INTO: {e}"))?;
    // Verify restored
    let restored = Connection::open(target_path).map_err(|e| e.to_string())?;
    integrity_check(&restored)?;
    Ok(())
}

/// Default backups directory next to data dir: `{data_dir}/backups`.
pub fn default_backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::store::Database;

    fn temp_db(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lc-bak-{label}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir.join("app.sqlite")
    }

    #[test]
    fn integrity_backup_restore_roundtrip() {
        let live = temp_db("live");
        let db = Database::open(&live).unwrap();
        let project = db.open_project(live.parent().unwrap()).unwrap();
        let chat = db.create_chat(&project.id, Some("c")).unwrap();
        db.append_event(&chat.id, None, "user_message", r#"{"text":"hi"}"#)
            .unwrap();
        drop(db);

        let bak = live.parent().unwrap().join("backups").join("snap.sqlite");
        backup_database(&live, &bak).unwrap();
        assert!(bak.is_file());
        // Not a naive -wal copy name
        assert!(!bak.to_string_lossy().ends_with("-wal"));

        let restored = live.parent().unwrap().join("restored.sqlite");
        restore_from_backup(&bak, &restored).unwrap();
        let db2 = Database::open(&restored).unwrap();
        let events = db2.list_events(&chat.id, None).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn integrity_fail_closed_on_garbage_file() {
        let path = temp_db("bad");
        fs::write(&path, b"not a sqlite database at all!!!!!").unwrap();
        let err = open_with_integrity(&path).unwrap_err();
        assert!(
            err.contains("integrity") || err.contains("sqlite") || err.contains("file is not"),
            "{err}"
        );
        // Original garbage still present (no silent wipe)
        assert!(path.is_file());
    }
}
