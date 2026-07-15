//! Slice 10: focus order, keyboard, themes, single-instance, notifications, docs.

use loopcode_lib::a11y::focus::{
    escape_closes_overlay, focus_order_regions, next_focus_region, FocusRegion,
};
use loopcode_lib::a11y::notifications::{
    build_approval_notification, notification_payload_is_safe,
};
use loopcode_lib::a11y::single_instance::{
    acquire_single_instance, decide_instance, SingleInstanceDecision,
};
use loopcode_lib::a11y::theme::{
    apply_theme_preference, default_theme, parse_theme, ThemePreference,
};
use loopcode_lib::cockpit::keyboard::{
    default_keyboard_map, required_expert_commands, CockpitCommand, KeyboardRegistry,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn focus_order_and_keyboard_registry() {
    let order = focus_order_regions();
    assert_eq!(
        order,
        vec![
            "left_rail",
            "top_chrome",
            "timeline",
            "composer",
            "right_pane"
        ]
    );
    assert_eq!(
        next_focus_region(FocusRegion::Composer),
        FocusRegion::RightPane
    );
    assert!(escape_closes_overlay(true));

    let reg = KeyboardRegistry::with_defaults();
    for cmd in required_expert_commands() {
        assert!(
            reg.registered_commands().contains(cmd),
            "missing expert {cmd:?}"
        );
    }
    // Approval Once/Deny keyboard-addressable
    assert_eq!(
        reg.resolve(false, true, false, "a"),
        Some(CockpitCommand::ApproveOnce)
    );
    assert_eq!(
        reg.resolve(false, true, false, "d"),
        Some(CockpitCommand::DenyApproval)
    );
    assert!(default_keyboard_map().len() >= 7);
}

#[test]
fn theme_preference_helpers() {
    assert_eq!(default_theme(), ThemePreference::Default);
    assert_eq!(
        parse_theme("high_contrast"),
        Some(ThemePreference::HighContrast)
    );
    let applied = apply_theme_preference(ThemePreference::HighContrast);
    assert_eq!(applied.css_class, "theme-high-contrast");
    assert_eq!(applied.data_theme, "high_contrast");
}

#[test]
fn single_instance_decision_and_lock() {
    assert_eq!(
        decide_instance(true),
        SingleInstanceDecision::SecondaryAlreadyRunning
    );
    assert_eq!(decide_instance(false), SingleInstanceDecision::Primary);

    let dir = std::env::temp_dir().join(format!(
        "lc-s10-si-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    let lock = dir.join("inst.lock");

    // Concurrent second acquire while first exclusive guard is live.
    let (d1, g1) = acquire_single_instance(&lock).unwrap();
    assert_eq!(d1, SingleInstanceDecision::Primary);
    let g1 = g1.expect("primary returns exclusive guard held for process lifetime");

    let (d2, g2) = acquire_single_instance(&lock).unwrap();
    assert_eq!(
        d2,
        SingleInstanceDecision::SecondaryAlreadyRunning,
        "while primary guard is held, second acquire must be Secondary (exclusive OS lock)"
    );
    assert!(g2.is_none(), "secondary must not own a guard");

    drop(g1);
    let (d3, g3) = acquire_single_instance(&lock).unwrap();
    assert_eq!(d3, SingleInstanceDecision::Primary);
    assert!(g3.is_some());
}

#[test]
fn approval_notification_excludes_secrets() {
    let detail = json!({
        "apiKey": "sk-super-secret-key-value",
        "command": "export TOKEN=secret && curl -H 'Authorization: Bearer tok123' http://x",
        "summary": "Approve exec",
        "arguments": {"password": "hunter2"}
    });
    let n = build_approval_notification(
        "shell",
        Some("exec"),
        Some("c1"),
        Some("r1"),
        Some(&detail),
    );
    assert!(!n.includes_secrets);
    assert!(notification_payload_is_safe(&n));
    assert!(!n.body.contains("sk-super"));
    assert!(!n.body.contains("tok123"));
    assert!(!n.body.contains("hunter2"));
    assert!(!n.body.contains("Bearer tok"));
    assert!(n.title.to_lowercase().contains("approval") || n.body.contains("shell"));
}

#[test]
fn install_docs_cover_unsigned_friction() {
    let root = repo_root();
    let install = fs::read_to_string(root.join("docs/install.md")).unwrap();
    for needle in [
        "SmartScreen",
        "Run anyway",
        "Gatekeeper",
        "AppImage",
        "chmod",
    ] {
        assert!(install.contains(needle), "install.md missing {needle}");
    }
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.contains("docs/install.md") || readme.contains("SmartScreen"));
}

#[test]
fn a11y_smoke_gates() {
    let reg = KeyboardRegistry::with_defaults();
    assert!(reg.resolve(true, false, false, "n").is_some());
    assert_eq!(focus_order_regions().len(), 5);
    assert!(parse_theme("default").is_some());
    assert!(parse_theme("high_contrast").is_some());

    let html = fs::read_to_string(repo_root().join("index.html")).unwrap();
    assert!(html.contains("id=\"root\""));
    let css = fs::read_to_string(repo_root().join("src/styles/theme.css")).unwrap();
    assert!(css.contains(":focus-visible"));
}
