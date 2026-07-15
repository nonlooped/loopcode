//! Integration-style smoke checks for the Slice 0 scaffold.
//! These exercise real shipped library APIs and on-disk config, not re-implemented stubs.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/src-tauri
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri parent")
        .to_path_buf()
}

#[test]
fn health_entry_is_capability_locked_shell() {
    let status = loopcode_lib::health_status();
    assert_eq!(status.name, loopcode_lib::APP_NAME);
    assert_eq!(status.shell, "capability-locked");
    assert_eq!(status.version, loopcode_lib::APP_VERSION);
    assert!(loopcode_lib::SCHEMA_VERSION >= 2);
}

#[test]
fn capability_file_exists_and_excludes_dangerous_permissions() {
    let path = repo_root().join("src-tauri/capabilities/default.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

    // Parse JSON so description prose cannot false-positive permission checks.
    let value: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let perms = value["permissions"]
        .as_array()
        .expect("permissions array")
        .iter()
        .filter_map(|p| p.as_str())
        .collect::<Vec<_>>();

    assert!(
        perms.iter().any(|p| *p == "core:default" || p.starts_with("core:")),
        "expected core permissions, got {perms:?}"
    );

    let denied_prefixes = ["fs:", "shell:", "opener:", "http:", "process:"];
    for p in &perms {
        for d in denied_prefixes {
            assert!(
                !p.contains(d),
                "capability must not grant '{p}' (matched '{d}') for empty shell"
            );
        }
    }

    assert!(
        perms.iter().any(|p| *p == "dialog:allow-open"),
        "expected dialog:allow-open for native folder picker, got {perms:?}"
    );
}

#[test]
fn tauri_conf_product_is_loopcode() {
    let path = repo_root().join("src-tauri/tauri.conf.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(raw.contains("\"productName\": \"LoopCode\""));
    assert!(raw.contains("\"title\": \"LoopCode\""));
    assert!(raw.contains("\"identifier\": \"com.loopcode.app\""));
}
