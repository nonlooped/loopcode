//! Slice 3 security spine tests — drive shipped Core functions only.

use loopcode_lib::db::{export_chat_redacted, redact_json_value, Database};
use loopcode_lib::domain::SCHEMA_VERSION;
use loopcode_lib::runtime::tools::ToolProposal;
use loopcode_lib::runtime::{AgentRuntime, Mode};
use loopcode_lib::security::approval::{
    approval_decision, security_gate_decision, ActionClass, ApprovalDecision, GrantScope,
    SECURITY_POLICY_VERSION,
};
use loopcode_lib::security::path::{classify_path, PathClass};
use loopcode_lib::security::secrets::{default_secret_store, SharedSecretStore};
use loopcode_lib::security::trust::{
    content_hash, grant_trust, invalidate_trust, is_trusted, TrustKind,
};
use loopcode_lib::security::audit::kinds as audit_kinds;
use loopcode_lib::runtime::mode::ToolEffect;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn temp_dir(label: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "loopcode-s3-{}-{}",
        label,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&p).unwrap();
    p
}

fn project_chat(db: &Database, root: &PathBuf) -> (String, String, String) {
    fs::create_dir_all(root).unwrap();
    let project = db.open_project(root).unwrap();
    let chat = db.create_chat(&project.id, Some("sec")).unwrap();
    (project.id, chat.id, project.resolved_path.clone())
}

#[test]
fn schema_v2_has_audit_and_trust() {
    let dir = temp_dir("schema");
    let db = Database::open(dir.join("app.sqlite")).unwrap();
    assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
    assert!(SCHEMA_VERSION >= 2);
    let _ = db
        .insert_audit("audit.test", "core", None, None, None, r#"{"ok":true}"#)
        .unwrap();
    assert_eq!(db.list_audit(10).unwrap().len(), 1);
}

#[test]
fn path_boundary_inside_outside_protected() {
    let root = temp_dir("path-root");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/a.rs"), "fn main(){}").unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join(".git/config"), "x").unwrap();
    fs::write(root.join(".env"), "SECRET=1").unwrap();

    let inside = classify_path(&root, "src/a.rs");
    assert_eq!(inside.class, PathClass::InWorkspace, "{inside:?}");

    let escape = classify_path(&root, "../secret.txt");
    assert!(
        matches!(
            escape.class,
            PathClass::OutsideWorkspace | PathClass::Rejected
        ),
        "{escape:?}"
    );

    let git = classify_path(&root, ".git/config");
    assert_eq!(git.class, PathClass::ProtectedInWorkspace, "{git:?}");

    let envf = classify_path(&root, ".env");
    assert_eq!(envf.class, PathClass::ProtectedInWorkspace, "{envf:?}");
}

#[test]
fn approval_matrix_fixed_policy() {
    assert_eq!(
        approval_decision(ActionClass::WorkspaceRead),
        ApprovalDecision::Allow
    );
    assert_eq!(
        approval_decision(ActionClass::WorkspaceWrite),
        ApprovalDecision::Allow
    );
    for class in [
        ActionClass::Shell,
        ActionClass::Network,
        ActionClass::OutsideWorkspaceFs,
        ActionClass::Secrets,
        ActionClass::Mcp,
        ActionClass::SkillScript,
        ActionClass::PackageInstall,
        ActionClass::Privilege,
        ActionClass::Destructive,
        ActionClass::Publish,
        ActionClass::ProtectedPathWrite,
    ] {
        assert_eq!(
            approval_decision(class),
            ApprovalDecision::RequireApproval,
            "{class:?}"
        );
    }
    assert_eq!(SECURITY_POLICY_VERSION, "security-v1");
}

#[test]
fn ask_plan_mutating_denied_by_security_gate_fn() {
    assert_eq!(
        security_gate_decision(Mode::Ask, ToolEffect::Mutating, ActionClass::WorkspaceWrite),
        ApprovalDecision::Deny
    );
    assert_eq!(
        security_gate_decision(Mode::Plan, ToolEffect::Mutating, ActionClass::WorkspaceWrite),
        ApprovalDecision::Deny
    );
    assert_eq!(
        security_gate_decision(Mode::Build, ToolEffect::Mutating, ActionClass::WorkspaceWrite),
        ApprovalDecision::Allow
    );
}

#[test]
fn ask_plan_write_not_executed_on_real_propose_path() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("ask-write");
    let (_pid, chat_id, resolved) = project_chat(&db, &root);
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(PathBuf::from(&resolved)));

    for mode in [Mode::Ask, Mode::Plan] {
        let run = rt.start_run(&db, &chat_id, mode, None).unwrap();
        assert_eq!(run.policy_version.as_deref(), Some(SECURITY_POLICY_VERSION));
        let res = rt
            .propose_tool(
                &db,
                &run.id,
                ToolProposal::new("write", json!({"path": "x.txt", "body": "nope"})),
            )
            .unwrap();
        assert!(
            !res.executed,
            "{mode:?} must not execute write: {:?}",
            res
        );
        rt.complete(&db, &run.id).unwrap();
    }
}

#[test]
fn build_ordinary_write_executes_without_prompt() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("build-write");
    let (_pid, chat_id, resolved) = project_chat(&db, &root);
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(PathBuf::from(&resolved)));
    let run = rt.start_run(&db, &chat_id, Mode::Build, None).unwrap();
    let res = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("write", json!({"path": "ok.txt", "body": "yes"})),
        )
        .unwrap();
    assert!(res.executed, "Build free write: {:?}", res);
}

#[test]
fn shell_requires_approval_then_grant_once() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("shell");
    let (pid, chat_id, resolved) = project_chat(&db, &root);
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(PathBuf::from(&resolved)));
    let run = rt.start_run(&db, &chat_id, Mode::Build, None).unwrap();

    let blocked = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("exec", json!({"cmd": "echo hi"})),
        )
        .unwrap();
    assert!(!blocked.executed);
    assert!(
        blocked
            .content
            .get("status")
            .and_then(|v| v.as_str())
            == Some("require_approval")
            || blocked
                .error_message
                .as_deref()
                .unwrap_or("")
                .contains("requires approval"),
        "{:?}",
        blocked
    );

    rt.grant_approval(
        &db,
        &chat_id,
        Some(&run.id),
        Some(&pid),
        ActionClass::Shell,
        GrantScope::AllowOnce,
    )
    .unwrap();

    let allowed = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("exec", json!({"cmd": "echo hi"})),
        )
        .unwrap();
    assert!(allowed.executed, "after allow once: {:?}", allowed);

    // Second shell without new grant → blocked again
    let blocked2 = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("exec", json!({"cmd": "echo again"})),
        )
        .unwrap();
    assert!(!blocked2.executed);

    let audits = db.list_audit(100).unwrap();
    assert!(audits.iter().any(|a| a.kind == audit_kinds::APPROVAL));
}

#[test]
fn network_and_secrets_require_approval() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("net");
    let (_pid, chat_id, resolved) = project_chat(&db, &root);
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(PathBuf::from(&resolved)));
    let run = rt.start_run(&db, &chat_id, Mode::Build, None).unwrap();

    for tool in ["webfetch", "secrets.reveal"] {
        let res = rt
            .propose_tool(&db, &run.id, ToolProposal::new(tool, json!({})))
            .unwrap();
        assert!(!res.executed, "{tool} must require approval: {:?}", res);
    }
}

#[test]
fn secret_store_not_in_sqlite_or_export() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("sec");
    let (pid, chat_id, _) = project_chat(&db, &root);
    let store = SharedSecretStore::memory();
    let plaintext = "sk-supersecretvalue99999";
    let store_key = "loopcode/test/openai";
    store.set(store_key, plaintext).unwrap();
    assert_eq!(store.get(store_key).unwrap().as_deref(), Some(plaintext));

    // SQLite holds reference only
    let sref = db
        .insert_secret_ref(
            "provider_api_key",
            "OpenAI",
            store_key,
            Some("openai"),
            Some(&pid),
        )
        .unwrap();
    assert_eq!(sref.store_key, store_key);
    let refs = db.list_secret_refs(Some(&pid)).unwrap();
    let blob = serde_json::to_string(&refs).unwrap();
    assert!(!blob.contains(plaintext), "DTO must not contain plaintext");

    // Event with secret-shaped content redacted on export
    db.append_event(
        &chat_id,
        None,
        "user_message",
        &json!({"text": format!("key={plaintext}"), "api_key": plaintext}).to_string(),
    )
    .unwrap();
    let export = export_chat_redacted(&db, &chat_id).unwrap();
    let serialized = serde_json::to_string(&export).unwrap();
    assert!(!serialized.contains(plaintext));
    assert!(serialized.contains("[REDACTED]") || export.redacted);

    // Audit secret access logs store_key only
    let _ = loopcode_lib::security::audit::audit_secret_access(
        &db,
        store_key,
        Some(&chat_id),
        None,
        Some(&pid),
        "get",
    )
    .unwrap();
    let audits = db.list_audit(50).unwrap();
    let sec = audits
        .iter()
        .find(|a| a.kind == audit_kinds::SECRET_REF_ACCESS)
        .unwrap();
    assert!(sec.payload_json.contains(store_key));
    assert!(!sec.payload_json.contains(plaintext));
}

#[test]
fn default_secret_store_works_without_platform_keyring() {
    let root = temp_dir("secstore");

    // A prior build's plaintext fallback must be purged, never imported.
    let legacy = root.join("secrets-fallback.json");
    fs::write(&legacy, "{}").unwrap();

    // Keyring where available, memory fallback otherwise (e.g. headless
    // Linux CI without a Secret Service) — set/get/delete must work either way.
    let store = default_secret_store(&root);
    assert!(!legacy.exists(), "legacy plaintext fallback must be removed");

    let store_key = "loopcode/test/backend-availability";
    store.set(store_key, "probe-value").unwrap();
    assert_eq!(store.get(store_key).unwrap().as_deref(), Some("probe-value"));
    store.delete(store_key).unwrap();
    assert_eq!(store.get(store_key).unwrap(), None);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn redaction_strips_json_secrets() {
    let v = json!({"token": "abc", "ok": 1});
    let r = redact_json_value(&v);
    assert_eq!(r["token"], "[REDACTED]");
    assert_eq!(r["ok"], 1);
}

#[test]
fn mcp_skill_trust_invalidates_on_hash_change() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("trust");
    let (pid, _, _) = project_chat(&db, &root);
    let h1 = content_hash(b"server-config-v1");
    let h2 = content_hash(b"server-config-v2");
    assert_ne!(h1, h2);

    grant_trust(&db, TrustKind::McpServer, "mcp://demo", &h1, Some(&pid)).unwrap();
    assert!(is_trusted(&db, TrustKind::McpServer, "mcp://demo", &h1, Some(&pid)).unwrap());
    assert!(!is_trusted(&db, TrustKind::McpServer, "mcp://demo", &h2, Some(&pid)).unwrap());

    invalidate_trust(&db, TrustKind::McpServer, "mcp://demo", Some(&pid)).unwrap();
    assert!(!is_trusted(&db, TrustKind::McpServer, "mcp://demo", &h1, Some(&pid)).unwrap());

    let h_skill = content_hash(b"skill-script-body");
    grant_trust(
        &db,
        TrustKind::SkillScript,
        "skills/demo/SKILL.md",
        &h_skill,
        Some(&pid),
    )
    .unwrap();
    assert!(is_trusted(
        &db,
        TrustKind::SkillScript,
        "skills/demo/SKILL.md",
        &h_skill,
        Some(&pid)
    )
    .unwrap());

    let audits = db.list_audit(50).unwrap();
    assert!(audits.iter().any(|a| a.kind == audit_kinds::MCP_TRUST));
    assert!(audits.iter().any(|a| a.kind == audit_kinds::SKILL_TRUST));
}

#[test]
fn protected_write_requires_approval() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("prot");
    fs::create_dir_all(root.join(".git")).unwrap();
    let (_pid, chat_id, resolved) = project_chat(&db, &root);
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(PathBuf::from(&resolved)));
    let run = rt.start_run(&db, &chat_id, Mode::Build, None).unwrap();
    let res = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("write", json!({"path": ".git/config"})),
        )
        .unwrap();
    assert!(!res.executed, "protected path write must prompt: {:?}", res);
}

#[test]
fn grant_executes_pending_shell_without_repropose() {
    let db = Database::open_in_memory().unwrap();
    let root = temp_dir("pending-exec");
    let (pid, chat_id, resolved) = project_chat(&db, &root);
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(PathBuf::from(&resolved)));
    let run = rt.start_run(&db, &chat_id, Mode::Build, None).unwrap();

    let blocked = rt
        .propose_tool(
            &db,
            &run.id,
            ToolProposal::new("exec", json!({"command": "echo pending-exec"})),
        )
        .unwrap();
    assert!(!blocked.executed);
    assert_eq!(
        blocked.content.get("status").and_then(|s| s.as_str()),
        Some("require_approval")
    );
    assert!(blocked.error_message.is_none(), "approval pause must not set error_message");
    assert_eq!(
        db.get_run(&run.id).unwrap().unwrap().status,
        loopcode_lib::domain::RunStatus::ApprovalWaiting
    );
    assert!(rt.pending_approval(&run.id).is_some());

    rt.grant_approval(
        &db,
        &chat_id,
        Some(&run.id),
        Some(&pid),
        ActionClass::Shell,
        GrantScope::AllowOnce,
    )
    .unwrap();

    let (result, remaining) = rt.execute_granted_pending(&db, &run.id).unwrap();
    assert!(remaining.is_empty());
    assert!(result.executed, "{:?}", result);
    assert!(rt.pending_approval(&run.id).is_none());
}
