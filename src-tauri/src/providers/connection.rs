//! Connect provider: store secret via Core store + secret_ref; probe; set default model.

use crate::db::store::Database;
use crate::providers::catalog::{load_bundled_catalog, select_default_model, CatalogProvider};
use crate::providers::first_party::{first_party_adapter, hero_provider_ids};
use crate::providers::probe::{probe_first_party, ProbeResult};
use crate::security::audit;
use crate::security::secrets::SharedSecretStore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConnectionDto {
    pub id: String,
    pub provider_id: String,
    pub display_name: String,
    pub ready: bool,
    pub default_model: Option<String>,
    pub secret_ref_id: Option<String>,
    pub store_key: String,
    /// Never includes API key plaintext.
    pub last_probe_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroCard {
    pub id: String,
    pub name: String,
    pub docs_url: Option<String>,
    pub first_party: bool,
    pub default_model: Option<String>,
    /// True when a non-empty API key is already in the secret store for this provider.
    /// Never includes the key itself — UI can offer "use saved key" without re-paste.
    #[serde(default)]
    pub has_saved_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectResult {
    pub connection: ProviderConnectionDto,
    pub probe: ProbeResult,
}

/// List hero cards for onboarding UI. When `secrets` is provided, each card
/// reports whether a key is already on file (metadata only — no values).
/// When `db` is provided, `default_model` prefers the last selection for that
/// provider (`provider.{id}.default_model`) so switch-back restores the model.
pub fn list_hero_cards(
    secrets: Option<&SharedSecretStore>,
    db: Option<&Database>,
) -> Result<Vec<HeroCard>, String> {
    let cat = load_bundled_catalog()?;
    Ok(hero_provider_ids()
        .iter()
        .filter_map(|id| {
            let p = cat.providers.iter().find(|x| x.id == *id)?;
            let has_saved_key = secrets
                .map(|s| {
                    s.get(&store_key_for(id))
                        .ok()
                        .flatten()
                        .map(|v| !v.trim().is_empty())
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            let stored_model = db.and_then(|database| {
                database
                    .get_setting("user", &format!("provider.{id}.default_model"))
                    .ok()
                    .flatten()
                    .and_then(|s| serde_json::from_str::<String>(&s.value_json).ok())
                    .filter(|m| !m.trim().is_empty())
            });
            Some(HeroCard {
                id: p.id.clone(),
                name: p.name.clone(),
                docs_url: p.docs_url.clone(),
                first_party: true,
                default_model: stored_model.or_else(|| select_default_model(p)),
                has_saved_key,
            })
        })
        .collect())
}

/// List catalog providers for expandable grid (advisory).
pub fn list_catalog_cards() -> Result<Vec<CatalogProvider>, String> {
    Ok(load_bundled_catalog()?.providers)
}

fn store_key_for(provider_id: &str) -> String {
    format!("loopcode/provider/{provider_id}/api_key")
}

/// Connect first-party provider: put secret in Core store, secret_ref in SQLite,
/// then perform a live probe. `fixture_success` is test-only.
///
/// `api_key` may be empty when a key is already stored for this provider — the
/// saved secret is reused so users can switch providers without re-pasting.
pub fn connect_first_party(
    db: &Database,
    secrets: &SharedSecretStore,
    provider_id: &str,
    api_key: &str,
    fixture_success: Option<bool>,
) -> Result<ConnectResult, String> {
    let _ = first_party_adapter(provider_id)
        .ok_or_else(|| format!("not a first-party adapter: {provider_id}"))?;

    let store_key = store_key_for(provider_id);
    let incoming = api_key.trim();
    let reusing_saved = incoming.is_empty();

    if reusing_saved {
        let existing = secrets.get(&store_key)?;
        if existing.as_ref().map(|v| v.trim().is_empty()).unwrap_or(true) {
            return Err("api key required — none saved for this provider yet".into());
        }
        let _ = audit::audit_secret_access(db, &store_key, None, None, None, "get")?;
    } else {
        secrets.set(&store_key, incoming)?;
        let _ = audit::audit_secret_access(db, &store_key, None, None, None, "set")?;
    }

    // SQLite: reference only (metadata). Always record a ref so connection DTO has an id.
    let sref = db.insert_secret_ref(
        "provider_api_key",
        provider_id,
        &store_key,
        Some(provider_id),
        None,
    )?;

    // Retrieve for probe without returning key to caller DTOs
    let key = secrets
        .get(&store_key)?
        .ok_or_else(|| "secret missing after set".to_string())?;
    // Leak check uses the key actually used for the probe (new or reused).
    let leak_check_key = key.clone();
    let probe = probe_first_party(provider_id, &key, fixture_success)?;
    // Drop key from stack ASAP (not perfect but no DTO leak)
    drop(key);

    let cat = load_bundled_catalog()?;
    let provider = cat
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or("catalog missing provider")?;

    let default_model = probe
        .model
        .clone()
        .or_else(|| select_default_model(provider));

    if probe.ready {
        if let Some(m) = &default_model {
            db.set_setting(
                "user",
                &format!("provider.{provider_id}.default_model"),
                &json!(m).to_string(),
            )?;
            db.set_setting(
                "user",
                "provider.ready",
                &json!({
                    "providerId": provider_id,
                    "model": m,
                    "connectionId": sref.id,
                })
                .to_string(),
            )?;
        }
    }

    let connection = ProviderConnectionDto {
        id: sref.id.clone(),
        provider_id: provider_id.into(),
        display_name: provider.name.clone(),
        ready: probe.ready,
        default_model: if probe.ready {
            default_model
        } else {
            None
        },
        secret_ref_id: Some(sref.id),
        store_key: store_key.clone(),
        last_probe_error: probe.error.as_ref().map(|e| e.message.clone()),
    };

    // Ensure DTO serialization cannot include the raw key (new or reused).
    let check = serde_json::to_string(&connection).unwrap_or_default();
    if !leak_check_key.is_empty() && check.contains(&leak_check_key) {
        return Err("internal: connection DTO leaked api key".into());
    }

    Ok(ConnectResult { connection, probe })
}

/// Create a custom profile config in settings (no live network).
pub fn create_custom_profile(
    db: &Database,
    display_name: &str,
    protocol: &str,
    base_url: &str,
    model: Option<&str>,
) -> Result<ValueDto, String> {
    let id = format!("custom-{}", Uuid::new_v4());
    let proto = crate::providers::adapter::Protocol::parse(protocol)
        .ok_or_else(|| format!("invalid protocol: {protocol}"))?;
    let profile = json!({
        "id": id,
        "displayName": display_name,
        "protocol": proto.as_str(),
        "baseUrl": base_url,
        "defaultModel": model,
        "catalogExecutable": false,
    });
    db.set_setting("user", &format!("custom_profile.{id}"), &profile.to_string())?;
    Ok(ValueDto {
        value: profile,
        message: "custom profile saved; connect with secret + probe next".into(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueDto {
    pub value: serde_json::Value,
    pub message: String,
}

/// Read ready provider settings (no secrets).
pub fn get_ready_provider(db: &Database) -> Result<Option<serde_json::Value>, String> {
    Ok(db
        .get_setting("user", "provider.ready")?
        .map(|s| serde_json::from_str(&s.value_json).unwrap_or(serde_json::Value::Null)))
}

/// Persist the user's selected provider + model so restarts and `cockpit_send`
/// resolve the same pair. Requires a stored API key for the provider.
pub fn set_ready_provider_selection(
    db: &Database,
    secrets: &SharedSecretStore,
    provider_id: &str,
    model: &str,
) -> Result<serde_json::Value, String> {
    let provider_id = provider_id.trim();
    let model = model.trim();
    if provider_id.is_empty() {
        return Err("provider id required".into());
    }
    if model.is_empty() {
        return Err("model id required".into());
    }
    let _ = first_party_adapter(provider_id)
        .ok_or_else(|| format!("not a first-party adapter: {provider_id}"))?;

    let store_key = store_key_for(provider_id);
    let has_key = secrets
        .get(&store_key)?
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    if !has_key {
        return Err(format!(
            "no API key stored for provider {provider_id}; connect it first"
        ));
    }

    // Preserve connectionId when re-selecting the same provider; otherwise drop it.
    let prev = get_ready_provider(db)?;
    let connection_id = prev
        .as_ref()
        .and_then(|v| v.get("providerId").and_then(|p| p.as_str()))
        .filter(|p| *p == provider_id)
        .and_then(|_| {
            prev.as_ref()
                .and_then(|v| v.get("connectionId").and_then(|c| c.as_str()))
                .map(|s| s.to_string())
        });

    db.set_setting(
        "user",
        &format!("provider.{provider_id}.default_model"),
        &json!(model).to_string(),
    )?;

    let value = json!({
        "providerId": provider_id,
        "model": model,
        "connectionId": connection_id,
    });
    db.set_setting("user", "provider.ready", &value.to_string())?;
    Ok(value)
}
