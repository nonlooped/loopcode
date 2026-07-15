//! Slice 2: agent runtime state machine tests against real shipped code + SQLite.

use loopcode_lib::db::store::Database;
use loopcode_lib::domain::RunStatus;
use loopcode_lib::runtime::error::{ErrorCategory, SideEffectCertainty};
use loopcode_lib::runtime::events::kinds;
use loopcode_lib::runtime::mode::{mode_allows_tool_effect, Mode, ToolEffect};
use loopcode_lib::runtime::tools::{ToolOutcomeKind, ToolProposal};
use loopcode_lib::tools::hash::content_hash_str;
use loopcode_lib::runtime::AgentRuntime;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn temp_dir(label: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "loopcode-s2-{}-{}",
        label,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&p).unwrap();
    p
}

fn open_chat(db: &Database) -> (String, String) {
    let dir = temp_dir("ws");
    let project = db.open_project(&dir).unwrap();
    let chat = db.create_chat(&project.id, Some("runtime")).unwrap();
    (project.id, chat.id)
}

/// Drive the shipped `mode_allows_tool_effect` for every mode × effect matrix cell.
#[test]
fn mode_mutation_boundaries_matrix() {
    // Read-only allowed everywhere
    for mode in [Mode::Ask, Mode::Plan, Mode::Build, Mode::Debug] {
        assert!(
            mode_allows_tool_effect(mode, ToolEffect::ReadOnly),
            "{:?} should allow read-only",
            mode
        );
    }
    // Mutating only Build
    assert!(!mode_allows_tool_effect(Mode::Ask, ToolEffect::Mutating));
    assert!(!mode_allows_tool_effect(Mode::Plan, ToolEffect::Mutating));
    assert!(mode_allows_tool_effect(Mode::Build, ToolEffect::Mutating));
    assert!(!mode_allows_tool_effect(Mode::Debug, ToolEffect::Mutating));
    // Diagnostic: Build + Debug only
    assert!(!mode_allows_tool_effect(Mode::Ask, ToolEffect::Diagnostic));
    assert!(!mode_allows_tool_effect(Mode::Plan, ToolEffect::Diagnostic));
    assert!(mode_allows_tool_effect(Mode::Build, ToolEffect::Diagnostic));
    assert!(mode_allows_tool_effect(Mode::Debug, ToolEffect::Diagnostic));
}

#[test]
fn mock_runner_mode_boundaries_execute_vs_reject() {
    let db = Database::open_in_memory().unwrap();
    let (_pid, chat_id) = open_chat(&db);
    let mut rt = AgentRuntime::new();

    let cases: &[(Mode, &str, bool)] = &[
        // mode, tool, expect_executed
        (Mode::Ask, "read", true),
        (Mode::Ask, "write", false),
        (Mode::Plan, "glob", true),
        (Mode::Plan, "patch", false),
        (Mode::Build, "read", true),
        (Mode::Build, "write", true),
        (Mode::Debug, "read", true),
        (Mode::Debug, "write", false),
    ];

    for (mode, tool, expect_exec) in cases {
        let run = rt
            .start_run(&db, &chat_id, *mode, Some("mock-model"))
            .unwrap();
        assert_eq!(run.status, RunStatus::ModelActive);

        // Real Slice-4 tools need valid args; create a file for read/list paths.
        let ws = std::env::temp_dir().join(format!("s2-mode-{}", run.id));
        let _ = std::fs::create_dir_all(&ws);
        let _ = std::fs::write(ws.join("a.txt"), "hello");
        rt.set_project_root_override(Some(ws.clone()));
        let args = if *tool == "write" || *tool == "patch" {
            json!({
                "path": "a.txt",
                "content": "mutated",
                "expectedHash": content_hash_str("hello"),
            })
        } else if *tool == "glob" {
            json!({"pattern": "**/*"})
        } else {
            json!({"path": "a.txt"})
        };
        let proposal = ToolProposal::new(*tool, args);
        let result = rt.propose_tool(&db, &run.id, proposal).unwrap();
        assert_eq!(
            result.executed, *expect_exec,
            "mode={:?} tool={tool} expected executed={expect_exec}",
            mode
        );
        if *expect_exec {
            assert_eq!(result.kind, ToolOutcomeKind::Executed);
        } else {
            assert_eq!(result.kind, ToolOutcomeKind::Rejected);
            let err = result.error_message.as_deref().unwrap_or("");
            let status = result.content.get("status").and_then(|v| v.as_str());
            assert!(
                err.contains("mode_policy")
                    || err.contains("denied by mode")
                    || status == Some("mode_policy_denied")
                    || status == Some("deny"),
                "structured reject for {:?}",
                result
            );
            // Run must not crash — still non-fatal
            let after = db.get_run(&run.id).unwrap().unwrap();
            assert!(
                !after.status.is_terminal() || after.status == RunStatus::ModelActive,
                "reject must not fail the run, got {:?}",
                after.status
            );
        }
        // Finish run so next start is clean
        rt.complete(&db, &run.id).unwrap();
    }
}

#[test]
fn lifecycle_emits_semantic_events() {
    let db = Database::open_in_memory().unwrap();
    let (_pid, chat_id) = open_chat(&db);
    let mut rt = AgentRuntime::new();
    let run = rt.start_run(&db, &chat_id, Mode::Ask, None).unwrap();

    let proposal = ToolProposal::new("glob", json!({"pattern": "**/*"}));
    let _ = rt.propose_tool(&db, &run.id, proposal).unwrap();
    rt.complete(&db, &run.id).unwrap();

    let events = db.list_events(&chat_id, None).unwrap();
    let kinds: Vec<&str> = events.iter().map(|e| e.kind.as_str()).collect();
    assert!(
        kinds.contains(&kinds::RUN_QUEUED) || kinds.iter().any(|k| k.contains("queued")),
        "expected queued lifecycle, got {kinds:?}"
    );
    assert!(kinds.iter().any(|k| *k == kinds::RUN_PREPARING || *k == kinds::RUN_MODEL_ACTIVE));
    assert!(kinds.contains(&kinds::TOOL_PROPOSAL));
    assert!(kinds.contains(&kinds::TOOL_RESULT));
    assert!(kinds.contains(&kinds::RUN_COMPLETED));
    // Monotonic seq
    for w in events.windows(2) {
        assert!(w[0].seq < w[1].seq);
    }
}

#[test]
fn cancel_stops_further_tool_execution() {
    let db = Database::open_in_memory().unwrap();
    let (_pid, chat_id) = open_chat(&db);
    let mut rt = AgentRuntime::new();
    let run = rt.start_run(&db, &chat_id, Mode::Build, None).unwrap();

    // One successful tool before cancel
    let ws = std::env::temp_dir().join(format!("s2-cancel-{}", run.id));
    let _ = std::fs::create_dir_all(&ws);
    rt.set_project_root_override(Some(ws));
    let r1 = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("write", json!({"path": "x.txt", "content": "x"})),
        )
        .unwrap();
    assert!(r1.executed, "{:?}", r1);

    let cancelled = rt.request_cancel(&db, &run.id).unwrap();
    assert_eq!(
        cancelled.status,
        RunStatus::Cancelled,
        "known-stopped cancel should be cancelled"
    );

    // Further proposals must not execute
    let r2 = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("write", json!({"path": "y.txt", "content": "y"})),
        );
    // propose on cancelled run should error OR return non-executed
    match r2 {
        Ok(res) => {
            assert!(!res.executed, "no tool execution after cancel");
        }
        Err(e) => {
            assert!(
                e.contains("status") || e.contains("cancelled") || e.contains("terminal"),
                "unexpected err: {e}"
            );
        }
    }
    assert!(
        rt.post_cancel_executions().is_empty(),
        "no post-cancel executions recorded: {:?}",
        rt.post_cancel_executions()
    );

    let events = db.list_events(&chat_id, None).unwrap();
    assert!(events.iter().any(|e| e.kind == kinds::CANCEL_REQUESTED || e.kind == kinds::RUN_CANCELLED));
}

#[test]
fn cancel_with_uncertain_side_effects_suspends() {
    let db = Database::open_in_memory().unwrap();
    let (_pid, chat_id) = open_chat(&db);
    let mut rt = AgentRuntime::new();
    let run = rt.start_run(&db, &chat_id, Mode::Build, None).unwrap();
    rt.mark_uncertain_side_effect(&run.id);
    let after = rt.request_cancel(&db, &run.id).unwrap();
    assert_eq!(after.status, RunStatus::Suspended);
    assert_ne!(after.status, RunStatus::Completed);
    assert_ne!(after.status, RunStatus::Cancelled);

    let events = db.list_events(&chat_id, None).unwrap();
    let err = events.iter().find(|e| e.kind == kinds::ERROR).expect("error event");
    assert!(err.payload_json.contains("unknown") || err.payload_json.contains("Unknown"));
}

#[test]
fn crash_suspend_fixture_never_completed() {
    let dir = temp_dir("suspend");
    let db_path = dir.join("app.sqlite");
    let chat_id;
    let run_id;
    {
        let db = Database::open(&db_path).unwrap();
        let (_pid, cid) = open_chat(&db);
        chat_id = cid;
        let mut rt = AgentRuntime::new();
        let run = rt
            .start_run(&db, &chat_id, Mode::Build, Some("m"))
            .unwrap();
        run_id = run.id.clone();
        // Simulate mid tool-active work
        rt.enter_phase(&db, &run_id, RunStatus::ToolActive).unwrap();
        assert_eq!(
            db.get_run(&run_id).unwrap().unwrap().status,
            RunStatus::ToolActive
        );
        // Crash: drop without completing
    }

    // Reopen + real reconcile path used by the app
    {
        let db = Database::open(&db_path).unwrap();
        let n = AgentRuntime::reconcile_after_restart(&db).unwrap();
        assert!(n >= 1, "expected at least one suspended run, n={n}");
        let run = db.get_run(&run_id).unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Suspended);
        assert_ne!(run.status, RunStatus::Completed);
        assert_ne!(run.status, RunStatus::Cancelled);
        assert!(!run.status.is_terminal());
    }
}

#[test]
fn open_app_database_source_calls_reconcile() {
    // Structural: shipped entry uses AgentRuntime::reconcile_after_restart
    let lib = include_str!("../src/lib.rs");
    assert!(
        lib.contains("reconcile_after_restart"),
        "open_app_database must call runtime reconcile_after_restart"
    );
    // Behavioral: same function the entry path invokes
    let dir = temp_dir("openapp");
    let db_path = dir.join("app.sqlite");
    let run_id;
    {
        let db = Database::open(&db_path).unwrap();
        let ws = dir.join("ws");
        fs::create_dir_all(&ws).unwrap();
        let project = db.open_project(&ws).unwrap();
        let chat = db.create_chat(&project.id, Some("c")).unwrap();
        let mut rt = AgentRuntime::new();
        let run = rt.start_run(&db, &chat.id, Mode::Ask, None).unwrap();
        run_id = run.id;
    }
    let db = Database::open(&db_path).unwrap();
    AgentRuntime::reconcile_after_restart(&db).unwrap();
    let run = db.get_run(&run_id).unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Suspended);
}

#[test]
fn error_taxonomy_on_fail() {
    let db = Database::open_in_memory().unwrap();
    let (_pid, chat_id) = open_chat(&db);
    let mut rt = AgentRuntime::new();
    let run = rt.start_run(&db, &chat_id, Mode::Ask, None).unwrap();
    let err = loopcode_lib::runtime::engine::internal_runtime_error("mock failure");
    assert_eq!(err.category, ErrorCategory::Runtime);
    assert_eq!(err.side_effect_certainty, SideEffectCertainty::Unknown);
    let failed = rt.fail(&db, &run.id, err).unwrap();
    assert_eq!(failed.status, RunStatus::Failed);
    let events = db.list_events(&chat_id, None).unwrap();
    assert!(events.iter().any(|e| e.kind == kinds::ERROR));
    assert!(events.iter().any(|e| e.kind == kinds::RUN_FAILED));
}

#[test]
fn single_active_run_suspends_prior() {
    let db = Database::open_in_memory().unwrap();
    let (_pid, chat_id) = open_chat(&db);
    let mut rt = AgentRuntime::new();
    let r1 = rt.start_run(&db, &chat_id, Mode::Ask, None).unwrap();
    let r2 = rt.start_run(&db, &chat_id, Mode::Plan, None).unwrap();
    let first = db.get_run(&r1.id).unwrap().unwrap();
    assert_eq!(first.status, RunStatus::Suspended);
    assert_eq!(r2.status, RunStatus::ModelActive);
}

#[test]
fn cancel_one_run_does_not_poison_tools_for_other_chat() {
    let db = Database::open_in_memory().unwrap();
    let dir_a = temp_dir("chat-a");
    let dir_b = temp_dir("chat-b");
    let proj_a = db.open_project(&dir_a).unwrap();
    let proj_b = db.open_project(&dir_b).unwrap();
    let chat_a = db.create_chat(&proj_a.id, Some("A")).unwrap();
    let chat_b = db.create_chat(&proj_b.id, Some("B")).unwrap();
    let mut rt = AgentRuntime::new();

    // Two chats, both ModelActive (single-active is per-chat, not global)
    let run_a = rt.start_run(&db, &chat_a.id, Mode::Build, None).unwrap();
    // start_run resets tools; keep run_a active by not completing, start B on same runtime
    // without going through start_run's tool reset after cancel path — use create + transition
    // so both remain active under one AgentRuntime instance.
    let run_b = db
        .create_run(&chat_b.id, "build", None, Some("mock"), Some("slice2-v1"))
        .unwrap();
    rt.transition(&db, &run_b.id, RunStatus::Preparing).unwrap();
    rt.transition(&db, &run_b.id, RunStatus::ModelActive).unwrap();

    assert_eq!(db.get_run(&run_a.id).unwrap().unwrap().status, RunStatus::ModelActive);
    assert_eq!(db.get_run(&run_b.id).unwrap().unwrap().status, RunStatus::ModelActive);

    // Cancel only A
    let cancelled = rt.request_cancel(&db, &run_a.id).unwrap();
    assert_eq!(cancelled.status, RunStatus::Cancelled);

    // B must still execute tools (cancel is run-scoped)
    let ws = std::env::temp_dir().join(format!("s2-iso-{}", run_b.id));
    let _ = std::fs::create_dir_all(&ws);
    rt.set_project_root_override(Some(ws));
    let result = rt
        .propose_tool(
            &db,
            &run_b.id,
            ToolProposal::new("write", json!({"path": "b.txt", "content": "b"})),
        )
        .unwrap();
    assert!(
        result.executed,
        "cancel of run A must not block tools for run B: {:?}",
        result
    );
    assert_eq!(result.kind, ToolOutcomeKind::Executed);
}

#[test]
fn terminal_run_cannot_become_suspended() {
    let db = Database::open_in_memory().unwrap();
    let (_pid, chat_id) = open_chat(&db);
    let mut rt = AgentRuntime::new();
    let run = rt.start_run(&db, &chat_id, Mode::Ask, None).unwrap();
    rt.complete(&db, &run.id).unwrap();
    assert_eq!(
        db.get_run(&run.id).unwrap().unwrap().status,
        RunStatus::Completed
    );

    let err = rt
        .transition(&db, &run.id, RunStatus::Suspended)
        .expect_err("completed → suspended must be rejected");
    assert!(
        err.contains("cannot transition terminal"),
        "unexpected error: {err}"
    );
    assert_eq!(
        db.get_run(&run.id).unwrap().unwrap().status,
        RunStatus::Completed,
        "status must remain completed"
    );

    // Idempotent same-terminal is OK
    let same = rt.transition(&db, &run.id, RunStatus::Completed).unwrap();
    assert_eq!(same.status, RunStatus::Completed);
}
