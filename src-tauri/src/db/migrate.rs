use crate::domain::SCHEMA_VERSION;
use rusqlite::{Connection, OptionalExtension};

/// Apply migrations up to [`SCHEMA_VERSION`]. Fail closed on unknown future versions.
pub fn migrate(conn: &Connection) -> Result<(), String> {
    let current = ensure_schema_not_newer(conn)?;

    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| format!("begin migration transaction: {e}"))?;
    let result = (|| {
        if current < 1 {
            apply_v1(conn)?;
            set_user_version(conn, 1)?;
        }

        if current < 2 {
            apply_v2(conn)?;
            set_user_version(conn, 2)?;
        }

        let after: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|e| format!("verify user_version: {e}"))?;
        if after != SCHEMA_VERSION {
            return Err(format!(
                "migration incomplete: expected user_version={SCHEMA_VERSION}, got {after}"
            ));
        }
        Ok(())
    })();

    match result {
        Ok(()) => conn.execute_batch("COMMIT").map_err(|e| format!("commit migrations: {e}")),
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

/// Check the version before any configuration pragma that could mutate a newer
/// database file.
pub fn ensure_schema_not_newer(conn: &Connection) -> Result<u32, String> {
    let current: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| format!("read user_version: {e}"))?;
    if current > SCHEMA_VERSION {
        return Err(format!(
            "database schema version {current} is newer than this app supports ({SCHEMA_VERSION}); refuse to open"
        ));
    }
    Ok(current)
}

fn set_user_version(conn: &Connection, v: u32) -> Result<(), String> {
    conn.pragma_update(None, "user_version", v)
        .map_err(|e| format!("set user_version: {e}"))
}

fn apply_v1(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY NOT NULL,
            root_path TEXT NOT NULL UNIQUE,
            resolved_path TEXT NOT NULL,
            display_path TEXT NOT NULL,
            orphaned INTEGER NOT NULL DEFAULT 0 CHECK (orphaned IN (0, 1)),
            last_active_chat_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS chats (
            id TEXT PRIMARY KEY NOT NULL,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_chats_project ON chats(project_id);

        CREATE TABLE IF NOT EXISTS runs (
            id TEXT PRIMARY KEY NOT NULL,
            chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
            mode TEXT NOT NULL,
            model TEXT,
            adapter TEXT,
            policy_version TEXT,
            status TEXT NOT NULL,
            predecessor_run_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            finished_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_runs_chat ON runs(chat_id);

        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY NOT NULL,
            chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
            run_id TEXT REFERENCES runs(id) ON DELETE SET NULL,
            seq INTEGER NOT NULL,
            kind TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE (chat_id, seq)
        );
        CREATE INDEX IF NOT EXISTS idx_events_chat_seq ON events(chat_id, seq);

        CREATE TABLE IF NOT EXISTS artifacts (
            id TEXT PRIMARY KEY NOT NULL,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            chat_id TEXT REFERENCES chats(id) ON DELETE SET NULL,
            run_id TEXT REFERENCES runs(id) ON DELETE SET NULL,
            kind TEXT NOT NULL,
            title TEXT NOT NULL,
            body_inline TEXT,
            body_path TEXT,
            version INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_artifacts_project ON artifacts(project_id);

        CREATE TABLE IF NOT EXISTS usage_ledger (
            id TEXT PRIMARY KEY NOT NULL,
            run_id TEXT REFERENCES runs(id) ON DELETE SET NULL,
            chat_id TEXT REFERENCES chats(id) ON DELETE SET NULL,
            provider TEXT,
            model TEXT,
            input_tokens INTEGER,
            output_tokens INTEGER,
            cost_micros INTEGER,
            elapsed_ms INTEGER,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_usage_run ON usage_ledger(run_id);

        CREATE TABLE IF NOT EXISTS settings (
            scope TEXT NOT NULL,
            key TEXT NOT NULL,
            value_json TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (scope, key)
        );

        CREATE TABLE IF NOT EXISTS secret_refs (
            id TEXT PRIMARY KEY NOT NULL,
            kind TEXT NOT NULL,
            label TEXT NOT NULL,
            store_key TEXT NOT NULL,
            provider TEXT,
            project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )
    .map_err(|e| format!("apply schema v1: {e}"))?;
    Ok(())
}

fn apply_v2(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS audit_log (
            id TEXT PRIMARY KEY NOT NULL,
            kind TEXT NOT NULL,
            actor TEXT NOT NULL,
            run_id TEXT,
            chat_id TEXT,
            project_id TEXT,
            payload_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_log(created_at);

        CREATE TABLE IF NOT EXISTS trust_records (
            id TEXT PRIMARY KEY NOT NULL,
            kind TEXT NOT NULL,
            subject_key TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            project_id TEXT,
            trusted_at TEXT NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_trust_subject
            ON trust_records(kind, subject_key, IFNULL(project_id, ''));
        "#,
    )
    .map_err(|e| format!("apply schema v2: {e}"))?;
    Ok(())
}

/// Configure WAL and busy timeout on a live connection.
///
/// File-backed databases must use WAL. In-memory databases report
/// `journal_mode=memory` (SQLite cannot WAL a pure memory DB); that is accepted
/// only for tests via [`super::store::Database::open_in_memory`].
pub fn configure_connection(conn: &Connection) -> Result<(), String> {
    configure_connection_inner(conn, false)
}

pub fn configure_connection_allow_memory(conn: &Connection) -> Result<(), String> {
    configure_connection_inner(conn, true)
}

fn configure_connection_inner(conn: &Connection, allow_memory: bool) -> Result<(), String> {
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("enable WAL: {e}"))?;
    conn.pragma_update(None, "foreign_keys", true)
        .map_err(|e| format!("enable foreign_keys: {e}"))?;
    conn.pragma_update(None, "busy_timeout", 5000i32)
        .map_err(|e| format!("set busy_timeout: {e}"))?;

    let mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .map_err(|e| format!("read journal_mode: {e}"))?;
    let ok = mode.eq_ignore_ascii_case("wal")
        || (allow_memory && mode.eq_ignore_ascii_case("memory"));
    if !ok {
        return Err(format!("expected WAL journal_mode, got {mode}"));
    }
    Ok(())
}

#[allow(dead_code)]
pub fn current_user_version(conn: &Connection) -> Result<u32, String> {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| format!("read user_version: {e}"))
}

/// Soft check used by tests.
#[allow(dead_code)]
pub fn table_exists(conn: &Connection, name: &str) -> Result<bool, String> {
    let found: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name=?1",
            [name],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("table_exists: {e}"))?;
    Ok(found.is_some())
}
