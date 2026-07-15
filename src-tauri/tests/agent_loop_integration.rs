//! Integration tests for the shipped agent loop, tool rounds, cancel, MCP, HTTPS fetch.

use loopcode_lib::db::store::Database;
use loopcode_lib::domain::RunStatus;
use loopcode_lib::extensibility::{
    grant_mcp_server_trust, invoke_mcp_http_with_transport, invoke_mcp_transport, mcp_call,
    register_mcp_server, McpTransport,
};
use loopcode_lib::providers::chat::{openai_assistant_text_body, openai_assistant_tool_body};
use loopcode_lib::providers::transport::FixtureHttpTransport;
use loopcode_lib::runtime::agent_loop::{execute_agent_run, AgentLoopInput};
use loopcode_lib::runtime::events::kinds;
use loopcode_lib::runtime::mode::Mode;
use loopcode_lib::runtime::AgentRuntime;
use loopcode_lib::tools::network::web_fetch_with_transport;
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn temp_project(label: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "loopcode-int-{}-{}",
        label,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn open_chat_with_files(db: &Database) -> (String, PathBuf) {
    let dir = temp_project("ws");
    std::fs::write(dir.join("readme.md"), "# Demo project\n").unwrap();
    let project = db.open_project(&dir).unwrap();
    let chat = db.create_chat(&project.id, Some("integration")).unwrap();
    (chat.id, dir)
}

#[test]
fn send_path_text_turn_completes_with_provider_not_mock() {
    let db = Database::open_in_memory().unwrap();
    let (chat_id, root) = open_chat_with_files(&db);
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root.clone()));

    let body = openai_assistant_text_body("This is a demo project with a readme.", "gpt-fixture");
    let transport = FixtureHttpTransport::single(200, body);

    let run = rt
        .start_run_with_provider(&db, &chat_id, Mode::Ask, Some("gpt-fixture"), "openai")
        .unwrap();
    assert_eq!(run.adapter.as_deref(), Some("openai"));
    assert_ne!(run.adapter.as_deref(), Some("mock"));

    db.append_event(
        &chat_id,
        Some(&run.id),
        "user_message",
        &json!({"text": "What is this project?"}).to_string(),
    )
    .unwrap();

    let finished = execute_agent_run(
        &db,
        &mut rt,
        AgentLoopInput {
            run_id: &run.id,
            user_text: "What is this project?",
            provider_id: "openai",
            model: "gpt-fixture",
            api_key: "sk-fixture",
            project_root: Some(&root),
            transport: &transport,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            base_url_override: None,
            reasoning: None,
        },
    )
    .unwrap();

    assert_eq!(finished.status, RunStatus::Completed);
    assert_eq!(finished.adapter.as_deref(), Some("openai"));

    let events = db.list_events(&chat_id, None).unwrap();
    assert!(
        events.iter().any(|e| e.kind == "assistant_message"),
        "expected assistant_message event"
    );
    assert!(events.iter().any(|e| e.kind == kinds::RUN_COMPLETED));
    let assistant = events
        .iter()
        .find(|e| e.kind == "assistant_message")
        .unwrap();
    assert!(assistant.payload_json.contains("demo project"));
}

#[test]
fn provider_http_error_fails_run_not_hang() {
    let db = Database::open_in_memory().unwrap();
    let (chat_id, root) = open_chat_with_files(&db);
    let mut rt = AgentRuntime::new();
    let transport = FixtureHttpTransport::single(500, r#"{"error":{"message":"server exploded"}}"#);
    let run = rt
        .start_run_with_provider(&db, &chat_id, Mode::Ask, Some("m"), "opencode")
        .unwrap();
    let finished = execute_agent_run(
        &db,
        &mut rt,
        AgentLoopInput {
            run_id: &run.id,
            user_text: "hi",
            provider_id: "opencode",
            model: "m",
            api_key: "k",
            project_root: Some(&root),
            transport: &transport,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            base_url_override: None,
            reasoning: None,
        },
    )
    .unwrap();
    assert_eq!(finished.status, RunStatus::Failed);
    assert_eq!(finished.adapter.as_deref(), Some("opencode"));
    let events = db.list_events(&chat_id, None).unwrap();
    assert!(events
        .iter()
        .any(|e| e.kind == kinds::ERROR || e.kind == kinds::RUN_FAILED));
}

#[test]
fn tool_loop_glob_real_fs() {
    let db = Database::open_in_memory().unwrap();
    let (chat_id, root) = open_chat_with_files(&db);
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root.clone()));

    let tool_body = openai_assistant_tool_body(
        "glob",
        &json!({"pattern": "**/*"}),
        "gpt-fixture",
        "call_ws_1",
    );
    let final_body = openai_assistant_text_body("The project contains readme.md.", "gpt-fixture");
    let transport = FixtureHttpTransport::new(vec![(200, tool_body), (200, final_body)]);

    let run = rt
        .start_run_with_provider(&db, &chat_id, Mode::Ask, Some("gpt-fixture"), "openai")
        .unwrap();
    let finished = execute_agent_run(
        &db,
        &mut rt,
        AgentLoopInput {
            run_id: &run.id,
            user_text: "List the project files",
            provider_id: "openai",
            model: "gpt-fixture",
            api_key: "sk",
            project_root: Some(&root),
            transport: &transport,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            base_url_override: None,
            reasoning: None,
        },
    )
    .unwrap();
    assert_eq!(finished.status, RunStatus::Completed);

    let events = db.list_events(&chat_id, None).unwrap();
    assert!(events.iter().any(|e| e.kind == kinds::TOOL_PROPOSAL));
    assert!(events.iter().any(|e| e.kind == kinds::TOOL_RESULT));
    let result = events
        .iter()
        .find(|e| e.kind == kinds::TOOL_RESULT)
        .unwrap();
    assert!(
        result.payload_json.contains("readme.md"),
        "expected real list entries in tool result: {}",
        result.payload_json
    );
}

#[test]
fn cancel_during_delayed_model_request() {
    let db = Database::open_in_memory().unwrap();
    let (chat_id, root) = open_chat_with_files(&db);
    let mut rt = AgentRuntime::new();
    let flag = Arc::new(AtomicBool::new(false));
    let transport =
        FixtureHttpTransport::single(200, openai_assistant_text_body("should not finish", "m"))
            .with_delay(Duration::from_millis(400))
            .with_cancel_flag(Arc::clone(&flag));

    let run = rt
        .start_run_with_provider(&db, &chat_id, Mode::Ask, Some("m"), "openai")
        .unwrap();
    let run_id = run.id.clone();

    // Flip cooperative cancel mid-request (while transport is delayed).
    let cancel_flag = Arc::clone(&flag);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        cancel_flag.store(true, Ordering::SeqCst);
    });

    let finished = execute_agent_run(
        &db,
        &mut rt,
        AgentLoopInput {
            run_id: &run_id,
            user_text: "hi",
            provider_id: "openai",
            model: "m",
            api_key: "sk",
            project_root: Some(&root),
            transport: &transport,
            cancel_flag: Arc::clone(&flag),
            base_url_override: None,
            reasoning: None,
        },
    )
    .unwrap();

    assert_eq!(finished.status, RunStatus::Cancelled);
    // No tool execution after cancel.
    assert!(rt.post_cancel_executions().is_empty());
    let events = db.list_events(&chat_id, None).unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.kind == kinds::CANCEL_REQUESTED || e.kind == kinds::RUN_CANCELLED),
        "expected cancel lifecycle events"
    );
    // Must not complete with assistant text from the delayed body.
    assert!(!events.iter().any(|e| {
        e.kind == "assistant_message" && e.payload_json.contains("should not finish")
    }));
}

/// Product cancel path: start a delayed turn via concurrent host driver (send-equivalent),
/// then cancel through `product_request_cancel` (same code as `runtime_cancel_run`).
#[test]
fn product_cancel_via_runtime_cancel_mid_send_equivalent_turn() {
    use loopcode_lib::runtime::agent_loop::{
        execute_agent_run_concurrent, product_request_cancel, AgentLoopOwned,
    };
    use std::collections::HashMap;
    use std::sync::Mutex;

    let db = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
    let rt = Arc::new(Mutex::new(AgentRuntime::new()));
    let flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>> = Arc::new(Mutex::new(HashMap::new()));

    let (chat_id, root, run_id, cancel_flag) = {
        let db_g = db.lock().unwrap();
        let (chat_id, root) = open_chat_with_files(&db_g);
        let mut rt_g = rt.lock().unwrap();
        rt_g.set_project_root_override(Some(root.clone()));
        let run = rt_g
            .start_run_with_provider(&db_g, &chat_id, Mode::Ask, Some("m"), "openai")
            .unwrap();
        db_g.append_event(
            &chat_id,
            Some(&run.id),
            "user_message",
            &json!({"text": "hi"}).to_string(),
        )
        .unwrap();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        flags
            .lock()
            .unwrap()
            .insert(run.id.clone(), Arc::clone(&cancel_flag));
        (chat_id, root, run.id.clone(), cancel_flag)
    };

    let owned = AgentLoopOwned {
        run_id: run_id.clone(),
        user_text: "hi".into(),
        provider_id: "openai".into(),
        model: "m".into(),
        api_key: "sk".into(),
        project_root: Some(root),
        cancel_flag: Arc::clone(&cancel_flag),
        base_url_override: None,
        reasoning: None,
    };

    let transport =
        FixtureHttpTransport::single(200, openai_assistant_text_body("should never land", "m"))
            .with_delay(Duration::from_millis(600))
            .with_cancel_flag(Arc::clone(&cancel_flag));

    let db_w = Arc::clone(&db);
    let rt_w = Arc::clone(&rt);
    let worker = std::thread::spawn(move || {
        let t = transport;
        execute_agent_run_concurrent(&db_w, &rt_w, owned, &t)
    });

    // Mid-request: product cancel (flag + request_cancel under short locks).
    std::thread::sleep(Duration::from_millis(50));
    let cancelled = product_request_cancel(&db, &rt, &flags, &run_id).unwrap();
    assert_eq!(
        cancelled.status,
        RunStatus::Cancelled,
        "runtime_cancel_run-equivalent must reach cancelled while turn is in flight"
    );

    let finished = worker.join().unwrap().unwrap();
    assert_eq!(finished.status, RunStatus::Cancelled);

    let events = db.lock().unwrap().list_events(&chat_id, None).unwrap();
    assert!(
        events
            .iter()
            .any(|e| e.kind == kinds::CANCEL_REQUESTED || e.kind == kinds::RUN_CANCELLED),
        "expected cancel events from product_request_cancel"
    );
    assert!(
        !events.iter().any(|e| {
            e.kind == "assistant_message" && e.payload_json.contains("should never land")
        }),
        "delayed assistant body must not be committed after cancel"
    );
}

#[test]
fn https_fetch_shared_path_with_fixture_transport() {
    let t = FixtureHttpTransport::single(200, "hello-https-body");
    let out = web_fetch_with_transport("https://example.invalid/resource", &t).unwrap();
    assert_eq!(out["status"], "ok");
    assert_eq!(out["https"], true);
    assert_eq!(out["body"], "hello-https-body");
    assert_eq!(out["statusCode"], 200);
    assert_eq!(out["fixture"], false);
}

#[test]
fn mcp_http_transport_not_fixture_echo() {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [{"type": "text", "text": "real-mcp-payload"}]
        }
    })
    .to_string();
    let t = FixtureHttpTransport::single(200, body);
    let out =
        invoke_mcp_http_with_transport("https://mcp.local/rpc", "demo.tool", &json!({"q": 1}), &t)
            .unwrap();
    assert_eq!(out["fixture"], false);
    assert!(
        out.to_string().contains("real-mcp-payload"),
        "unexpected: {out}"
    );
}

#[test]
fn mcp_stdio_real_echo_server() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mjs = manifest.join("assets/fixtures/mcp/echo_server.mjs");
    assert!(
        mjs.is_file(),
        "MCP echo fixture missing: assets/fixtures/mcp/echo_server.mjs"
    );
    assert!(
        std::process::Command::new("node")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false),
        "node is required to run the MCP stdio fixture"
    );
    let transport = McpTransport::Stdio {
        command: "node".into(),
        args: vec![mjs.display().to_string()],
        env_keys: vec![],
    };

    let out = invoke_mcp_transport(&transport, "echo", &json!({"hello": true}))
        .expect("MCP stdio invoke should succeed against local fixture server");
    assert_eq!(out["fixture"], false);
    let s = out.to_string();
    assert!(
        s.contains("echo:") || s.contains("hello"),
        "expected real echo server content, got: {s}"
    );
}

#[test]
fn mcp_dual_gate_then_real_http_path() {
    let db = Database::open_in_memory().unwrap();
    let cfg = register_mcp_server(
        &db,
        "http-real",
        McpTransport::Http {
            url: "https://mcp.example/rpc".into(),
            header_keys: vec![],
        },
        Some("proj"),
        None,
    )
    .unwrap();
    let r = mcp_call(&db, &cfg.id, "t", &json!({}), Some("proj"), true).unwrap();
    assert_eq!(r.status, "require_server_trust");
    grant_mcp_server_trust(&db, &cfg.id, Some("proj")).unwrap();

    // Without fixture env, call attempts real HTTP and returns transport_error (no server).
    std::env::remove_var("LOOPCODE_MCP_FIXTURE");
    let r = mcp_call(&db, &cfg.id, "t", &json!({}), Some("proj"), true).unwrap();
    assert!(
        r.status == "transport_error" || r.status == "ok",
        "status={}",
        r.status
    );
    if r.status == "transport_error" {
        assert!(!r.executed);
        assert_eq!(r.content["fixture"], false);
    }
}

#[test]
fn timeline_projects_assistant_message() {
    use chrono::Utc;
    use loopcode_lib::cockpit::project_timeline;
    use loopcode_lib::domain::{Event, Run};

    let events = vec![Event {
        id: "e1".into(),
        chat_id: "c1".into(),
        run_id: Some("r1".into()),
        seq: 1,
        kind: "assistant_message".into(),
        payload_json: json!({"text": "Hello from the model"}).to_string(),
        created_at: Utc::now(),
    }];
    let runs: Vec<Run> = vec![];
    let proj = project_timeline(&events, &runs, true, false, None);
    assert!(!proj.items.is_empty());
    assert!(
        proj.items
            .iter()
            .any(|i| i.body.contains("Hello from the model")),
        "timeline should show assistant text"
    );
}

#[test]
fn second_send_receives_prior_chat_history() {
    use loopcode_lib::runtime::history::history_from_events;

    let db = Database::open_in_memory().unwrap();
    let (chat_id, _root) = open_chat_with_files(&db);

    let run_a = db
        .create_run(
            &chat_id,
            "build",
            Some("gpt-fixture"),
            Some("openai"),
            Some("security-v1"),
        )
        .unwrap();
    db.append_event(
        &chat_id,
        Some(&run_a.id),
        "user_message",
        &json!({"text": "What files mention Looped?"}).to_string(),
    )
    .unwrap();
    db.append_event(
        &chat_id,
        Some(&run_a.id),
        "assistant_message",
        &json!({
            "text": "AGENTS.md and LICENSE",
            "toolCalls": [{
                "id": "call_hist",
                "name": "grep",
                "arguments": {"query": "Looped"}
            }]
        })
        .to_string(),
    )
    .unwrap();
    db.append_event(
        &chat_id,
        Some(&run_a.id),
        "tool.result",
        &json!({
            "callId": "call_hist",
            "toolName": "grep",
            "content": {"matches": [{"path": "LICENSE"}]}
        })
        .to_string(),
    )
    .unwrap();

    let run_b = db
        .create_run(
            &chat_id,
            "build",
            Some("gpt-fixture"),
            Some("openai"),
            Some("security-v1"),
        )
        .unwrap();
    let events = db.list_events(&chat_id, None).unwrap();
    let prior = history_from_events(&events, &run_b.id);
    assert!(
        prior.iter().any(|m| m.role == "user"
            && m.content.as_deref() == Some("What files mention Looped?")),
        "prior should include earlier user turn: {prior:?}"
    );
    assert!(
        prior
            .iter()
            .any(|m| m.role == "assistant" && !m.tool_calls.is_empty()),
        "prior should include assistant tool_calls"
    );
    assert!(
        prior
            .iter()
            .any(|m| m.role == "tool" && m.tool_call_id.as_deref() == Some("call_hist")),
        "prior should include tool result"
    );
}
