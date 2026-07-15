//! Slice 8: skills script gate, MCP dual-gate.

use loopcode_lib::db::store::Database;
use loopcode_lib::extensibility::mcp::{
    grant_mcp_server_trust, is_mcp_server_trusted, mcp_call, reconfigure_mcp_server,
    register_mcp_server, McpTransport,
};
use loopcode_lib::runtime::tools::ToolProposal;
use loopcode_lib::runtime::{AgentRuntime, Mode};
use loopcode_lib::tools::catalog::{builtin_catalog, catalog_has_no_git_tools};
use loopcode_lib::tools::skills::{
    discover_skills, evaluate_skill_script_run, grant_skill_script_trust, skills_list, skills_load,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn temp(label: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "loopcode-s8-{}-{}",
        label,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&p).unwrap();
    p
}

fn install_skill(root: &PathBuf) -> (String, String) {
    let dir = root.join("skills/demo");
    fs::create_dir_all(&dir).unwrap();
    let skill_md = dir.join("SKILL.md");
    fs::write(
        &skill_md,
        "---\nname: demo\ndescription: Demo skill\n---\n# SECRET_BODY_TEXT\nDo not list this.\n",
    )
    .unwrap();
    fs::write(dir.join("run.js"), "console.log('v1');\n").unwrap();
    (
        skill_md.to_string_lossy().replace('\\', "/"),
        "run.js".into(),
    )
}

#[test]
fn skills_metadata_load_and_script_hash_trust() {
    let db = Database::open_in_memory().unwrap();
    let root = temp("skills");
    let (skill_path, script) = install_skill(&root);

    let meta = discover_skills(&root);
    assert_eq!(meta.len(), 1);
    assert_eq!(meta[0].name, "demo");
    assert!(meta[0].scripts.iter().any(|s| s == "run.js"));

    let list = skills_list(&root, &json!({})).unwrap().to_string();
    assert!(!list.contains("SECRET_BODY_TEXT"), "list must not include body");
    assert!(!list.contains("console.log"), "list must not include script body");

    let loaded = skills_load(&root, &json!({"path": skill_path})).unwrap();
    assert!(
        loaded["body"]
            .as_str()
            .unwrap_or("")
            .contains("SECRET_BODY_TEXT"),
        "explicit load returns body"
    );

    // Script requires approval without trust
    let r = evaluate_skill_script_run(&db, &root, &skill_path, &script, Some("p1"), false)
        .unwrap();
    assert_eq!(r["status"], "require_approval");
    assert_eq!(r["actionClass"], "skill_script");
    assert_eq!(r["executed"], false);
    let hash1 = r["contentHash"].as_str().unwrap().to_string();
    assert!(
        r.get("command").and_then(|c| c.as_str()).is_some_and(|c| !c.is_empty()),
        "exact command must be visible: {:?}",
        r
    );

    grant_skill_script_trust(&db, &root, &skill_path, &script, Some("p1")).unwrap();

    // Still require invocation grant (shell-class) unless granted
    let r = evaluate_skill_script_run(&db, &root, &skill_path, &script, Some("p1"), false)
        .unwrap();
    assert_eq!(r["status"], "require_approval");

    // Change script → prior trust invalid at new hash
    fs::write(root.join("skills/demo/run.js"), "console.log('v2');\n").unwrap();
    let r = evaluate_skill_script_run(&db, &root, &skill_path, &script, Some("p1"), true)
        .unwrap();
    assert_eq!(r["status"], "require_approval");
    assert_ne!(r["contentHash"].as_str().unwrap(), hash1);
    assert!(
        r["reason"]
            .as_str()
            .unwrap_or("")
            .contains("hash")
            || r["reason"].as_str().unwrap_or("").contains("trust"),
        "{:?}",
        r
    );
}

#[test]
fn mcp_stdio_and_http_dual_gate_fingerprint_reset() {
    let db = Database::open_in_memory().unwrap();

    let stdio = register_mcp_server(
        &db,
        "stdio-demo",
        McpTransport::Stdio {
            command: "npx".into(),
            args: vec!["-y".into(), "demo".into()],
            env_keys: vec!["TOKEN".into()],
        },
        Some("proj"),
        None,
    )
    .unwrap();
    let fp_a = stdio.fingerprint.clone();

    let http = register_mcp_server(
        &db,
        "http-demo",
        McpTransport::Http {
            url: "https://mcp.example/sse".into(),
            header_keys: vec!["Authorization".into()],
        },
        Some("proj"),
        None,
    )
    .unwrap();
    assert!(!http.fingerprint.is_empty());

    // Gate 1
    let r = mcp_call(&db, &stdio.id, "tools/list", &json!({}), Some("proj"), true).unwrap();
    assert_eq!(r.status, "require_server_trust");
    assert!(!r.executed);

    grant_mcp_server_trust(&db, &stdio.id, Some("proj")).unwrap();
    assert!(is_mcp_server_trusted(&db, &stdio.id, Some("proj")).unwrap());

    // Gate 2: trusted server still needs invocation approval
    let r = mcp_call(&db, &stdio.id, "tools/list", &json!({}), Some("proj"), false).unwrap();
    assert_eq!(r.status, "require_approval");
    assert!(r.server_trusted);
    assert!(!r.executed);

    // Dual-gate contract only — force fixture transport so CI does not spawn `npx`.
    let _env = crate::support::env_guard();
    std::env::set_var("LOOPCODE_MCP_FIXTURE", "1");
    let r = mcp_call(&db, &stdio.id, "tools/list", &json!({"x": 1}), Some("proj"), true).unwrap();
    std::env::remove_var("LOOPCODE_MCP_FIXTURE");
    assert_eq!(r.status, "ok");
    assert!(r.executed);

    // Fingerprint change resets trust
    let b = reconfigure_mcp_server(
        &db,
        &stdio.id,
        McpTransport::Stdio {
            command: "npx".into(),
            args: vec!["-y".into(), "demo@2".into()],
            env_keys: vec!["TOKEN".into()],
        },
    )
    .unwrap();
    assert_ne!(b.fingerprint, fp_a);
    assert!(!is_mcp_server_trusted(&db, &stdio.id, Some("proj")).unwrap());
    let r = mcp_call(&db, &stdio.id, "tools/list", &json!({}), Some("proj"), true).unwrap();
    assert_eq!(r.status, "require_server_trust");
}

#[test]
fn regression_shell_gated_no_git_skill_body_not_in_list() {
    // shell still requires approval via runtime
    let db = Database::open_in_memory().unwrap();
    let root = temp("reg");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("r")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root.clone()));
    let run = rt.start_run(&db, &chat.id, Mode::Build, None).unwrap();
    let blocked = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("exec", json!({"command": "echo s8"})),
        )
        .unwrap();
    assert!(!blocked.executed);
    assert_eq!(
        blocked.content.get("status").and_then(|s| s.as_str()),
        Some("require_approval")
    );

    let cat = builtin_catalog();
    assert!(catalog_has_no_git_tools(&cat));
    assert!(!cat.iter().any(|d| d.name.starts_with("git.")));
    assert!(cat.iter().any(|d| d.name == "skill"));
    assert!(!cat.iter().any(|d| d.name == "mcp.call"));
    assert!(!cat.iter().any(|d| d.name == "skills.list"));
    assert!(!cat.iter().any(|d| d.name == "skills.run_script"));

    let (skill_path, _) = install_skill(&root);
    let list = skills_list(&root, &json!({})).unwrap().to_string();
    assert!(!list.contains("SECRET_BODY_TEXT"));

    let loaded = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("skill", json!({"path": skill_path})),
        )
        .unwrap();
    assert!(loaded.executed, "{:?}", loaded);
    assert!(
        loaded.content["body"]
            .as_str()
            .unwrap_or("")
            .contains("SECRET_BODY_TEXT")
    );
}
