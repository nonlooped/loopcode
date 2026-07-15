//! Slice 6: cockpit timeline projection, composer run path, approvals, keyboard.

use loopcode_lib::cockpit::keyboard::{
    default_keyboard_map, required_expert_commands, CockpitCommand, KeyboardRegistry,
};
use loopcode_lib::cockpit::timeline::{project_timeline, TimelineItemKind, TimelineViewState};
use loopcode_lib::db::store::Database;
use loopcode_lib::runtime::tools::ToolProposal;
use loopcode_lib::runtime::{AgentRuntime, Mode};
use loopcode_lib::security::approval::GrantScope;
use loopcode_lib::security::ActionClass;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn temp_dir(label: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "loopcode-s6-{}-{}",
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
fn timeline_empty_states_distinct() {
    let none = project_timeline(&[], &[], false, false, None);
    assert_eq!(none.state, TimelineViewState::EmptyStart);
    let chat = project_timeline(&[], &[], true, false, None);
    assert_eq!(chat.state, TimelineViewState::EmptyChat);
    assert_ne!(none.status_label, chat.status_label);
    let loading = project_timeline(&[], &[], true, true, None);
    assert_eq!(loading.state, TimelineViewState::Loading);
    let err = project_timeline(&[], &[], true, false, Some("boom"));
    assert_eq!(err.state, TimelineViewState::Error);
}

#[test]
fn timeline_from_real_core_events() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("tl");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("Cockpit chat")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root));
    // Build mode: shell is require_approval (Ask mode hard-denies mutating/shell).
    let run = rt
        .start_run(&db, &chat.id, Mode::Build, Some("gpt-test"))
        .unwrap();
    db.append_event(
        &chat.id,
        Some(&run.id),
        "user_message",
        &json!({"text": "hello cockpit"}).to_string(),
    )
    .unwrap();
    // shell requires approval → tool.rejected with nested content.status=require_approval
    let blocked = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("exec", json!({"command": "echo hi"})),
        )
        .unwrap();
    assert!(!blocked.executed);
    assert_eq!(
        blocked
            .content
            .get("status")
            .and_then(|s| s.as_str()),
        Some("require_approval"),
        "Build+shell must require approval, got {:?}",
        blocked.content
    );

    let events = db.list_events(&chat.id, None).unwrap();
    let runs = db.list_runs(&chat.id).unwrap();
    assert!(!events.is_empty());
    let proj = project_timeline(&events, &runs, true, false, None);
    assert_eq!(proj.state, TimelineViewState::Populated);
    assert!(
        proj.items
            .iter()
            .any(|i| i.kind == TimelineItemKind::UserMessage && i.body.contains("hello cockpit")),
        "{:?}",
        proj.items
    );
    assert!(
        proj.items
            .iter()
            .any(|i| i.kind == TimelineItemKind::ToolProposal),
        "expected tool proposal: {:?}",
        proj.items.iter().map(|i| &i.kind).collect::<Vec<_>>()
    );
    assert!(
        proj.items.iter().any(|i| {
            i.needs_approval
                && i.kind == TimelineItemKind::Approval
                && i.action_class.as_deref() == Some("shell")
        }),
        "expected nested tool.rejected → needs_approval shell card: {:?}",
        proj.items
    );
}

#[test]
fn composer_start_run_with_mode_and_model() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("compose");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("send")).unwrap();
    let mut rt = AgentRuntime::new();
    let run = rt
        .start_run(&db, &chat.id, Mode::Build, Some("model-x"))
        .unwrap();
    assert_eq!(run.mode, "build");
    assert_eq!(run.model.as_deref(), Some("model-x"));
    db.append_event(
        &chat.id,
        Some(&run.id),
        "user_message",
        &json!({"text": "go", "mode": "build"}).to_string(),
    )
    .unwrap();
    let events = db.list_events(&chat.id, None).unwrap();
    assert!(events.iter().any(|e| e.kind == "user_message"));

    // stop path
    let cancelled = rt.request_cancel(&db, &run.id).unwrap();
    assert_eq!(cancelled.status.as_str(), "cancelled");
}

#[test]
fn approval_grant_allows_shell_once() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("appr");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("a")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root));
    let run = rt.start_run(&db, &chat.id, Mode::Build, None).unwrap();

    let blocked = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("exec", json!({"command": "echo s6"})),
        )
        .unwrap();
    assert!(!blocked.executed);
    assert!(
        blocked
            .content
            .get("status")
            .and_then(|s| s.as_str())
            == Some("require_approval")
            || blocked.error_message.as_deref().unwrap_or("").contains("approval")
    );

    // Projection must surface a true approval card from nested tool.rejected content,
    // not merely lifecycle.approval_waiting misclassified as Approval.
    let events = db.list_events(&chat.id, None).unwrap();
    assert!(
        events.iter().any(|e| e.kind == "tool.rejected"),
        "expected tool.rejected event from propose_tool: {:?}",
        events.iter().map(|e| &e.kind).collect::<Vec<_>>()
    );
    assert!(
        events.iter().any(|e| e.kind == "run.lifecycle.approval_waiting"),
        "lifecycle.approval_waiting should still be present for chrome status"
    );
    let proj = project_timeline(&events, &db.list_runs(&chat.id).unwrap(), true, false, None);

    // Lifecycle must stay Lifecycle (never Approval with needs_approval=false theater).
    for item in proj.items.iter().filter(|i| i.event_kind.contains("lifecycle")) {
        assert_eq!(
            item.kind,
            TimelineItemKind::Lifecycle,
            "lifecycle event misclassified: {:?}",
            item
        );
        assert!(
            !item.needs_approval,
            "lifecycle must not set needs_approval: {:?}",
            item
        );
    }

    let approval = proj
        .items
        .iter()
        .find(|i| i.needs_approval)
        .expect("expected needs_approval card from nested tool.rejected content");
    assert_eq!(
        approval.kind,
        TimelineItemKind::Approval,
        "approval card kind: {:?}",
        approval
    );
    assert_eq!(
        approval.action_class.as_deref(),
        Some("shell"),
        "action_class from nested content: {:?}",
        approval
    );
    assert_eq!(
        approval.tool_name.as_deref(),
        Some("exec"),
        "tool_name: {:?}",
        approval
    );
    assert_eq!(approval.event_kind, "tool.rejected");

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
            ToolProposal::new("exec", json!({"command": "echo s6"})),
        )
        .unwrap();
    assert!(allowed.executed, "{:?}", allowed);
}

#[test]
fn keyboard_map_registers_expert_bindings() {
    let map = default_keyboard_map();
    assert!(map.len() >= 7);
    let reg = KeyboardRegistry::with_defaults();
    for cmd in required_expert_commands() {
        assert!(
            reg.registered_commands().contains(cmd),
            "missing {cmd:?}"
        );
    }
    assert_eq!(
        reg.resolve(true, false, false, "n"),
        Some(CockpitCommand::NewChat)
    );
    assert_eq!(
        reg.resolve(true, false, false, "l"),
        Some(CockpitCommand::FocusComposer)
    );
    assert_eq!(
        reg.resolve(true, false, false, "b"),
        Some(CockpitCommand::ToggleLeft)
    );
    assert_eq!(
        reg.resolve(true, true, false, "b"),
        Some(CockpitCommand::ToggleRight)
    );
    assert_eq!(
        reg.resolve(true, false, true, "m"),
        Some(CockpitCommand::ModeCycle)
    );
    assert_eq!(
        reg.resolve(false, false, false, "escape"),
        Some(CockpitCommand::StopRun)
    );
}
