use crate::db::migrate::{configure_connection, configure_connection_allow_memory, ensure_schema_not_newer, migrate};
use crate::domain::{
    Artifact, AuditRecord, Chat, Event, Project, Run, RunStatus, SecretRef, Setting, TrustRecord,
    UsageRecord, DEFAULT_CHAT_TITLE, SCHEMA_VERSION,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Input for [`Database::record_usage`].
#[derive(Debug, Clone, Default)]
pub struct NewUsage {
    pub run_id: Option<String>,
    pub chat_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cost_micros: Option<i64>,
    pub elapsed_ms: Option<i64>,
}

/// Open LoopCode app database at `db_path` (creates parent dirs).
pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Database {
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self, String> {
        let path = db_path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create data dir {}: {e}", parent.display()))?;
        }
        let conn = Connection::open(&path).map_err(|e| format!("open sqlite: {e}"))?;
        ensure_schema_not_newer(&conn)?;
        configure_connection(&conn)?;
        // Check integrity before normal writes or migrations.
        crate::reliability::backup::integrity_check(&conn)?;
        migrate(&conn)?;
        Ok(Self { conn, path })
    }

    /// Run integrity check on the live connection (fail-closed).
    pub fn integrity_check(&self) -> Result<crate::reliability::backup::IntegrityReport, String> {
        crate::reliability::backup::integrity_check(&self.conn)
    }

    /// Safe snapshot via VACUUM INTO (not a hot WAL file copy).
    pub fn backup_to(&self, backup_path: impl AsRef<Path>) -> Result<PathBuf, String> {
        crate::reliability::backup::backup_database(&self.path, backup_path.as_ref())
    }

    /// Open an in-memory DB (tests).
    pub fn open_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| format!("open memory sqlite: {e}"))?;
        configure_connection_allow_memory(&conn)?;
        migrate(&conn)?;
        Ok(Self {
            conn,
            path: PathBuf::from(":memory:"),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn schema_version(&self) -> Result<u32, String> {
        self.conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|e| format!("user_version: {e}"))
    }

    pub fn journal_mode(&self) -> Result<String, String> {
        self.conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .map_err(|e| format!("journal_mode: {e}"))
    }

    // --- Projects ---

    /// Open or create a project for a local folder path. Marks orphaned if path unreadable.
    pub fn open_project(&self, folder: impl AsRef<Path>) -> Result<Project, String> {
        let folder = folder.as_ref();
        let display_path = normalize_display_path(folder)?;
        let (resolved_path, orphaned) = resolve_project_path(folder);

        let now = Utc::now();
        // Upsert by root_path (normalized display form)
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM projects WHERE root_path = ?1",
                [&display_path],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("lookup project: {e}"))?;

        if let Some(id) = existing {
            self.conn
                .execute(
                    "UPDATE projects SET resolved_path = ?1, display_path = ?2, orphaned = ?3, updated_at = ?4 WHERE id = ?5",
                    params![
                        resolved_path,
                        display_path,
                        orphaned as i32,
                        now.to_rfc3339(),
                        id
                    ],
                )
                .map_err(|e| format!("update project: {e}"))?;
            return self
                .get_project(&id)?
                .ok_or_else(|| "project missing after update".into());
        }

        let id = Uuid::new_v4().to_string();
        self.conn
            .execute(
                "INSERT INTO projects (id, root_path, resolved_path, display_path, orphaned, last_active_chat_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?6)",
                params![
                    id,
                    display_path,
                    resolved_path,
                    display_path,
                    orphaned as i32,
                    now.to_rfc3339()
                ],
            )
            .map_err(|e| format!("insert project: {e}"))?;
        self.get_project(&id)?
            .ok_or_else(|| "project missing after insert".into())
    }

    /// Rebind an orphaned (or moved) project to a new root path.
    pub fn rebind_project(
        &self,
        project_id: &str,
        new_folder: impl AsRef<Path>,
    ) -> Result<Project, String> {
        let _ = self
            .get_project(project_id)?
            .ok_or_else(|| format!("project not found: {project_id}"))?;
        let folder = new_folder.as_ref();
        let display_path = normalize_display_path(folder)?;
        let (resolved_path, orphaned) = resolve_project_path(folder);
        let now = Utc::now();

        // Ensure new path not already owned by another project
        let clash: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM projects WHERE root_path = ?1 AND id != ?2",
                params![display_path, project_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("rebind clash check: {e}"))?;
        if clash.is_some() {
            return Err(format!("another project already uses path {display_path}"));
        }

        self.conn
            .execute(
                "UPDATE projects SET root_path = ?1, resolved_path = ?2, display_path = ?3, orphaned = ?4, updated_at = ?5 WHERE id = ?6",
                params![
                    display_path,
                    resolved_path,
                    display_path,
                    orphaned as i32,
                    now.to_rfc3339(),
                    project_id
                ],
            )
            .map_err(|e| format!("rebind project: {e}"))?;
        self.get_project(project_id)?
            .ok_or_else(|| "project missing after rebind".into())
    }

    /// Refresh orphan flags for all projects (e.g. on app start).
    pub fn refresh_orphan_flags(&self) -> Result<usize, String> {
        let projects = self.list_projects()?;
        let mut n = 0;
        let now = Utc::now().to_rfc3339();
        for p in projects {
            let path = PathBuf::from(&p.root_path);
            let orphaned = !path_is_usable_dir(&path);
            if orphaned != p.orphaned {
                self.conn
                    .execute(
                        "UPDATE projects SET orphaned = ?1, updated_at = ?2 WHERE id = ?3",
                        params![orphaned as i32, now, p.id],
                    )
                    .map_err(|e| format!("refresh orphan: {e}"))?;
                n += 1;
            }
        }
        Ok(n)
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>, String> {
        self.conn
            .query_row(
                "SELECT id, root_path, resolved_path, display_path, orphaned, last_active_chat_id, created_at, updated_at
                 FROM projects WHERE id = ?1",
                [id],
                map_project,
            )
            .optional()
            .map_err(|e| format!("get_project: {e}"))
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, root_path, resolved_path, display_path, orphaned, last_active_chat_id, created_at, updated_at
                 FROM projects ORDER BY updated_at DESC",
            )
            .map_err(|e| format!("list_projects prepare: {e}"))?;
        let rows = stmt
            .query_map([], map_project)
            .map_err(|e| format!("list_projects: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_projects row: {e}"))
    }

    pub fn delete_project_record(&self, project_id: &str) -> Result<(), String> {
        let n = self
            .conn
            .execute("DELETE FROM projects WHERE id = ?1", [project_id])
            .map_err(|e| format!("delete project: {e}"))?;
        if n == 0 {
            return Err(format!("project not found: {project_id}"));
        }
        Ok(())
    }

    pub fn set_last_active_chat(
        &self,
        project_id: &str,
        chat_id: Option<&str>,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE projects SET last_active_chat_id = ?1, updated_at = ?2 WHERE id = ?3",
                params![chat_id, now, project_id],
            )
            .map_err(|e| format!("set last active chat: {e}"))?;
        Ok(())
    }

    // --- Chats ---

    pub fn create_chat(&self, project_id: &str, title: Option<&str>) -> Result<Chat, String> {
        let _ = self
            .get_project(project_id)?
            .ok_or_else(|| format!("project not found: {project_id}"))?;
        // Reuse an existing empty chat instead of piling up blank ones.
        // Explicitly titled chats are always created.
        let untitled = title.map(str::trim).filter(|s| !s.is_empty()).is_none();
        if untitled {
            if let Some(existing) = self.find_empty_chat(project_id)? {
                self.set_last_active_chat(project_id, Some(&existing.id))?;
                return Ok(existing);
            }
        }
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let title = title
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_CHAT_TITLE)
            .to_string();
        self.conn
            .execute(
                "INSERT INTO chats (id, project_id, title, archived, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 0, ?4, ?4)",
                params![id, project_id, title, now.to_rfc3339()],
            )
            .map_err(|e| format!("create chat: {e}"))?;
        self.set_last_active_chat(project_id, Some(&id))?;
        self.get_chat(&id)?
            .ok_or_else(|| "chat missing after insert".into())
    }

    /// Most recent non-archived chat with no events (i.e. no content yet).
    fn find_empty_chat(&self, project_id: &str) -> Result<Option<Chat>, String> {
        self.conn
            .query_row(
                "SELECT c.id, c.project_id, c.title, c.archived, c.created_at, c.updated_at
                 FROM chats c
                 WHERE c.project_id = ?1 AND c.archived = 0
                   AND NOT EXISTS (SELECT 1 FROM events e WHERE e.chat_id = c.id)
                 ORDER BY c.updated_at DESC LIMIT 1",
                [project_id],
                map_chat,
            )
            .optional()
            .map_err(|e| format!("find_empty_chat: {e}"))
    }

    pub fn get_chat(&self, id: &str) -> Result<Option<Chat>, String> {
        self.conn
            .query_row(
                "SELECT id, project_id, title, archived, created_at, updated_at FROM chats WHERE id = ?1",
                [id],
                map_chat,
            )
            .optional()
            .map_err(|e| format!("get_chat: {e}"))
    }

    pub fn list_chats(
        &self,
        project_id: &str,
        include_archived: bool,
    ) -> Result<Vec<Chat>, String> {
        let sql = if include_archived {
            "SELECT id, project_id, title, archived, created_at, updated_at FROM chats
             WHERE project_id = ?1 ORDER BY updated_at DESC"
        } else {
            "SELECT id, project_id, title, archived, created_at, updated_at FROM chats
             WHERE project_id = ?1 AND archived = 0 ORDER BY updated_at DESC"
        };
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| format!("list_chats prepare: {e}"))?;
        let rows = stmt
            .query_map([project_id], map_chat)
            .map_err(|e| format!("list_chats: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_chats row: {e}"))
    }

    pub fn update_chat(
        &self,
        chat_id: &str,
        title: Option<&str>,
        archived: Option<bool>,
    ) -> Result<Chat, String> {
        let chat = self
            .get_chat(chat_id)?
            .ok_or_else(|| format!("chat not found: {chat_id}"))?;
        let now = Utc::now().to_rfc3339();
        let new_title = title
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&chat.title);
        let new_archived = archived.unwrap_or(chat.archived);
        self.conn
            .execute(
                "UPDATE chats SET title = ?1, archived = ?2, updated_at = ?3 WHERE id = ?4",
                params![new_title, new_archived as i32, now, chat_id],
            )
            .map_err(|e| format!("update chat: {e}"))?;
        self.get_chat(chat_id)?
            .ok_or_else(|| "chat missing after update".into())
    }

    pub fn delete_chat(&self, chat_id: &str) -> Result<(), String> {
        let chat = self
            .get_chat(chat_id)?
            .ok_or_else(|| format!("chat not found: {chat_id}"))?;
        self.conn
            .execute("DELETE FROM chats WHERE id = ?1", [chat_id])
            .map_err(|e| format!("delete chat: {e}"))?;
        // Clear last active if it pointed here
        if let Some(p) = self.get_project(&chat.project_id)? {
            if p.last_active_chat_id.as_deref() == Some(chat_id) {
                self.set_last_active_chat(&chat.project_id, None)?;
            }
        }
        Ok(())
    }

    // --- Runs ---

    pub fn create_run(
        &self,
        chat_id: &str,
        mode: &str,
        model: Option<&str>,
        adapter: Option<&str>,
        policy_version: Option<&str>,
    ) -> Result<Run, String> {
        let _ = self
            .get_chat(chat_id)?
            .ok_or_else(|| format!("chat not found: {chat_id}"))?;
        // Suspend any non-terminal active run (exactly one active run rule for later slices)
        self.suspend_non_terminal_runs(chat_id)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        self.conn
            .execute(
                "INSERT INTO runs (id, chat_id, mode, model, adapter, policy_version, status, predecessor_run_id, created_at, updated_at, finished_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?8, NULL)",
                params![
                    id,
                    chat_id,
                    mode,
                    model,
                    adapter,
                    policy_version,
                    RunStatus::Queued.as_str(),
                    now.to_rfc3339()
                ],
            )
            .map_err(|e| format!("create run: {e}"))?;
        self.get_run(&id)?
            .ok_or_else(|| "run missing after insert".into())
    }

    pub fn set_run_status(&self, run_id: &str, status: RunStatus) -> Result<Run, String> {
        let now = Utc::now();
        let finished = if status.is_terminal() {
            Some(now.to_rfc3339())
        } else {
            None
        };
        self.conn
            .execute(
                "UPDATE runs SET status = ?1, updated_at = ?2, finished_at = COALESCE(?3, finished_at) WHERE id = ?4",
                params![status.as_str(), now.to_rfc3339(), finished, run_id],
            )
            .map_err(|e| format!("set run status: {e}"))?;
        self.get_run(run_id)?
            .ok_or_else(|| format!("run not found: {run_id}"))
    }

    /// Mark non-terminal active runs as suspended (restart reconciliation).
    /// Never guesses completed or cancelled.
    pub fn suspend_non_terminal_runs(&self, chat_id: &str) -> Result<usize, String> {
        let now = Utc::now().to_rfc3339();
        let sql = format!(
            "UPDATE runs SET status = 'suspended', updated_at = ?1
             WHERE chat_id = ?2 AND status IN ({})",
            RunStatus::inflight_sql_list()
        );
        let n = self
            .conn
            .execute(&sql, params![now, chat_id])
            .map_err(|e| format!("suspend runs: {e}"))?;
        Ok(n)
    }

    /// On app start: any in-flight host-phase run becomes suspended.
    pub fn suspend_all_inflight_runs(&self) -> Result<usize, String> {
        let now = Utc::now().to_rfc3339();
        let sql = format!(
            "UPDATE runs SET status = 'suspended', updated_at = ?1
             WHERE status IN ({})",
            RunStatus::inflight_sql_list()
        );
        let n = self
            .conn
            .execute(&sql, params![now])
            .map_err(|e| format!("suspend all inflight: {e}"))?;
        Ok(n)
    }

    pub fn get_run(&self, id: &str) -> Result<Option<Run>, String> {
        self.conn
            .query_row(
                "SELECT id, chat_id, mode, model, adapter, policy_version, status, predecessor_run_id, created_at, updated_at, finished_at
                 FROM runs WHERE id = ?1",
                [id],
                map_run,
            )
            .optional()
            .map_err(|e| format!("get_run: {e}"))
    }

    pub fn list_runs(&self, chat_id: &str) -> Result<Vec<Run>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, chat_id, mode, model, adapter, policy_version, status, predecessor_run_id, created_at, updated_at, finished_at
                 FROM runs WHERE chat_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| format!("list_runs prepare: {e}"))?;
        let rows = stmt
            .query_map([chat_id], map_run)
            .map_err(|e| format!("list_runs: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list_runs row: {e}"))
    }

    // --- Events ---

    pub fn append_event(
        &self,
        chat_id: &str,
        run_id: Option<&str>,
        kind: &str,
        payload_json: &str,
    ) -> Result<Event, String> {
        let _ = self
            .get_chat(chat_id)?
            .ok_or_else(|| format!("chat not found: {chat_id}"))?;
        // Validate JSON payload
        let _: serde_json::Value = serde_json::from_str(payload_json)
            .map_err(|e| format!("payload_json must be valid JSON: {e}"))?;
        // This is the single durable event-write boundary. Redact here as well
        // as at producers so future call sites cannot accidentally persist a
        // credential-shaped payload.
        let payload_json = crate::security::redact_payload_json(payload_json);

        let next_seq: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM events WHERE chat_id = ?1",
                [chat_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("next seq: {e}"))?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        self.conn
            .execute(
                "INSERT INTO events (id, chat_id, run_id, seq, kind, payload_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    id,
                    chat_id,
                    run_id,
                    next_seq,
                    kind,
                    payload_json,
                    now.to_rfc3339()
                ],
            )
            .map_err(|e| format!("append event: {e}"))?;

        // Bump chat updated_at
        self.conn
            .execute(
                "UPDATE chats SET updated_at = ?1 WHERE id = ?2",
                params![now.to_rfc3339(), chat_id],
            )
            .map_err(|e| format!("touch chat: {e}"))?;

        self.get_event(&id)?
            .ok_or_else(|| "event missing after insert".into())
    }

    pub fn get_event(&self, id: &str) -> Result<Option<Event>, String> {
        self.conn
            .query_row(
                "SELECT id, chat_id, run_id, seq, kind, payload_json, created_at FROM events WHERE id = ?1",
                [id],
                map_event,
            )
            .optional()
            .map_err(|e| format!("get_event: {e}"))
    }

    /// Replay events for a chat in seq order. Optional `after_seq` exclusive lower bound.
    pub fn list_events(&self, chat_id: &str, after_seq: Option<i64>) -> Result<Vec<Event>, String> {
        if let Some(after) = after_seq {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, chat_id, run_id, seq, kind, payload_json, created_at FROM events
                     WHERE chat_id = ?1 AND seq > ?2 ORDER BY seq ASC",
                )
                .map_err(|e| format!("list_events prepare: {e}"))?;
            let rows = stmt
                .query_map(params![chat_id, after], map_event)
                .map_err(|e| format!("list_events: {e}"))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_events row: {e}"))
        } else {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, chat_id, run_id, seq, kind, payload_json, created_at FROM events
                     WHERE chat_id = ?1 ORDER BY seq ASC",
                )
                .map_err(|e| format!("list_events prepare: {e}"))?;
            let rows = stmt
                .query_map([chat_id], map_event)
                .map_err(|e| format!("list_events: {e}"))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list_events row: {e}"))
        }
    }

    // --- Artifacts ---

    pub fn create_artifact(
        &self,
        project_id: &str,
        chat_id: Option<&str>,
        run_id: Option<&str>,
        kind: &str,
        title: &str,
        body_inline: Option<&str>,
    ) -> Result<Artifact, String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        self.conn
            .execute(
                "INSERT INTO artifacts (id, project_id, chat_id, run_id, kind, title, body_inline, body_path, version, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, 1, ?8, ?8)",
                params![
                    id,
                    project_id,
                    chat_id,
                    run_id,
                    kind,
                    title,
                    body_inline,
                    now.to_rfc3339()
                ],
            )
            .map_err(|e| format!("create artifact: {e}"))?;
        self.get_artifact(&id)?
            .ok_or_else(|| "artifact missing after insert".into())
    }

    pub fn get_artifact(&self, id: &str) -> Result<Option<Artifact>, String> {
        self.conn
            .query_row(
                "SELECT id, project_id, chat_id, run_id, kind, title, body_inline, body_path, version, created_at, updated_at
                 FROM artifacts WHERE id = ?1",
                [id],
                map_artifact,
            )
            .optional()
            .map_err(|e| format!("get_artifact: {e}"))
    }

    // --- Usage ---

    pub fn record_usage(&self, entry: NewUsage) -> Result<UsageRecord, String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        self.conn
            .execute(
                "INSERT INTO usage_ledger (id, run_id, chat_id, provider, model, input_tokens, output_tokens, cost_micros, elapsed_ms, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    id,
                    entry.run_id,
                    entry.chat_id,
                    entry.provider,
                    entry.model,
                    entry.input_tokens,
                    entry.output_tokens,
                    entry.cost_micros,
                    entry.elapsed_ms,
                    now.to_rfc3339()
                ],
            )
            .map_err(|e| format!("record usage: {e}"))?;
        self.conn
            .query_row(
                "SELECT id, run_id, chat_id, provider, model, input_tokens, output_tokens, cost_micros, elapsed_ms, created_at
                 FROM usage_ledger WHERE id = ?1",
                [&id],
                map_usage,
            )
            .map_err(|e| format!("get usage: {e}"))
    }

    // --- Settings ---

    pub fn set_setting(&self, scope: &str, key: &str, value_json: &str) -> Result<Setting, String> {
        let _: serde_json::Value = serde_json::from_str(value_json)
            .map_err(|e| format!("value_json must be valid JSON: {e}"))?;
        let now = Utc::now();
        self.conn
            .execute(
                "INSERT INTO settings (scope, key, value_json, updated_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(scope, key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
                params![scope, key, value_json, now.to_rfc3339()],
            )
            .map_err(|e| format!("set setting: {e}"))?;
        Ok(Setting {
            scope: scope.into(),
            key: key.into(),
            value_json: value_json.into(),
            updated_at: now,
        })
    }

    pub fn get_setting(&self, scope: &str, key: &str) -> Result<Option<Setting>, String> {
        self.conn
            .query_row(
                "SELECT scope, key, value_json, updated_at FROM settings WHERE scope = ?1 AND key = ?2",
                params![scope, key],
                map_setting,
            )
            .optional()
            .map_err(|e| format!("get setting: {e}"))
    }

    // --- Secret refs (metadata only) ---

    pub fn insert_secret_ref(
        &self,
        kind: &str,
        label: &str,
        store_key: &str,
        provider: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<SecretRef, String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        self.conn
            .execute(
                "INSERT INTO secret_refs (id, kind, label, store_key, provider, project_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    id,
                    kind,
                    label,
                    store_key,
                    provider,
                    project_id,
                    now.to_rfc3339()
                ],
            )
            .map_err(|e| format!("insert secret_ref: {e}"))?;
        self.conn
            .query_row(
                "SELECT id, kind, label, store_key, provider, project_id, created_at, updated_at FROM secret_refs WHERE id = ?1",
                [&id],
                map_secret_ref,
            )
            .map_err(|e| format!("get secret_ref: {e}"))
    }

    pub fn count_secret_refs_for_project(&self, project_id: &str) -> Result<i64, String> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM secret_refs WHERE project_id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("count secret_refs: {e}"))
    }

    pub fn list_secret_refs(&self, project_id: Option<&str>) -> Result<Vec<SecretRef>, String> {
        if let Some(pid) = project_id {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, kind, label, store_key, provider, project_id, created_at, updated_at
                     FROM secret_refs WHERE project_id = ?1",
                )
                .map_err(|e| format!("list secret_refs: {e}"))?;
            let rows = stmt
                .query_map([pid], map_secret_ref)
                .map_err(|e| format!("list secret_refs: {e}"))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list secret_refs row: {e}"))
        } else {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, kind, label, store_key, provider, project_id, created_at, updated_at
                     FROM secret_refs",
                )
                .map_err(|e| format!("list secret_refs: {e}"))?;
            let rows = stmt
                .query_map([], map_secret_ref)
                .map_err(|e| format!("list secret_refs: {e}"))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("list secret_refs row: {e}"))
        }
    }

    // --- Audit ---

    pub fn insert_audit(
        &self,
        kind: &str,
        actor: &str,
        run_id: Option<&str>,
        chat_id: Option<&str>,
        project_id: Option<&str>,
        payload_json: &str,
    ) -> Result<AuditRecord, String> {
        let payload_json = crate::security::redact_payload_json(payload_json);
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO audit_log (id, kind, actor, run_id, chat_id, project_id, payload_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    id,
                    kind,
                    actor,
                    run_id,
                    chat_id,
                    project_id,
                    &payload_json,
                    created_at
                ],
            )
            .map_err(|e| format!("insert audit: {e}"))?;
        Ok(AuditRecord {
            id,
            kind: kind.into(),
            actor: actor.into(),
            run_id: run_id.map(str::to_string),
            chat_id: chat_id.map(str::to_string),
            project_id: project_id.map(str::to_string),
            payload_json,
            created_at,
        })
    }

    pub fn list_audit(&self, limit: i64) -> Result<Vec<AuditRecord>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, kind, actor, run_id, chat_id, project_id, payload_json, created_at
                 FROM audit_log ORDER BY created_at ASC LIMIT ?1",
            )
            .map_err(|e| format!("list audit prepare: {e}"))?;
        let rows = stmt
            .query_map([limit], |row| {
                Ok(AuditRecord {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    actor: row.get(2)?,
                    run_id: row.get(3)?,
                    chat_id: row.get(4)?,
                    project_id: row.get(5)?,
                    payload_json: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| format!("list audit: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("list audit row: {e}"))
    }

    // --- Trust ---

    pub fn upsert_trust(
        &self,
        kind: &str,
        subject_key: &str,
        content_hash: &str,
        project_id: Option<&str>,
    ) -> Result<TrustRecord, String> {
        self.delete_trust(kind, subject_key, project_id)?;
        let id = Uuid::new_v4().to_string();
        let trusted_at = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO trust_records (id, kind, subject_key, content_hash, project_id, trusted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, kind, subject_key, content_hash, project_id, trusted_at],
            )
            .map_err(|e| format!("upsert trust: {e}"))?;
        Ok(TrustRecord {
            id,
            kind: kind.into(),
            subject_key: subject_key.into(),
            content_hash: content_hash.into(),
            project_id: project_id.map(str::to_string),
            trusted_at,
        })
    }

    pub fn get_trust(
        &self,
        kind: &str,
        subject_key: &str,
        project_id: Option<&str>,
    ) -> Result<Option<TrustRecord>, String> {
        let result = match project_id {
            Some(pid) => self
                .conn
                .query_row(
                    "SELECT id, kind, subject_key, content_hash, project_id, trusted_at
                     FROM trust_records WHERE kind = ?1 AND subject_key = ?2 AND project_id = ?3",
                    params![kind, subject_key, pid],
                    map_trust,
                )
                .optional(),
            None => self
                .conn
                .query_row(
                    "SELECT id, kind, subject_key, content_hash, project_id, trusted_at
                     FROM trust_records WHERE kind = ?1 AND subject_key = ?2 AND project_id IS NULL",
                    params![kind, subject_key],
                    map_trust,
                )
                .optional(),
        };
        result.map_err(|e| format!("get trust: {e}"))
    }

    pub fn delete_trust(
        &self,
        kind: &str,
        subject_key: &str,
        project_id: Option<&str>,
    ) -> Result<(), String> {
        match project_id {
            Some(pid) => {
                self.conn
                    .execute(
                        "DELETE FROM trust_records WHERE kind = ?1 AND subject_key = ?2 AND project_id = ?3",
                        params![kind, subject_key, pid],
                    )
                    .map_err(|e| format!("delete trust: {e}"))?;
            }
            None => {
                self.conn
                    .execute(
                        "DELETE FROM trust_records WHERE kind = ?1 AND subject_key = ?2 AND project_id IS NULL",
                        params![kind, subject_key],
                    )
                    .map_err(|e| format!("delete trust: {e}"))?;
            }
        }
        Ok(())
    }

    pub fn expected_schema_version() -> u32 {
        SCHEMA_VERSION
    }
}

fn map_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        root_path: row.get(1)?,
        resolved_path: row.get(2)?,
        display_path: row.get(3)?,
        orphaned: row.get::<_, i32>(4)? != 0,
        last_active_chat_id: row.get(5)?,
        created_at: parse_dt(&row.get::<_, String>(6)?)?,
        updated_at: parse_dt(&row.get::<_, String>(7)?)?,
    })
}

fn map_chat(row: &rusqlite::Row<'_>) -> rusqlite::Result<Chat> {
    Ok(Chat {
        id: row.get(0)?,
        project_id: row.get(1)?,
        title: row.get(2)?,
        archived: row.get::<_, i32>(3)? != 0,
        created_at: parse_dt(&row.get::<_, String>(4)?)?,
        updated_at: parse_dt(&row.get::<_, String>(5)?)?,
    })
}

fn map_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<Run> {
    let status_s: String = row.get(6)?;
    let status = RunStatus::parse(&status_s).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            6,
            rusqlite::types::Type::Text,
            format!("unknown run status: {status_s}").into(),
        )
    })?;
    let finished: Option<String> = row.get(10)?;
    Ok(Run {
        id: row.get(0)?,
        chat_id: row.get(1)?,
        mode: row.get(2)?,
        model: row.get(3)?,
        adapter: row.get(4)?,
        policy_version: row.get(5)?,
        status,
        predecessor_run_id: row.get(7)?,
        created_at: parse_dt(&row.get::<_, String>(8)?)?,
        updated_at: parse_dt(&row.get::<_, String>(9)?)?,
        finished_at: finished.as_deref().map(parse_dt).transpose()?,
    })
}

fn map_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<Event> {
    Ok(Event {
        id: row.get(0)?,
        chat_id: row.get(1)?,
        run_id: row.get(2)?,
        seq: row.get(3)?,
        kind: row.get(4)?,
        payload_json: row.get(5)?,
        created_at: parse_dt(&row.get::<_, String>(6)?)?,
    })
}

fn map_artifact(row: &rusqlite::Row<'_>) -> rusqlite::Result<Artifact> {
    Ok(Artifact {
        id: row.get(0)?,
        project_id: row.get(1)?,
        chat_id: row.get(2)?,
        run_id: row.get(3)?,
        kind: row.get(4)?,
        title: row.get(5)?,
        body_inline: row.get(6)?,
        body_path: row.get(7)?,
        version: row.get(8)?,
        created_at: parse_dt(&row.get::<_, String>(9)?)?,
        updated_at: parse_dt(&row.get::<_, String>(10)?)?,
    })
}

fn map_usage(row: &rusqlite::Row<'_>) -> rusqlite::Result<UsageRecord> {
    Ok(UsageRecord {
        id: row.get(0)?,
        run_id: row.get(1)?,
        chat_id: row.get(2)?,
        provider: row.get(3)?,
        model: row.get(4)?,
        input_tokens: row.get(5)?,
        output_tokens: row.get(6)?,
        cost_micros: row.get(7)?,
        elapsed_ms: row.get(8)?,
        created_at: parse_dt(&row.get::<_, String>(9)?)?,
    })
}

fn map_setting(row: &rusqlite::Row<'_>) -> rusqlite::Result<Setting> {
    Ok(Setting {
        scope: row.get(0)?,
        key: row.get(1)?,
        value_json: row.get(2)?,
        updated_at: parse_dt(&row.get::<_, String>(3)?)?,
    })
}

fn map_secret_ref(row: &rusqlite::Row<'_>) -> rusqlite::Result<SecretRef> {
    Ok(SecretRef {
        id: row.get(0)?,
        kind: row.get(1)?,
        label: row.get(2)?,
        store_key: row.get(3)?,
        provider: row.get(4)?,
        project_id: row.get(5)?,
        created_at: parse_dt(&row.get::<_, String>(6)?)?,
        updated_at: parse_dt(&row.get::<_, String>(7)?)?,
    })
}

fn map_trust(row: &rusqlite::Row<'_>) -> rusqlite::Result<TrustRecord> {
    Ok(TrustRecord {
        id: row.get(0)?,
        kind: row.get(1)?,
        subject_key: row.get(2)?,
        content_hash: row.get(3)?,
        project_id: row.get(4)?,
        trusted_at: row.get(5)?,
    })
}

fn parse_dt(s: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

fn normalize_display_path(path: &Path) -> Result<String, String> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("cwd: {e}"))?
            .join(path)
    };
    // Prefer dunce-style strip of \\?\ if present via canonicalize when exists
    let s = abs
        .canonicalize()
        .unwrap_or(abs)
        .to_string_lossy()
        .to_string();
    Ok(strip_unc_prefix(&s))
}

fn resolve_project_path(path: &Path) -> (String, bool) {
    match path.canonicalize() {
        Ok(p) => {
            let s = strip_unc_prefix(&p.to_string_lossy());
            let orphaned = !path_is_usable_dir(&p);
            (s, orphaned)
        }
        Err(_) => {
            let s = path.to_string_lossy().to_string();
            (s, true)
        }
    }
}

fn path_is_usable_dir(path: &Path) -> bool {
    path.is_dir()
}

fn strip_unc_prefix(s: &str) -> String {
    s.strip_prefix(r"\\?\").unwrap_or(s).to_string()
}

/// Default application data directory for LoopCode.
pub fn default_data_dir() -> Result<PathBuf, String> {
    let base = directories::ProjectDirs::from("com", "loopcode", "loopcode")
        .ok_or_else(|| "could not resolve OS data directory".to_string())?;
    Ok(base.data_dir().to_path_buf())
}

pub fn default_db_path() -> Result<PathBuf, String> {
    Ok(default_data_dir()?.join("app.sqlite"))
}
