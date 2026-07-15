//! Slice 9: retry/timeout, suspend/limits, backup, updates.

use loopcode_lib::db::store::Database;
use loopcode_lib::reliability::backup::{backup_database, restore_from_backup};
use loopcode_lib::reliability::limits::{
    default_limits, evaluate_run_limits, LimitKind, ResourceLimits, RunUsageSnapshot,
};
use loopcode_lib::reliability::logs::append_local_log;
use loopcode_lib::reliability::retry::{
    plan_provider_retry, should_auto_retry, RetryDecision, ToolTimeoutClass, MAX_AUTO_RETRIES,
};
use loopcode_lib::reliability::updates::{
    reset_update_http_calls, run_update_check, update_http_call_count,
    verify_update_signature, UpdateCheckConfig, UpdateCheckTransport,
};
use loopcode_lib::runtime::error::{
    ErrorCategory, ErrorOrigin, ErrorStage, RuntimeError, SideEffectCertainty,
};
use loopcode_lib::runtime::{AgentRuntime, Mode};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn temp(label: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "loopcode-s9-{}-{}",
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
fn retry_timeout_bounds_retryable_vs_not() {
    assert_eq!(MAX_AUTO_RETRIES, 2);
    assert!(matches!(
        should_auto_retry(ErrorCategory::RateLimit, 0),
        RetryDecision::AutoRetry { attempt: 1, .. }
    ));
    assert!(matches!(
        should_auto_retry(ErrorCategory::Network, 1),
        RetryDecision::AutoRetry { attempt: 2, .. }
    ));
    assert_eq!(
        should_auto_retry(ErrorCategory::Timeout, 2),
        RetryDecision::Exhausted
    );

    let auth = RuntimeError::new(
        ErrorCategory::SchemaValidation,
        ErrorOrigin::Provider,
        ErrorStage::Model,
        SideEffectCertainty::None,
        "invalid_api_key",
    );
    let plan = plan_provider_retry(&auth, 0, None);
    assert_eq!(plan.decision, RetryDecision::FailImmediately);
    assert!(!plan.next_action.is_empty());
    assert_eq!(plan.category, "schema_validation");

    let rl = RuntimeError::new(
        ErrorCategory::RateLimit,
        ErrorOrigin::Provider,
        ErrorStage::Model,
        SideEffectCertainty::None,
        "429 Too Many Requests",
    );
    let plan = plan_provider_retry(&rl, 0, Some(5));
    match plan.decision {
        RetryDecision::AutoRetry { backoff_ms, attempt } => {
            assert_eq!(attempt, 1);
            assert_eq!(backoff_ms, 5000);
        }
        other => panic!("expected auto retry, got {other:?}"),
    }

    assert_eq!(
        ToolTimeoutClass::for_tool("exec").timeout_secs(),
        120
    );
    assert_eq!(
        ToolTimeoutClass::for_tool("read").timeout_secs(),
        15
    );
}

#[test]
fn suspend_reconcile_and_cost_limits() {
    let db = Database::open_in_memory().unwrap();
    let root = temp("suspend");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("c")).unwrap();
    let mut rt = AgentRuntime::new();
    let run = rt
        .start_run(&db, &chat.id, Mode::Build, None)
        .unwrap();
    // In-flight (model_active)
    assert!(!run.status.is_terminal());
    assert_ne!(run.status.as_str(), "completed");

    let n = AgentRuntime::reconcile_after_restart(&db).unwrap();
    assert!(n >= 1);
    let after = db.get_run(&run.id).unwrap().unwrap();
    assert_eq!(after.status.as_str(), "suspended");
    assert_ne!(after.status.as_str(), "completed");

    let lim = ResourceLimits {
        max_wall_ms: 1000,
        max_cost_micros: 500,
        warn_ratio: 0.8,
    };
    let stop = evaluate_run_limits(
        &lim,
        &RunUsageSnapshot {
            elapsed_ms: 1000,
            cost_micros: 0,
        },
    );
    assert!(stop.should_stop);
    assert!(stop.security_policy_intact);
    assert_eq!(stop.kind, LimitKind::WallTimeStop);

    let cost = evaluate_run_limits(
        &lim,
        &RunUsageSnapshot {
            elapsed_ms: 0,
            cost_micros: 500,
        },
    );
    assert!(cost.should_stop);
    assert!(cost.security_policy_intact);

    // Defaults exist and don't weaken security
    let d = evaluate_run_limits(
        &default_limits(),
        &RunUsageSnapshot {
            elapsed_ms: 1,
            cost_micros: 1,
        },
    );
    assert!(d.security_policy_intact);
}

#[test]
fn sqlite_integrity_and_vacuum_backup() {
    let dir = temp("bak");
    let live = dir.join("app.sqlite");
    let db = Database::open(&live).unwrap();
    db.integrity_check().unwrap();
    let project = db.open_project(&dir).unwrap();
    let chat = db.create_chat(&project.id, Some("t")).unwrap();
    db.append_event(&chat.id, None, "user_message", r#"{"text":"backup-me"}"#)
        .unwrap();
    drop(db);

    let bak = dir.join("backups").join("manual.sqlite");
    backup_database(&live, &bak).unwrap();
    assert!(bak.is_file());
    assert!(!bak.to_string_lossy().contains("-wal"));

    let restored = dir.join("restored.sqlite");
    restore_from_backup(&bak, &restored).unwrap();
    let db2 = Database::open(&restored).unwrap();
    let events = db2.list_events(&chat.id, None).unwrap();
    assert_eq!(events.len(), 1);
    assert!(events[0].payload_json.contains("backup-me"));

    // Fail-closed: garbage is not silently wiped
    let bad = dir.join("corrupt.sqlite");
    fs::write(&bad, b"GARBAGE_NOT_SQLITE").unwrap();
    match Database::open(&bad) {
        Ok(_) => panic!("corrupt DB must not open cleanly"),
        Err(err) => assert!(!err.is_empty(), "expected open error"),
    }
    assert_eq!(fs::read(&bad).unwrap(), b"GARBAGE_NOT_SQLITE");
}

#[test]
fn local_log_redacts_secret_shaped_fields() {
    let logs = temp("logs");
    let append = append_local_log(
        &logs,
        "warn",
        "rate_limited",
        "provider 429",
        Some(&json!({"apiKey": "sk-should-redact", "note": "x"})),
    )
    .unwrap();
    let body = fs::read_to_string(&append.path).unwrap();
    assert!(body.contains("provider 429"));
    assert!(!body.contains("sk-should-redact"));
}

#[test]
fn update_check_disabled_and_signature_gate() {
    reset_update_http_calls();
    struct Boom;
    impl UpdateCheckTransport for Boom {
        fn fetch_manifest(&mut self) -> Result<String, String> {
            panic!("HTTP must not run when check disabled");
        }
    }
    let cfg = UpdateCheckConfig {
        check_enabled: false,
    };
    let err = run_update_check(&cfg, &mut Boom).unwrap_err();
    assert!(err.contains("disabled"));
    assert_eq!(update_http_call_count(), 0);

    let pk = "loopcode-v1-pubkey";
    let body = b"artifact-bytes";
    assert!(!verify_update_signature(pk, body, "forged").accepted);
    let bad = verify_update_signature(pk, body, "bad-sig:0");
    assert!(!bad.accepted);
    assert!(bad.reason.contains("unavailable"));
}

#[test]
fn runtime_enforces_retry_limits_and_timeout_fail() {
    use loopcode_lib::runtime::tools::ToolProposal;
    use std::time::Duration;

    let db = Database::open_in_memory().unwrap();
    let root = temp("rt-wire");
    let project = db.open_project(&root).unwrap();
    let chat = db.create_chat(&project.id, Some("r")).unwrap();
    let mut rt = AgentRuntime::new();
    rt.set_project_root_override(Some(root.clone()));

    // --- Provider retry path uses plan_provider_retry ---
    let run = rt
        .start_run(&db, &chat.id, Mode::Build, None)
        .unwrap();
    let err = RuntimeError::new(
        ErrorCategory::RateLimit,
        ErrorOrigin::Provider,
        ErrorStage::Model,
        SideEffectCertainty::None,
        "429 from provider",
    );
    let (plan1, run1) = rt
        .record_provider_failure(&db, &run.id, err.clone(), Some(1))
        .unwrap();
    assert!(matches!(
        plan1.decision,
        RetryDecision::AutoRetry { attempt: 1, .. }
    ));
    assert_ne!(run1.status.as_str(), "completed");
    assert_ne!(run1.status.as_str(), "failed"); // still retrying

    let events = db.list_events(&chat.id, None).unwrap();
    assert!(
        events.iter().any(|e| e.kind == "run.retry_planned"),
        "retry plan must be emitted on live path: {:?}",
        events.iter().map(|e| &e.kind).collect::<Vec<_>>()
    );

    // Exhaust retries → fail (not completed)
    let _ = rt
        .record_provider_failure(&db, &run.id, err.clone(), None)
        .unwrap();
    let (plan3, run3) = rt
        .record_provider_failure(&db, &run.id, err, None)
        .unwrap();
    assert!(matches!(
        plan3.decision,
        RetryDecision::Exhausted | RetryDecision::FailImmediately
    ) || run3.status.as_str() == "failed");
    // After enough failures status is failed
    let final_run = db.get_run(&run.id).unwrap().unwrap();
    // May already be failed from attempt 2 or 3
    if final_run.status.as_str() != "failed" {
        let (_, r) = rt
            .record_provider_failure(
                &db,
                &run.id,
                RuntimeError::new(
                    ErrorCategory::RateLimit,
                    ErrorOrigin::Provider,
                    ErrorStage::Model,
                    SideEffectCertainty::None,
                    "429 again",
                ),
                None,
            )
            .unwrap();
        assert_eq!(r.status.as_str(), "failed");
    }
    assert!(
        rt.complete(&db, &run.id).is_err(),
        "cannot complete a failed run"
    );

    // --- Cost/time limits enforced on propose_tool ---
    let run2 = rt
        .start_run(&db, &chat.id, Mode::Build, None)
        .unwrap();
    rt.set_resource_limits(ResourceLimits {
        max_wall_ms: 1,
        max_cost_micros: 1_000_000,
        warn_ratio: 0.8,
    });
    // Force elapsed past limit
    std::thread::sleep(Duration::from_millis(5));
    let blocked = rt.propose_tool(
        &db,
        &run2.id,
        ToolProposal::new("glob", json!({"pattern": "**/*"})),
    );
    assert!(blocked.is_err(), "over-limit propose must fail: {blocked:?}");
    let stopped = db.get_run(&run2.id).unwrap().unwrap();
    assert_eq!(
        stopped.status.as_str(),
        "failed",
        "limit stop must fail run, not complete"
    );
    assert!(rt.complete(&db, &run2.id).is_err());

    // --- Tool timeout fails run (shell with tiny timeout) ---
    rt.set_resource_limits(default_limits());
    let run3 = rt
        .start_run(&db, &chat.id, Mode::Build, None)
        .unwrap();
    // Grant shell so execution path runs
    use loopcode_lib::security::approval::GrantScope;
    use loopcode_lib::security::ActionClass;
    rt.grant_approval(
        &db,
        &chat.id,
        Some(&run3.id),
        Some(&project.id),
        ActionClass::Shell,
        GrantScope::AllowOnce,
    )
    .unwrap();
    // Windows: ping -n 5 waits ~4s; timeout 1s → timedOut
    let timeout_cmd = if cfg!(windows) {
        "ping -n 5 127.0.0.1"
    } else {
        "sleep 5"
    };
    let tr = rt
        .propose_tool(
            &db,
            &run3.id,
            ToolProposal::new(
                "exec",
                json!({"command": timeout_cmd, "timeoutSecs": 1}),
            ),
        )
        .unwrap();
    assert!(!tr.executed || tr.content.get("timedOut") == Some(&json!(true)) || tr.content.get("status") == Some(&json!("timeout")),
        "{:?}", tr);
    let after_timeout = db.get_run(&run3.id).unwrap().unwrap();
    assert_eq!(
        after_timeout.status.as_str(),
        "failed",
        "timeout must fail run (not completed): {:?}",
        after_timeout.status
    );
    assert!(rt.complete(&db, &run3.id).is_err());

    // Error events carry retryPlan for UI projection
    let evs = db.list_events(&chat.id, None).unwrap();
    let err_ev = evs.iter().find(|e| e.kind.contains("error"));
    assert!(err_ev.is_some(), "expected error event");
    if let Some(e) = err_ev {
        assert!(
            e.payload_json.contains("retryPlan") || e.payload_json.contains("nextAction") || e.payload_json.contains("category"),
            "error payload must project retry plan fields: {}",
            e.payload_json
        );
    }
}
