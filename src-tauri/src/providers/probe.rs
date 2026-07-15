//! Connection probe — fixture transport by default; optional live HTTP behind env.

use crate::providers::adapter::{AdapterError, HttpRequestSpec, NormalizedText, ProviderAdapter};
use crate::providers::catalog::{
    load_bundled_catalog, next_model_after, select_default_model, CatalogProvider,
};
use crate::providers::first_party::first_party_adapter;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub ready: bool,
    pub provider_id: String,
    pub adapter_id: String,
    pub model: Option<String>,
    pub text: Option<NormalizedText>,
    pub error: Option<AdapterError>,
    pub notice: Option<String>,
    pub used_fixture: bool,
}

/// Transport for probe HTTP (fixtures or live).
pub trait ProbeTransport: Send + Sync {
    fn execute(&self, req: &HttpRequestSpec) -> Result<(u16, String), String>;
}

/// Recorded fixture transport — loads JSON from assets/fixtures/providers.
pub struct FixtureTransport {
    pub fixture_name: String,
}

impl FixtureTransport {
    pub fn for_provider(provider_id: &str, success: bool) -> Self {
        let name = match (provider_id, success) {
            ("openai", true) => "openai_probe_ok.json",
            ("openai", false) => "openai_probe_fail.json",
            ("anthropic", true) => "anthropic_probe_ok.json",
            ("anthropic", false) => "anthropic_probe_fail.json",
            ("openrouter", true) => "openrouter_probe_ok.json",
            ("opencode", true) => "opencode_probe_ok.json",
            (_, true) => "openai_probe_ok.json",
            (_, false) => "openai_probe_fail.json",
        };
        Self {
            fixture_name: name.into(),
        }
    }

    pub fn load_fixture(name: &str) -> Result<(u16, String), String> {
        let raw = match name {
            "openai_probe_ok.json" => {
                include_str!("../../assets/fixtures/providers/openai_probe_ok.json")
            }
            "openai_probe_fail.json" => {
                include_str!("../../assets/fixtures/providers/openai_probe_fail.json")
            }
            "openai_responses_probe_ok.json" => {
                include_str!("../../assets/fixtures/providers/openai_responses_probe_ok.json")
            }
            "anthropic_probe_ok.json" => {
                include_str!("../../assets/fixtures/providers/anthropic_probe_ok.json")
            }
            "anthropic_probe_fail.json" => {
                include_str!("../../assets/fixtures/providers/anthropic_probe_fail.json")
            }
            "openrouter_probe_ok.json" => {
                include_str!("../../assets/fixtures/providers/openrouter_probe_ok.json")
            }
            "opencode_probe_ok.json" => {
                include_str!("../../assets/fixtures/providers/opencode_probe_ok.json")
            }
            other => return Err(format!("unknown fixture: {other}")),
        };
        let v: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        let status = v.get("status").and_then(|s| s.as_u64()).unwrap_or(500) as u16;
        let body = v
            .get("body")
            .cloned()
            .unwrap_or(Value::Null)
            .to_string();
        Ok((status, body))
    }
}

impl ProbeTransport for FixtureTransport {
    fn execute(&self, _req: &HttpRequestSpec) -> Result<(u16, String), String> {
        // Never use req headers (would contain secrets) in fixture path logging
        Self::load_fixture(&self.fixture_name)
    }
}

/// Scripted transport: pops sequential (status, body) responses for multi-step probes.
pub struct SequenceTransport {
    queue: Mutex<VecDeque<(u16, String)>>,
}

impl SequenceTransport {
    pub fn new(responses: Vec<(u16, String)>) -> Self {
        Self {
            queue: Mutex::new(VecDeque::from(responses)),
        }
    }

    /// Fail once (401), then success body — drives default→next model advance.
    pub fn fail_then_success(fail_body: &str, success_status: u16, success_body: &str) -> Self {
        Self::new(vec![
            (401, fail_body.to_string()),
            (success_status, success_body.to_string()),
        ])
    }
}

impl ProbeTransport for SequenceTransport {
    fn execute(&self, _req: &HttpRequestSpec) -> Result<(u16, String), String> {
        let mut q = self
            .queue
            .lock()
            .map_err(|_| "sequence transport lock poisoned".to_string())?;
        q.pop_front()
            .ok_or_else(|| "sequence transport exhausted".to_string())
    }
}

/// Run probe against an adapter using the given transport.
pub fn run_probe(
    adapter: &dyn ProviderAdapter,
    model: &str,
    api_key: &str,
    transport: &dyn ProbeTransport,
    used_fixture: bool,
) -> ProbeResult {
    let req = adapter.build_probe_request(model, api_key);
    // Ensure secrets are not in URL
    debug_assert!(!req.url.contains(api_key));

    match transport.execute(&req) {
        Ok((status, body)) => match adapter.parse_probe_response(status, &body, model) {
            Ok(text) => ProbeResult {
                ready: true,
                provider_id: adapter.id().into(),
                adapter_id: text.adapter_id.clone(),
                model: Some(text.model.clone()),
                text: Some(text),
                error: None,
                notice: None,
                used_fixture,
            },
            Err(err) => ProbeResult {
                ready: false,
                provider_id: adapter.id().into(),
                adapter_id: err.adapter_id.clone(),
                model: Some(model.into()),
                text: None,
                error: Some(*err),
                notice: None,
                used_fixture,
            },
        },
        Err(e) => ProbeResult {
            ready: false,
            provider_id: adapter.id().into(),
            adapter_id: adapter.id().into(),
            model: Some(model.into()),
            text: None,
            error: Some(AdapterError {
                category: "network".into(),
                message: e,
                provider_id: adapter.id().into(),
                adapter_id: adapter.id().into(),
                retryable: true,
                status: None,
                detail: None,
            }),
            notice: None,
            used_fixture,
        },
    }
}

/// Core default-model selection + next-on-failure with a supplied transport.
pub fn probe_with_model_fallback(
    adapter: &dyn ProviderAdapter,
    provider: &CatalogProvider,
    api_key: &str,
    transport: &dyn ProbeTransport,
    used_fixture: bool,
) -> ProbeResult {
    let mut model = match select_default_model(provider) {
        Some(m) => m,
        None => {
            return ProbeResult {
                ready: false,
                provider_id: adapter.id().into(),
                adapter_id: adapter.id().into(),
                model: None,
                text: None,
                error: Some(AdapterError {
                    category: "configuration".into(),
                    message: "no model available in catalog".into(),
                    provider_id: adapter.id().into(),
                    adapter_id: adapter.id().into(),
                    retryable: false,
                    status: None,
                    detail: None,
                }),
                notice: None,
                used_fixture,
            };
        }
    };

    let first_model = model.clone();
    let mut result = run_probe(adapter, &model, api_key, transport, used_fixture);
    if !result.ready {
        if let Some(next) = next_model_after(provider, &model) {
            model = next;
            result = run_probe(adapter, &model, api_key, transport, used_fixture);
            if result.ready {
                result.notice = Some(format!(
                    "default model ({first_model}) failed probe; selected next model: {model}"
                ));
            }
        }
    }
    result
}

/// Probe a first-party provider with default model selection and optional next-model advance.
///
/// Production probes always use live HTTPS. `fixture_success` exists solely for
/// deterministic Core tests and is never supplied by WebView IPC.
pub fn probe_first_party(
    provider_id: &str,
    api_key: &str,
    fixture_success: Option<bool>,
) -> Result<ProbeResult, String> {
    let adapter =
        first_party_adapter(provider_id).ok_or_else(|| format!("unknown adapter: {provider_id}"))?;
    let catalog = load_bundled_catalog()?;
    let provider = catalog
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| format!("provider not in catalog: {provider_id}"))?;

    if fixture_success.is_none() {
        let live = crate::providers::transport::LiveHttpTransport::new()?;
        let bridge = LiveProbeBridge { inner: live };
        return Ok(probe_with_model_fallback(
            adapter.as_ref(),
            provider,
            api_key,
            &bridge,
            false,
        ));
    }

    let transport = FixtureTransport::for_provider(provider_id, fixture_success.unwrap_or(true));
    Ok(probe_with_model_fallback(
        adapter.as_ref(),
        provider,
        api_key,
        &transport,
        true,
    ))
}

/// Adapts [`HttpTransport`] to the probe-only [`ProbeTransport`] trait.
struct LiveProbeBridge {
    inner: crate::providers::transport::LiveHttpTransport,
}

impl ProbeTransport for LiveProbeBridge {
    fn execute(&self, req: &HttpRequestSpec) -> Result<(u16, String), String> {
        use crate::providers::transport::HttpTransport;
        self.inner.execute(req)
    }
}
