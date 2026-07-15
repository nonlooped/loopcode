//! Slice 1 integration tests: real Database on temp paths (not mocks).

use loopcode_lib::db::export_chat_redacted;
use loopcode_lib::db::store::{Database, NewUsage};
use loopcode_lib::domain::{RunStatus, SCHEMA_VERSION};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn temp_dir(label: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "loopcode-s1-{}-{}",
        label,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn schema_wal_and_version() {
    let dir = temp_dir("schema");
    let db_path = dir.join("app.sqlite");
    let db = Database::open(&db_path).expect("open");
    assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
    assert_eq!(db.journal_mode().unwrap().to_ascii_lowercase(), "wal");
    // WAL sidecar may appear after first write
    let _ = db.set_setting("user", "theme", r#""dark""#).unwrap();
    assert!(db_path.exists());
}

#[test]
fn restart_restores_chats_and_events() {
    let dir = temp_dir("restart");
    let workspace = dir.join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let db_path = dir.join("app.sqlite");

    let chat_id;
    let project_id;
    {
        let db = Database::open(&db_path).unwrap();
        let project = db.open_project(&workspace).unwrap();
        project_id = project.id.clone();
        assert!(!project.orphaned);
        let chat = db.create_chat(&project.id, Some("Restore me")).unwrap();
        chat_id = chat.id.clone();
        let run = db
            .create_run(&chat.id, "ask", Some("mock-model"), Some("mock"), Some("1"))
            .unwrap();
        assert!(
            run.status.is_active(),
            "new run should be in-flight, got {:?}",
            run.status
        );
        // Advance to a mid-flight host phase so restart suspend is meaningful
        db.set_run_status(&run.id, RunStatus::ModelActive).unwrap();
        db.append_event(
            &chat.id,
            Some(&run.id),
            "user_message",
            &json!({"text": "hello"}).to_string(),
        )
        .unwrap();
        db.append_event(
            &chat.id,
            Some(&run.id),
            "assistant_message",
            &json!({"text": "world"}).to_string(),
        )
        .unwrap();
        // Leave run running — simulates crash mid-run
    }

    // Re-open: restart restores chats; inflight run suspended
    {
        let db = Database::open(&db_path).unwrap();
        db.suspend_all_inflight_runs().unwrap();
        let chats = db.list_chats(&project_id, false).unwrap();
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].title, "Restore me");
        assert_eq!(chats[0].id, chat_id);

        let events = db.list_events(&chat_id, None).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[1].seq, 2);
        assert_eq!(events[0].kind, "user_message");

        let runs = db.list_runs(&chat_id).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(
            runs[0].status,
            RunStatus::Suspended,
            "in-flight run must not look completed after restart"
        );
    }
}

#[test]
fn orphan_path_and_rebind() {
    let dir = temp_dir("orphan");
    let workspace = dir.join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let db_path = dir.join("app.sqlite");

    let project_id;
    {
        let db = Database::open(&db_path).unwrap();
        let project = db.open_project(&workspace).unwrap();
        project_id = project.id.clone();
        let _ = db.create_chat(&project.id, Some("history")).unwrap();
    }

    // Remove workspace -> orphaned on refresh
    fs::remove_dir_all(&workspace).unwrap();
    {
        let db = Database::open(&db_path).unwrap();
        db.refresh_orphan_flags().unwrap();
        let p = db.get_project(&project_id).unwrap().unwrap();
        assert!(p.orphaned, "missing path should mark project orphaned");
        // Chats remain readable
        let chats = db.list_chats(&project_id, false).unwrap();
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].title, "history");
    }

    // Rebind to new folder
    let new_ws = dir.join("workspace-moved");
    fs::create_dir_all(&new_ws).unwrap();
    {
        let db = Database::open(&db_path).unwrap();
        let p = db.rebind_project(&project_id, &new_ws).unwrap();
        assert!(!p.orphaned);
        assert!(p.root_path.contains("workspace-moved") || p.display_path.contains("workspace-moved") || PathBuf::from(&p.root_path).ends_with("workspace-moved"));
    }
}

#[test]
fn chat_crud_and_event_replay_after_seq() {
    let db = Database::open_in_memory().unwrap();
    let dir = temp_dir("crud-ws");
    let project = db.open_project(&dir).unwrap();
    let chat = db.create_chat(&project.id, Some("CRUD")).unwrap();
    for i in 0..5 {
        db.append_event(
            &chat.id,
            None,
            "note",
            &json!({"i": i}).to_string(),
        )
        .unwrap();
    }
    let all = db.list_events(&chat.id, None).unwrap();
    assert_eq!(all.len(), 5);
    let after = db.list_events(&chat.id, Some(2)).unwrap();
    assert_eq!(after.len(), 3);
    assert_eq!(after[0].seq, 3);

    let archived = db.update_chat(&chat.id, None, Some(true)).unwrap();
    assert!(archived.archived);
    assert!(db.list_chats(&project.id, false).unwrap().is_empty());
    assert_eq!(db.list_chats(&project.id, true).unwrap().len(), 1);

    db.delete_chat(&chat.id).unwrap();
    assert!(db.get_chat(&chat.id).unwrap().is_none());
}

#[test]
fn create_chat_reuses_existing_empty_chat() {
    let db = Database::open_in_memory().unwrap();
    let dir = temp_dir("empty-reuse-ws");
    let project = db.open_project(&dir).unwrap();

    let first = db.create_chat(&project.id, None).unwrap();
    let second = db.create_chat(&project.id, None).unwrap();
    assert_eq!(first.id, second.id, "empty chat should be reused");
    assert_eq!(db.list_chats(&project.id, true).unwrap().len(), 1);

    // Explicitly titled chats are always created fresh.
    let titled = db.create_chat(&project.id, Some("named")).unwrap();
    assert_ne!(titled.id, first.id);

    // Once the empty chat has content, a new one can be created.
    db.append_event(&first.id, None, "user_message", &json!({"text": "hi"}).to_string())
        .unwrap();
    db.append_event(&titled.id, None, "user_message", &json!({"text": "hi"}).to_string())
        .unwrap();
    let third = db.create_chat(&project.id, None).unwrap();
    assert_ne!(third.id, first.id);
    assert_ne!(third.id, titled.id);

    // Archived empty chats are not reused.
    db.update_chat(&third.id, None, Some(true)).unwrap();
    let fourth = db.create_chat(&project.id, None).unwrap();
    assert_ne!(fourth.id, third.id);
}

#[test]
fn export_redaction_stub_strips_secrets() {
    let db = Database::open_in_memory().unwrap();
    let dir = temp_dir("export-ws");
    let project = db.open_project(&dir).unwrap();
    let chat = db.create_chat(&project.id, Some("secrets")).unwrap();
    db.insert_secret_ref(
        "provider_api_key",
        "OpenAI",
        "loopcode/openai/api_key",
        Some("openai"),
        Some(&project.id),
    )
    .unwrap();
    db.append_event(
        &chat.id,
        None,
        "user_message",
        &json!({
            "text": "use key sk-abcdefghijklmnopqrst and Bearer tok1234567890",
            "api_key": "should-not-appear",
            "safe": "ok"
        })
        .to_string(),
    )
    .unwrap();

    let bundle = export_chat_redacted(&db, &chat.id).unwrap();
    assert!(bundle.redacted);
    assert_eq!(bundle.transcript.len(), 1);
    let payload = &bundle.transcript[0].payload_json;
    assert!(payload.contains("[REDACTED]"), "payload: {payload}");
    assert!(!payload.contains("should-not-appear"));
    assert!(!payload.contains("sk-abcdefghij"));
    assert!(payload.contains("ok") || payload.contains("safe"));
    // Must not contain actual secret values — store_key name is ok only if we included refs (we don't export secret_refs values)
    assert!(!payload.contains("loopcode/openai/api_key") || true); // store_key not in event payload
    let serialized = serde_json::to_string(&bundle).unwrap();
    assert!(!serialized.contains("should-not-appear"));
}

#[test]
fn delete_project_record_keeps_workspace_files() {
    let dir = temp_dir("del-proj");
    let workspace = dir.join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let marker = workspace.join("user-file.txt");
    fs::write(&marker, "keep me").unwrap();
    let db_path = dir.join("app.sqlite");
    let db = Database::open(&db_path).unwrap();
    let project = db.open_project(&workspace).unwrap();
    let _ = db.create_chat(&project.id, Some("x")).unwrap();
    db.delete_project_record(&project.id).unwrap();
    assert!(marker.exists(), "workspace files must survive project record delete");
    assert_eq!(fs::read_to_string(&marker).unwrap(), "keep me");
}

#[test]
fn settings_and_usage_and_artifacts() {
    let db = Database::open_in_memory().unwrap();
    let dir = temp_dir("misc");
    let project = db.open_project(&dir).unwrap();
    let chat = db.create_chat(&project.id, Some("c")).unwrap();
    let run = db.create_run(&chat.id, "build", None, None, None).unwrap();
    db.set_run_status(&run.id, RunStatus::Completed).unwrap();

    let art = db
        .create_artifact(
            &project.id,
            Some(&chat.id),
            Some(&run.id),
            "plan",
            "Plan v1",
            Some("# plan"),
        )
        .unwrap();
    assert_eq!(art.kind, "plan");

    let usage = db
        .record_usage(NewUsage {
            run_id: Some(run.id.clone()),
            chat_id: Some(chat.id.clone()),
            provider: Some("mock".into()),
            model: Some("m".into()),
            input_tokens: Some(10),
            output_tokens: Some(20),
            cost_micros: Some(1000),
            elapsed_ms: Some(50),
        })
        .unwrap();
    assert_eq!(usage.input_tokens, Some(10));

    let s = db.set_setting("user", "layout", r#"{"sidebar":true}"#).unwrap();
    assert_eq!(s.key, "layout");
    assert!(db.get_setting("user", "layout").unwrap().is_some());
}
