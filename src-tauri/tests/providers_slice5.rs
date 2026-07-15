//! Slice 5: adapters, catalog snapshot, probe fixtures, secret separation.

use loopcode_lib::db::store::Database;
use loopcode_lib::db::export_chat_redacted;
use loopcode_lib::providers::adapter::Protocol;
use loopcode_lib::providers::catalog::{
    catalog_npm_is_non_executable, hero_providers, load_bundled_catalog, select_default_model,
    BUNDLED_CATALOG_JSON,
};
use loopcode_lib::providers::connection::{
    connect_first_party, create_custom_profile, get_ready_provider, list_catalog_cards,
    list_hero_cards, set_ready_provider_selection,
};
use loopcode_lib::providers::custom::{CustomProfile, CustomProfileAdapter};
use loopcode_lib::providers::first_party::{first_party_adapter, hero_provider_ids};
use loopcode_lib::providers::probe::{
    probe_first_party, run_probe, FixtureTransport,
};
use loopcode_lib::providers::ProviderAdapter;
use loopcode_lib::security::secrets::SharedSecretStore;
use serde_json::json;
use std::fs;

#[test]
fn four_first_party_adapters_exist() {
    for id in hero_provider_ids() {
        let a = first_party_adapter(id).expect(id);
        assert_eq!(a.id(), *id);
        assert!(!a.display_name().is_empty());
    }
}

#[test]
fn adapter_contract_fixtures_all_heroes() {
    let cases = [
        ("openai", "gpt-4o-mini"),
        ("anthropic", "claude-sonnet-4-20250514"),
        ("openrouter", "openai/gpt-4o-mini"),
        ("opencode", "opencode/zen-default"),
    ];
    for (id, model) in cases {
        let adapter = first_party_adapter(id).unwrap();
        let transport = FixtureTransport::for_provider(id, true);
        let req = adapter.build_probe_request(model, "sk-test-secret-key-not-in-dto");
        assert!(req.url.starts_with("https://") || req.url.contains("api"));
        assert!(!req.url.contains("sk-test"));
        // Headers contain key but parse path uses fixture body only
        let result = run_probe(
            adapter.as_ref(),
            model,
            "sk-test-secret-key-not-in-dto",
            &transport,
            true,
        );
        assert!(result.ready, "{id}: {:?}", result.error);
        let text = result.text.as_ref().unwrap();
        assert_eq!(text.text, "pong");
        assert_eq!(text.provider_id, id);
        assert!(text.provenance.get("catalogExecutable") == Some(&json!(false)));
        assert!(result.used_fixture);
    }
}

#[test]
fn adapter_failure_fixture_not_ready() {
    let adapter = first_party_adapter("openai").unwrap();
    let transport = FixtureTransport::for_provider("openai", false);
    let result = run_probe(adapter.as_ref(), "gpt-4o-mini", "bad-key", &transport, true);
    assert!(!result.ready);
    assert!(result.error.is_some());
    assert_eq!(
        result.error.as_ref().unwrap().category,
        "authentication"
    );
}

#[test]
fn custom_profile_openai_chat_completions_distinct() {
    let profile = CustomProfile {
        id: "custom-local".into(),
        display_name: "Local".into(),
        protocol: Protocol::OpenaiChatCompletions,
        base_url: "http://127.0.0.1:8080/v1".into(),
        default_model: Some("local-model".into()),
    };
    let adapter = CustomProfileAdapter::new(profile);
    let req = adapter.build_probe_request("local-model", "k");
    assert!(
        req.url.ends_with("/chat/completions"),
        "chat completions URL: {}",
        req.url
    );
    assert!(req.body.get("messages").is_some());
    assert!(req.body.get("input").is_none());
    let body = include_str!("../assets/fixtures/providers/openai_probe_ok.json");
    let v: serde_json::Value = serde_json::from_str(body).unwrap();
    let status = v["status"].as_u64().unwrap() as u16;
    let body_s = v["body"].to_string();
    let text = adapter
        .parse_probe_response(status, &body_s, "local-model")
        .unwrap();
    assert_eq!(text.text, "pong");
    assert_eq!(text.provider_id, "custom-local");
    assert_eq!(text.protocol, Protocol::OpenaiChatCompletions.as_str());
    assert_eq!(text.provenance["endpointClass"], "chat.completions");
}

#[test]
fn custom_profile_openai_responses_distinct() {
    let profile = CustomProfile {
        id: "custom-responses".into(),
        display_name: "Responses Endpoint".into(),
        protocol: Protocol::OpenaiResponses,
        base_url: "https://api.openai.com/v1".into(),
        default_model: Some("gpt-4o-mini".into()),
    };
    let adapter = CustomProfileAdapter::new(profile);
    let req = adapter.build_probe_request("gpt-4o-mini", "sk-test");
    assert!(
        req.url.ends_with("/responses"),
        "Responses API must use /responses, got {}",
        req.url
    );
    assert!(
        !req.url.contains("/chat/completions"),
        "must not use chat completions URL"
    );
    assert!(req.body.get("input").is_some(), "Responses body uses input");
    assert!(
        req.body.get("messages").is_none(),
        "Responses body must not use chat messages"
    );

    let raw = include_str!("../assets/fixtures/providers/openai_responses_probe_ok.json");
    let v: serde_json::Value = serde_json::from_str(raw).unwrap();
    let status = v["status"].as_u64().unwrap() as u16;
    let body_s = v["body"].to_string();
    let text = adapter
        .parse_probe_response(status, &body_s, "gpt-4o-mini")
        .unwrap();
    assert_eq!(text.text, "pong");
    assert_eq!(text.protocol, Protocol::OpenaiResponses.as_str());
    assert_eq!(text.provenance["endpointClass"], "responses");
    assert_eq!(text.provider_id, "custom-responses");
}

#[test]
fn next_model_on_failure_sets_notice_via_scripted_transport() {
    use loopcode_lib::providers::probe::{
        probe_with_model_fallback, FixtureTransport, SequenceTransport,
    };

    let adapter = first_party_adapter("openai").unwrap();
    let catalog = load_bundled_catalog().unwrap();
    let provider = catalog.providers.iter().find(|p| p.id == "openai").unwrap();
    let default = select_default_model(provider).unwrap();
    let next = loopcode_lib::providers::catalog::next_model_after(provider, &default)
        .expect("openai catalog needs a second model");

    let fail = FixtureTransport::load_fixture("openai_probe_fail.json").unwrap();
    let ok = FixtureTransport::load_fixture("openai_probe_ok.json").unwrap();
    let transport = SequenceTransport::fail_then_success(&fail.1, ok.0, &ok.1);

    let result = probe_with_model_fallback(
        adapter.as_ref(),
        provider,
        "sk-test",
        &transport,
        true,
    );
    assert!(result.ready, "second model should succeed: {:?}", result.error);
    // Fixture body may report its own model id; the notice records the selected next id.
    let notice = result.notice.expect("user-visible notice required");
    assert!(
        notice.contains("failed probe") && notice.contains(&next),
        "notice should name next model {next}: {notice}"
    );
    assert!(
        notice.contains(&default) || notice.contains("default model"),
        "notice should mention failed default {default}: {notice}"
    );
    assert!(
        notice.contains(&format!("selected next model: {next}")),
        "exact fallback wording: {notice}"
    );
}

#[test]
fn bundled_catalog_heroes_and_no_execute() {
    let cat = load_bundled_catalog().unwrap();
    assert!(BUNDLED_CATALOG_JSON.contains("opencode"));
    assert!(BUNDLED_CATALOG_JSON.contains("DO_NOT_EXECUTE"));
    let heroes = hero_providers(&cat);
    assert_eq!(heroes.len(), 4);
    for id in ["openai", "anthropic", "openrouter", "opencode"] {
        assert!(heroes.iter().any(|p| p.id == id), "missing {id}");
    }
    let oc = heroes.iter().find(|p| p.id == "opencode").unwrap();
    assert!(oc.name.to_lowercase().contains("zen") || oc.name.contains("OpenCode"));
    for p in &cat.providers {
        assert!(
            catalog_npm_is_non_executable(p),
            "npm must not be executable: {:?}",
            p.npm
        );
    }
    // Must not contain package execution directives
    assert!(!BUNDLED_CATALOG_JSON.contains("\"executePackage\""));
}

#[test]
fn default_model_selection_order() {
    let cat = load_bundled_catalog().unwrap();
    let openai = cat.providers.iter().find(|p| p.id == "openai").unwrap();
    let selected = select_default_model(openai).expect("openai must have a default model");
    // Default must exist in the catalog list and match the provider's declared default when set.
    assert!(
        openai.models.iter().any(|m| m.id == selected),
        "selected default {selected} missing from openai models"
    );
    if let Some(declared) = openai.default_model.as_deref() {
        assert_eq!(selected, declared);
    }
    // Sanity: models.dev snapshot should expose modern GPT-5 family models.
    assert!(
        openai.models.iter().any(|m| m.id.starts_with("gpt-5")),
        "openai catalog should include gpt-5* models from models.dev"
    );
}

#[test]
fn connect_stores_secret_ref_not_plaintext_and_probe() {
    let db = Database::open_in_memory().unwrap();
    let secrets = SharedSecretStore::memory();
    let key = "sk-live-looking-secret-value-XYZ";
    let ok = connect_first_party(&db, &secrets, "openai", key, Some(true)).unwrap();
    assert!(ok.probe.ready);
    assert!(ok.connection.ready);
    // Probe fixture may report its own model id; otherwise Core falls back to catalog default.
    assert!(
        ok.connection.default_model.is_some(),
        "connected provider must expose a default model"
    );
    assert!(!ok.connection.store_key.is_empty());

    let dto = serde_json::to_string(&ok.connection).unwrap();
    assert!(!dto.contains(key));
    let probe_s = serde_json::to_string(&ok.probe).unwrap();
    assert!(!probe_s.contains(key));

    let refs = db.list_secret_refs(None).unwrap();
    assert!(!refs.is_empty());
    let refs_s = serde_json::to_string(&refs).unwrap();
    assert!(!refs_s.contains(key));
    assert!(refs_s.contains("loopcode/provider/openai"));

    // Stored value only in secret store
    assert_eq!(
        secrets.get(&ok.connection.store_key).unwrap().as_deref(),
        Some(key)
    );

    // Export path still redacts secret-shaped content
    let project = db
        .open_project(std::env::temp_dir().join("s5-export-ws"))
        .unwrap();
    let chat = db.create_chat(&project.id, Some("c")).unwrap();
    db.append_event(
        &chat.id,
        None,
        "note",
        &json!({"api_key": key}).to_string(),
    )
    .unwrap();
    let exp = export_chat_redacted(&db, &chat.id).unwrap();
    let exp_s = serde_json::to_string(&exp).unwrap();
    assert!(!exp_s.contains(key));
}

#[test]
fn connect_failure_fixture_not_ready() {
    let db = Database::open_in_memory().unwrap();
    let secrets = SharedSecretStore::memory();
    let res = connect_first_party(&db, &secrets, "openai", "sk-bad", Some(false)).unwrap();
    assert!(!res.probe.ready);
    assert!(!res.connection.ready);
    assert!(res.connection.last_probe_error.is_some());
}

#[test]
fn connect_reuses_saved_key_and_file_store_persists() {
    let db = Database::open_in_memory().unwrap();
    let secrets = SharedSecretStore::memory();
    let key = "sk-saved-and-reused-key-ABC";

    // First connect stores the key.
    connect_first_party(&db, &secrets, "openai", key, Some(true)).unwrap();
    let heroes = list_hero_cards(Some(&secrets), Some(&db)).unwrap();
    let openai = heroes.iter().find(|h| h.id == "openai").unwrap();
    assert!(openai.has_saved_key, "hero card should report saved key");

    // Empty api_key reuses the stored secret (switch-back without re-paste).
    let reused = connect_first_party(&db, &secrets, "openai", "", Some(true)).unwrap();
    assert!(reused.probe.ready);
    assert_eq!(
        secrets.get(&reused.connection.store_key).unwrap().as_deref(),
        Some(key)
    );

    // Switching to another provider without a key still requires one.
    let missing = connect_first_party(&db, &secrets, "anthropic", "", Some(true));
    assert!(missing.is_err());

    // File fallback survives process restart (re-open same path).
    let dir = std::env::temp_dir().join(format!(
        "lc-secrets-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    let store_a = loopcode_lib::security::secrets::default_secret_store(&dir);
    store_a
        .set("loopcode/provider/openai/api_key", key)
        .unwrap();
    drop(store_a);
    let store_b = loopcode_lib::security::secrets::default_secret_store(&dir);
    assert_eq!(
        store_b
            .get("loopcode/provider/openai/api_key")
            .unwrap()
            .as_deref(),
        Some(key)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn probe_first_party_helper() {
    let r = probe_first_party("anthropic", "sk-x", Some(true)).unwrap();
    assert!(r.ready);
    assert_eq!(r.text.as_ref().unwrap().text, "pong");
}

#[test]
fn provider_selection_persists_across_ready_reads() {
    let db = Database::open_in_memory().unwrap();
    let secrets = SharedSecretStore::memory();
    connect_first_party(&db, &secrets, "opencode", "sk-zen-test-key", Some(true)).unwrap();

    let updated = set_ready_provider_selection(
        &db,
        &secrets,
        "opencode",
        "deepseek-v4-flash-free",
    )
    .unwrap();
    assert_eq!(updated["providerId"], "opencode");
    assert_eq!(updated["model"], "deepseek-v4-flash-free");

    let ready = get_ready_provider(&db).unwrap().expect("ready set");
    assert_eq!(ready["providerId"], "opencode");
    assert_eq!(ready["model"], "deepseek-v4-flash-free");

    // Per-provider default also recorded for future switch-back.
    let per = db
        .get_setting("user", "provider.opencode.default_model")
        .unwrap()
        .unwrap();
    assert_eq!(per.value_json, "\"deepseek-v4-flash-free\"");

    // Cannot select a provider with no stored key.
    let missing = set_ready_provider_selection(&db, &secrets, "openai", "gpt-4o");
    assert!(missing.is_err());
}

#[test]
fn custom_profile_settings_and_heroes_api() {
    let db = Database::open_in_memory().unwrap();
    let heroes = list_hero_cards(None, None).unwrap();
    assert_eq!(heroes.len(), 4);
    let catalog = list_catalog_cards().unwrap();
    assert!(catalog.len() >= 4);

    let saved = create_custom_profile(
        &db,
        "My LLM",
        "openai_chat_completions",
        "https://example.com/v1",
        Some("m1"),
    )
    .unwrap();
    assert!(saved.value.get("id").is_some());
    assert_eq!(saved.value["catalogExecutable"], false);
}
