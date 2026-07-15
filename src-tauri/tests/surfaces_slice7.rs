//! Slice 7: explorer, editor, diffs, checkpoints, conflicts.

use loopcode_lib::db::store::Database;
use loopcode_lib::runtime::tools::ToolProposal;
use loopcode_lib::runtime::{AgentRuntime, Mode};
use loopcode_lib::security::approval::GrantScope;
use loopcode_lib::security::ActionClass;
use loopcode_lib::surfaces::checkpoint::{
    chat_event_cursor, create_checkpoint, load_manifest, preview_intervening, restore_both,
    restore_conversation, restore_files, store_is_outside_project, RestoreAxis,
};
use loopcode_lib::surfaces::conflict::{
    decide_patch_allowed, evaluate_agent_patch_preflight, ConflictKind, ConflictOutcome,
    DirtyBufferState,
};
use loopcode_lib::surfaces::diff::{project_diffs, BufferSnapshot, DiffScope};
use loopcode_lib::surfaces::editor::{open_text_file, save_text_file};
use loopcode_lib::surfaces::explorer::{is_noise_name, list_project_tree};
use loopcode_lib::tools::hash::content_hash_str;
use loopcode_lib::tools::workspace::workspace_apply_patch;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn temp(label: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "loopcode-s7-{}-{}",
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
fn explorer_hides_noise_and_show_excluded() {
    let root = temp("tree");
    fs::write(root.join("readme.md"), "hi").unwrap();
    fs::create_dir_all(root.join("node_modules/x")).unwrap();
    fs::write(root.join("node_modules/x/a.js"), "1").unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "fn main(){}").unwrap();

    assert!(is_noise_name("node_modules"));
    let hidden = list_project_tree(&root, false, Some(4)).unwrap();
    assert!(hidden.iter().any(|e| e.path == "readme.md" || e.name == "readme.md"));
    assert!(hidden.iter().any(|e| e.name == "src"));
    assert!(!hidden.iter().any(|e| e.name == "node_modules"));
    assert!(!hidden.iter().any(|e| e.name == ".git"));

    let shown = list_project_tree(&root, true, Some(4)).unwrap();
    assert!(shown.iter().any(|e| e.name == "node_modules" && e.excluded));
}

#[test]
fn editor_open_save_path_bounded_hash_conflict() {
    let root = temp("ed");
    fs::write(root.join("note.txt"), "alpha").unwrap();
    let open = open_text_file(&root, "note.txt").unwrap();
    assert_eq!(open.content, "alpha");
    assert!(!open.binary);

    let saved = save_text_file(&root, "note.txt", "beta", Some(&open.hash)).unwrap();
    assert_eq!(saved.status, "ok");
    assert_eq!(fs::read_to_string(root.join("note.txt")).unwrap(), "beta");

    let err = save_text_file(&root, "note.txt", "gamma", Some("stale:0")).unwrap_err();
    assert!(err.contains("hash_conflict"), "{err}");

    // Parent escape rejected
    let outside = open_text_file(&root, "../outside.txt");
    assert!(outside.is_err());
}

#[test]
fn diffs_turn_cumulative_and_manual_attribution() {
    let db = Database::open_in_memory().unwrap();
    let root = temp("diff");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("d")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root.clone()));
    let store = temp("diff-cp");
    rt.set_checkpoint_store_override(Some(store.clone()));

    let run = rt
        .start_run(&db, &chat.id, Mode::Build, Some("m"))
        .unwrap();
    // free workspace write in Build
    let res = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new(
                "write",
                json!({"path": "src/a.rs", "content": "agent-bytes"}),
            ),
        )
        .unwrap();
    assert!(res.executed, "{:?}", res);

    let events = db.list_events(&chat.id, None).unwrap();
    let turn = project_diffs(
        DiffScope::ThisRun,
        &events,
        Some(&root),
        Some(&run.id),
        &[],
        None,
    );
    assert!(
        turn.changes.iter().any(|c| {
            c.path.contains("a.rs")
                && c.attribution.contains("agent:")
                && c.run_id.as_deref() == Some(&run.id)
        }),
        "{:?}",
        turn.changes
    );

    let cum = project_diffs(
        DiffScope::SinceChatStart,
        &events,
        Some(&root),
        None,
        &[],
        None,
    );
    assert!(
        cum.changes
            .iter()
            .any(|c| c.attribution.starts_with("agent:") && c.path.contains("a.rs")),
        "{:?}",
        cum.changes
    );

    // Honest manual path: dirty editor buffer vs disk via real project_diffs (not a hard-coded DiffChange).
    // Include multi-byte UTF-8 past 200 bytes to exercise preview truncate.
    let human = format!("{}{}", "a".repeat(199), "世-human");
    let buffer_model = project_diffs(
        DiffScope::BufferVsDisk,
        &events,
        Some(&root),
        None,
        &[BufferSnapshot {
            path: "src/a.rs".into(),
            content: human.clone(),
            dirty: true,
        }],
        None,
    );
    assert!(
        buffer_model.changes.iter().any(|c| {
            c.path.contains("a.rs") && c.attribution == "manual" && c.kind == "modify"
        }),
        "BufferVsDisk must attribute human dirty buffer as manual: {:?}",
        buffer_model.changes
    );

    // Honest manual path: live disk diverged from checkpoint post-image (agent snapshot then human edit).
    let cp = create_checkpoint(
        &root,
        &store,
        &project.id,
        Some(&chat.id),
        Some(&run.id),
        Some(events.last().map(|e| e.seq).unwrap_or(0)),
    )
    .unwrap();
    fs::write(root.join("src/a.rs"), "human-on-disk").unwrap();
    let vs_cp = project_diffs(
        DiffScope::VsCheckpoint,
        &events,
        Some(&root),
        None,
        &[],
        Some(&cp),
    );
    assert!(
        vs_cp.changes.iter().any(|c| {
            c.path.contains("a.rs") && c.attribution == "manual"
        }),
        "VsCheckpoint after human disk edit must list manual: {:?}",
        vs_cp.changes
    );

    // Combined picture: agent turn attribution still present alongside manual scopes
    assert!(turn.changes.iter().any(|c| c.attribution.starts_with("agent:")));
    assert!(buffer_model.changes.iter().any(|c| c.attribution == "manual"));
}

#[test]
fn checkpoints_create_restore_axes_no_git() {
    let db = Database::open_in_memory().unwrap();
    let project_root = temp("cp-proj");
    let store = temp("cp-store");
    fs::write(project_root.join("file.txt"), "v1").unwrap();
    let project = db.open_project(&project_root).unwrap();
    let chat = db.create_chat(&project.id, Some("c")).unwrap();
    db.append_event(&chat.id, None, "user_message", r#"{"text":"one"}"#)
        .unwrap();
    db.append_event(&chat.id, None, "user_message", r#"{"text":"two"}"#)
        .unwrap();

    let m = create_checkpoint(
        &project_root,
        &store,
        &project.id,
        Some(&chat.id),
        Some("run1"),
        Some(1),
    )
    .unwrap();
    assert!(store_is_outside_project(std::path::Path::new(&m.store_path), &project_root));
    assert!(!std::path::Path::new(&m.root_dir).join(".git").exists());
    assert!(!project_root.join(".git").join("refs").exists()); // we never create git

    // intervening manual
    fs::write(project_root.join("file.txt"), "manual").unwrap();
    let intervening = preview_intervening(&project_root, &m).unwrap();
    assert!(!intervening.is_empty());
    let warned = restore_files(&project_root, &m, false).unwrap();
    assert!(warned.warned && !warned.applied);

    let forced = restore_files(&project_root, &m, true).unwrap();
    assert!(forced.applied);
    assert_eq!(
        fs::read_to_string(project_root.join("file.txt")).unwrap(),
        "v1"
    );

    // Conversation only — soft-hide, no event delete
    let conv = restore_conversation(&db, &chat.id, Some(1)).unwrap();
    assert!(conv.applied);
    assert_eq!(conv.conversation_cursor, Some(1));
    assert_eq!(db.list_events(&chat.id, None).unwrap().len(), 2);
    assert_eq!(chat_event_cursor(&db, &chat.id).unwrap(), Some(1));

    // Both axes: rewrite files + conversation cursor via shipped restore_both
    fs::write(project_root.join("file.txt"), "diverged-again").unwrap();
    // Clear cursor so we can observe Both re-applying conversation axis
    db.set_setting(
        "chat",
        &format!("{}.event_cursor", chat.id),
        r#"{"hiddenAfterSeq": 99}"#,
    )
    .unwrap();
    assert_eq!(chat_event_cursor(&db, &chat.id).unwrap(), Some(99));

    let both = restore_both(&project_root, &db, &chat.id, &m, true).unwrap();
    assert_eq!(both.axis, RestoreAxis::Both, "{:?}", both);
    assert!(both.applied, "{:?}", both);
    assert!(
        both.files_restored.iter().any(|p| p.contains("file.txt")),
        "Both must restore files: {:?}",
        both.files_restored
    );
    assert_eq!(
        both.conversation_cursor,
        Some(1),
        "Both must set conversation cursor from manifest: {:?}",
        both
    );
    assert_eq!(
        fs::read_to_string(project_root.join("file.txt")).unwrap(),
        "v1",
        "Both must rewrite workspace files"
    );
    assert_eq!(
        chat_event_cursor(&db, &chat.id).unwrap(),
        Some(1),
        "Both must soft-hide via cursor without deleting events"
    );
    assert_eq!(
        db.list_events(&chat.id, None).unwrap().len(),
        2,
        "events remain durable after Both"
    );
    assert!(!project_root.join(".git").join("refs").exists());
}

/// Build-path `write` must auto-create a content-addressed checkpoint
/// under the runtime store (engine hook — not a manual create_checkpoint call).
#[test]
fn build_mutation_auto_creates_checkpoint_manifest() {
    let db = Database::open_in_memory().unwrap();
    let root = temp("auto-cp");
    let store = temp("auto-cp-store");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("auto")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root.clone()));
    rt.set_checkpoint_store_override(Some(store.clone()));

    assert!(
        fs::read_dir(&store).map(|d| d.count()).unwrap_or(0) == 0,
        "store must start empty"
    );

    let run = rt.start_run(&db, &chat.id, Mode::Build, None).unwrap();
    let res = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new(
                "write",
                json!({"path": "auto.txt", "content": "from-agent-build"}),
            ),
        )
        .unwrap();
    assert!(res.executed, "Build write must execute: {:?}", res);
    assert_eq!(
        fs::read_to_string(root.join("auto.txt")).unwrap(),
        "from-agent-build"
    );

    // Honest: engine create_checkpoint left a dir with manifest.json outside the project.
    let entries: Vec<_> = fs::read_dir(&store)
        .expect("store should exist after mutation")
        .flatten()
        .collect();
    assert!(
        !entries.is_empty(),
        "Build mutation must create checkpoint under store {:?}",
        store
    );
    let mut found = false;
    for e in &entries {
        let cp_dir = e.path();
        if !cp_dir.join("manifest.json").is_file() {
            continue;
        }
        let m = load_manifest(&cp_dir).expect("auto checkpoint manifest must parse");
        assert_eq!(m.project_id, project.id);
        assert_eq!(m.chat_id.as_deref(), Some(chat.id.as_str()));
        assert_eq!(m.run_id.as_deref(), Some(run.id.as_str()));
        assert!(
            m.files
                .iter()
                .any(|f| f.path == "auto.txt" || f.path.ends_with("auto.txt")),
            "checkpoint must include written file: {:?}",
            m.files
        );
        assert!(
            store_is_outside_project(std::path::Path::new(&m.store_path), &root),
            "checkpoint store must be outside project"
        );
        assert!(!cp_dir.join(".git").exists());
        found = true;
    }
    assert!(
        found,
        "no manifest.json under auto checkpoint store {:?}",
        store
    );
}

#[test]
fn agent_shell_still_requires_grant() {
    let db = Database::open_in_memory().unwrap();
    let root = temp("shell");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("s")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root));
    let run = rt.start_run(&db, &chat.id, Mode::Build, None).unwrap();
    let blocked = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("exec", json!({"command": "echo s7"})),
        )
        .unwrap();
    assert!(!blocked.executed);
    assert_eq!(
        blocked.content.get("status").and_then(|s| s.as_str()),
        Some("require_approval")
    );
    let still = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("exec", json!({"command": "echo s7"})),
        )
        .unwrap();
    assert!(!still.executed);
    rt.grant_approval(
        &db,
        &chat.id,
        Some(&run.id),
        Some(&project.id),
        ActionClass::Shell,
        GrantScope::AllowOnce,
    )
    .unwrap();
    let allowed = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("exec", json!({"command": "echo s7"})),
        )
        .unwrap();
    assert!(allowed.executed, "{:?}", allowed);
}

#[test]
fn apply_patch_hash_conflict_and_dirty_buffer_decision() {
    let root = temp("conflict");
    fs::write(root.join("x.txt"), "base").unwrap();
    let base_hash = content_hash_str("base");

    // Manual intervening edit
    fs::write(root.join("x.txt"), "manual").unwrap();
    let err = workspace_apply_patch(
        &root,
        &json!({
            "path": "x.txt",
            "content": "agent",
            "expected_hash": base_hash,
        }),
    )
    .unwrap_err();
    assert!(err.contains("hash_conflict"), "{err}");

    let current = content_hash_str("manual");
    let kind = evaluate_agent_patch_preflight(
        Some(&current),
        Some(&base_hash),
        Some(&DirtyBufferState {
            path: "x.txt".into(),
            dirty: true,
            buffer_hash: Some(content_hash_str("buffer")),
        }),
    );
    assert_eq!(kind, ConflictKind::DirtyAndIntervening);
    assert!(decide_patch_allowed(kind, None).is_err());
    assert!(decide_patch_allowed(kind, Some(ConflictOutcome::Cancel)).is_err());
    assert!(decide_patch_allowed(kind, Some(ConflictOutcome::OverwriteWithAgent)).is_ok());
}

#[test]
fn package_deps_no_monaco_no_xterm() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let pkg = fs::read_to_string(root.join("package.json")).unwrap();
    assert!(
        !pkg.contains("monaco-editor"),
        "monaco-editor must not be a dependency"
    );
    assert!(
        !pkg.contains("@xterm/xterm") && !pkg.contains("\"xterm\""),
        "xterm package must not be a dependency"
    );
}
