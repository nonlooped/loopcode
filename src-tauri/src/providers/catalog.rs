//! Bundled models.dev catalog — advisory only; never executes npm/env recipes.
//!
//! Refresh with: `node scripts/refresh-models-catalog.mjs`

use crate::providers::first_party::hero_provider_ids;
use serde::{Deserialize, Serialize};

/// Embedded snapshot from https://models.dev/api.json (refresh via script).
pub const BUNDLED_CATALOG_JSON: &str =
    include_str!("../../assets/catalog/models-dev-snapshot.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSnapshot {
    pub snapshot_version: String,
    pub source: String,
    pub generated_at: String,
    pub providers: Vec<CatalogProvider>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogProvider {
    pub id: String,
    pub name: String,
    pub docs_url: Option<String>,
    pub api: Option<String>,
    /// Catalog may list npm package names — **never execute**.
    pub npm: Option<String>,
    pub env: Option<Vec<String>>,
    pub default_model: Option<String>,
    pub models: Vec<CatalogModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogModel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub default: bool,
    /// Model can reason/think at all (models.dev `reasoning`).
    #[serde(default)]
    pub reasoning: bool,
    /// Named effort levels the model accepts, canonical low→high order.
    #[serde(default)]
    pub reasoning_efforts: Vec<String>,
    /// Model supports a plain on/off reasoning switch.
    #[serde(default)]
    pub reasoning_toggle: bool,
}

/// Load the real bundled snapshot from the repo asset (shipped via include_str).
pub fn load_bundled_catalog() -> Result<CatalogSnapshot, String> {
    // Reject accidental execution markers being treated as runnable
    if BUNDLED_CATALOG_JSON.contains("\"executePackage\"") {
        return Err("catalog must not contain executable package directives".into());
    }
    serde_json::from_str(BUNDLED_CATALOG_JSON).map_err(|e| format!("catalog parse: {e}"))
}

/// Hero cards for onboarding (four first-party).
pub fn hero_providers(catalog: &CatalogSnapshot) -> Vec<CatalogProvider> {
    hero_provider_ids()
        .iter()
        .filter_map(|id| catalog.providers.iter().find(|p| p.id == *id).cloned())
        .collect()
}

/// Expandable catalog = all providers (heroes + rest).
pub fn all_providers(catalog: &CatalogSnapshot) -> &[CatalogProvider] {
    &catalog.providers
}

/// Default model selection after successful connection:
/// 1) catalog/adapter default 2) first listed 3) caller may advance on failure
pub fn select_default_model(provider: &CatalogProvider) -> Option<String> {
    if let Some(d) = &provider.default_model {
        if provider.models.iter().any(|m| m.id == *d) || provider.models.is_empty() {
            return Some(d.clone());
        }
    }
    if let Some(m) = provider.models.iter().find(|m| m.default) {
        return Some(m.id.clone());
    }
    provider.models.first().map(|m| m.id.clone())
}

/// Next model after `current` fails suitability/probe.
pub fn next_model_after(provider: &CatalogProvider, current: &str) -> Option<String> {
    let ids: Vec<&str> = provider.models.iter().map(|m| m.id.as_str()).collect();
    let pos = ids.iter().position(|id| *id == current)?;
    ids.get(pos + 1).map(|s| (*s).to_string())
}

/// Catalog npm field must never be treated as executable code path.
pub fn catalog_npm_is_non_executable(provider: &CatalogProvider) -> bool {
    provider
        .npm
        .as_deref()
        .map(|n| n.starts_with("DO_NOT_EXECUTE") || n.contains("DO_NOT_EXECUTE"))
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_has_four_heroes_and_opencode_zen() {
        let cat = load_bundled_catalog().unwrap();
        let heroes = hero_providers(&cat);
        assert_eq!(heroes.len(), 4);
        assert!(heroes.iter().any(|p| p.id == "opencode"));
        let oc = heroes.iter().find(|p| p.id == "opencode").unwrap();
        assert!(oc.name.contains("OpenCode") || oc.name.contains("Zen"));
        for h in &heroes {
            assert!(catalog_npm_is_non_executable(h));
        }
    }
}
