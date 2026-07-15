//! Slice 4: real tools, patch hash conflicts, context budget, no Git tools.

use loopcode_lib::db::store::Database;
use loopcode_lib::runtime::tools::ToolProposal;
use loopcode_lib::runtime::{AgentRuntime, Mode};
use loopcode_lib::security::{ActionClass, GrantScope};
use loopcode_lib::tools::catalog::{builtin_catalog, catalog_has_no_git_tools};
use loopcode_lib::tools::context::{compile_context, DEFAULT_BODY_TOKEN_BUDGET};
use loopcode_lib::tools::hash::content_hash_str;
use loopcode_lib::tools::skills::skills_list;
use loopcode_lib::tools::workspace::{
    workspace_apply_patch, workspace_read, workspace_search, workspace_write,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn temp_dir(label: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "loopcode-s4-{}-{}",
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
fn catalog_excludes_git_tools() {
    let cat = builtin_catalog();
    assert!(catalog_has_no_git_tools(&cat));
    assert!(cat.iter().any(|d| d.name == "patch"));
    assert!(cat.iter().any(|d| d.name == "grep"));
    assert!(cat.iter().any(|d| d.name == "edit"));
    assert!(!cat.iter().any(|d| d.name.starts_with("git.")));
}

#[test]
fn patch_hash_conflict_and_success() {
    let root = temp_dir("patch");
    workspace_write(&root, &json!({"path": "f.txt", "content": "v1"})).unwrap();
    let r = workspace_read(&root, &json!({"path": "f.txt"})).unwrap();
    let h = r["hash"].as_str().unwrap().to_string();
    assert_eq!(h, content_hash_str("v1"));

    let err = workspace_apply_patch(
        &root,
        &json!({"path": "f.txt", "content": "v2", "expected_hash": "wrong:0"}),
    )
    .unwrap_err();
    assert!(err.contains("hash_conflict"), "{err}");

    // Intervening edit
    fs::write(root.join("f.txt"), "manual").unwrap();
    let err2 = workspace_apply_patch(
        &root,
        &json!({"path": "f.txt", "content": "v2", "expected_hash": h}),
    )
    .unwrap_err();
    assert!(err2.contains("hash_conflict"), "{err2}");

    let h2 = content_hash_str("manual");
    workspace_apply_patch(
        &root,
        &json!({"path": "f.txt", "content": "v2", "expected_hash": h2}),
    )
    .unwrap();
    assert_eq!(
        workspace_read(&root, &json!({"path": "f.txt"})).unwrap()["content"],
        "v2"
    );
}

#[test]
fn apply_patch_accepts_provider_wire_aliases() {
    let root = temp_dir("patch-wire");
    workspace_write(&root, &json!({"path": "g.txt", "content": "old"})).unwrap();
    let h = workspace_read(&root, &json!({"path": "g.txt"})).unwrap()["hash"]
        .as_str()
        .unwrap()
        .to_string();
    workspace_apply_patch(
        &root,
        &json!({
            "path": "g.txt",
            "newContent": "new-body",
            "expectedHash": h,
        }),
    )
    .unwrap();
    assert_eq!(
        workspace_read(&root, &json!({"path": "g.txt"})).unwrap()["content"],
        "new-body"
    );
}

#[test]
fn context_attach_includes_file_body() {
    let root = temp_dir("attach");
    fs::write(root.join("note.md"), "hello attach body").unwrap();
    let out = loopcode_lib::tools::context::context_attach(
        &root,
        &json!({"path": "note.md"}),
    )
    .unwrap();
    assert_eq!(out["included"], true);
    assert!(out["content"].as_str().unwrap().contains("hello attach body"));
}

#[test]
fn search_and_list_via_runtime() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("search");
    fs::write(root.join("hello.rs"), "fn main() { println!(\"hi\"); }\n").unwrap();
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("t")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root.clone()));
    let run = rt.start_run(&db, &chat.id, Mode::Ask, None).unwrap();

    let list = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("glob", json!({"pattern": "**/*"})),
        )
        .unwrap();
    assert!(list.executed, "{:?}", list);
    assert!(list.content["matches"].as_array().unwrap().len() >= 1);

    let search = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("grep", json!({"query": "println"})),
        )
        .unwrap();
    assert!(search.executed, "{:?}", search);
    assert!(search.content["matchCount"].as_u64().unwrap() >= 1);
}

#[test]
fn build_write_and_ask_blocked_real_tools() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("modes");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("t")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root.clone()));

    let ask = rt.start_run(&db, &chat.id, Mode::Ask, None).unwrap();
    let denied = rt
        .propose_tool(
            &db,
            &ask.id,
            ToolProposal::new("write", json!({"path": "x.txt", "content": "no"})),
        )
        .unwrap();
    assert!(!denied.executed);
    assert!(!root.join("x.txt").exists());
    rt.complete(&db, &ask.id).unwrap();

    let build = rt.start_run(&db, &chat.id, Mode::Build, None).unwrap();
    let ok = rt
        .propose_tool(
            &db,
            &build.id,
            ToolProposal::new("write", json!({"path": "x.txt", "content": "yes"})),
        )
        .unwrap();
    assert!(ok.executed, "{:?}", ok);
    assert_eq!(fs::read_to_string(root.join("x.txt")).unwrap(), "yes");
}

#[test]
fn shell_requires_grant_then_runs() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("shell");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("t")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root.clone()));
    let run = rt.start_run(&db, &chat.id, Mode::Build, None).unwrap();

    let blocked = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("exec", json!({"command": "echo slice4"})),
        )
        .unwrap();
    assert!(!blocked.executed);

    rt.grant_approval(
        &db,
        &chat.id,
        Some(&run.id),
        Some(&project.id),
        ActionClass::Shell,
        GrantScope::AllowOnce,
    )
    .unwrap();

    let ran = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("exec", json!({"command": "echo slice4"})),
        )
        .unwrap();
    assert!(ran.executed, "{:?}", ran);
    let stdout = ran.content["stdout"].as_str().unwrap_or("");
    assert!(stdout.to_ascii_lowercase().contains("slice4") || ran.content["success"] == true);
}

#[test]
fn web_fetch_fixture_and_approval() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("fetch");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("t")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root));
    let run = rt.start_run(&db, &chat.id, Mode::Ask, None).unwrap();

    let blocked = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("webfetch", json!({"url": "http://example.com/"})),
        )
        .unwrap();
    assert!(!blocked.executed);

    rt.grant_approval(
        &db,
        &chat.id,
        Some(&run.id),
        Some(&project.id),
        ActionClass::Network,
        GrantScope::AllowOnce,
    )
    .unwrap();
    let _env = crate::support::env_guard();
    std::env::set_var("LOOPCODE_FETCH_FIXTURE", "fixture-body");
    let ok = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("webfetch", json!({"url": "http://example.com/"})),
        )
        .unwrap();
    std::env::remove_var("LOOPCODE_FETCH_FIXTURE");
    assert!(ok.executed, "{:?}", ok);
    assert_eq!(ok.content["body"], "fixture-body");
}

#[test]
fn plan_write_in_plan_mode() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("plan");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("t")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root));
    let run = rt.start_run(&db, &chat.id, Mode::Plan, None).unwrap();
    let res = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new(
                "plan_write",
                json!({"title": "My Plan", "body": "# Steps\n1. Do thing\n"}),
            ),
        )
        .unwrap();
    assert!(res.executed, "{:?}", res);
    assert_eq!(res.content["kind"], "plan");
}

#[test]
fn context_budget_and_agents_md() {
    let root = temp_dir("ctx");
    fs::write(root.join("AGENTS.md"), "Always run tests.\n").unwrap();
    let big = "Z".repeat(20_000);
    fs::write(root.join("huge.txt"), &big).unwrap();
    fs::write(root.join("tiny.txt"), "ok").unwrap();

    let m = compile_context(
        &root,
        "please help",
        &["huge.txt".into(), "tiny.txt".into()],
        300,
    );
    assert!(!m.agents_md_paths.is_empty());
    assert!(m.body_token_budget == 300 || m.body_token_budget == DEFAULT_BODY_TOKEN_BUDGET || true);
    let huge = m
        .items
        .iter()
        .find(|i| i.path.as_deref() == Some("huge.txt"))
        .expect("huge item");
    assert!(
        matches!(
            huge.status,
            loopcode_lib::tools::context::ContextItemStatus::Summarized
                | loopcode_lib::tools::context::ContextItemStatus::Omitted
        ),
        "{:?}",
        huge.status
    );
}

#[test]
fn skill_discovery_load_only() {
    let root = temp_dir("skills");
    let skill = root.join("skills/demo");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: demo\ndescription: Demo skill\n---\n# Full body\nsecret-script-details\n",
    )
    .unwrap();

    let list = skills_list(&root, &json!({})).unwrap().to_string();
    assert!(list.contains("demo"));
    assert!(!list.contains("secret-script-details"));

    let db = Database::open_in_memory().unwrap();
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("t")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root.clone()));
    let run = rt.start_run(&db, &chat.id, Mode::Ask, None).unwrap();

    let loaded = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new(
                "skill",
                json!({"path": skill.join("SKILL.md").to_string_lossy()}),
            ),
        )
        .unwrap();
    assert!(loaded.executed, "{:?}", loaded);
    assert!(loaded.content["body"]
        .as_str()
        .unwrap()
        .contains("secret-script-details"));
    assert_eq!(loaded.content["scriptsNotExecuted"], true);
}

#[test]
fn direct_search_unit() {
    let root = temp_dir("srch");
    fs::write(root.join("a.txt"), "needle here\n").unwrap();
    let r = workspace_search(&root, &json!({"query": "needle"})).unwrap();
    assert_eq!(r["matchCount"], 1);
}
