//! Provider adapter contract — pure mapping + probe (fixtures or live transport).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Wire protocol family for first-party and custom profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    OpenaiChatCompletions,
    OpenaiResponses,
    AnthropicMessages,
}

impl Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenaiChatCompletions => "openai_chat_completions",
            Self::OpenaiResponses => "openai_responses",
            Self::AnthropicMessages => "anthropic_messages",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "openai_chat_completions" | "openai-completions" | "chat_completions" => {
                Some(Self::OpenaiChatCompletions)
            }
            "openai_responses" | "openai-responses" | "responses" => Some(Self::OpenaiResponses),
            "anthropic_messages" | "anthropic-messages" | "messages" => {
                Some(Self::AnthropicMessages)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestSpec {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedText {
    pub text: String,
    pub model: String,
    pub provider_id: String,
    pub adapter_id: String,
    pub protocol: String,
    pub finish_reason: Option<String>,
    pub usage: Option<Value>,
    pub provenance: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterError {
    pub category: String,
    pub message: String,
    pub provider_id: String,
    pub adapter_id: String,
    pub retryable: bool,
    pub status: Option<u16>,
    /// Redacted/truncated provider detail (not full bodies).
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeSuccess {
    pub ready: bool,
    pub text: NormalizedText,
    pub models_hint: Vec<String>,
}

/// Signed first-party or custom profile adapter.
pub trait ProviderAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn protocol(&self) -> Protocol;
    fn default_base_url(&self) -> &'static str;

    /// Build a minimal text probe request (never logs the key).
    fn build_probe_request(&self, model: &str, api_key: &str) -> HttpRequestSpec;

    /// Parse HTTP response into normalized text or structured error.
    fn parse_probe_response(
        &self,
        status: u16,
        body: &str,
        model: &str,
    ) -> Result<NormalizedText, Box<AdapterError>>;

    /// Hard-coded attribution headers (never from catalog).
    fn extra_headers(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

pub fn bearer_headers(api_key: &str) -> HashMap<String, String> {
    let mut h = HashMap::new();
    h.insert("Authorization".into(), format!("Bearer {api_key}"));
    h.insert("Content-Type".into(), "application/json".into());
    h
}

pub fn anthropic_headers(api_key: &str) -> HashMap<String, String> {
    let mut h = HashMap::new();
    h.insert("x-api-key".into(), api_key.to_string());
    h.insert("anthropic-version".into(), "2023-06-01".into());
    h.insert("Content-Type".into(), "application/json".into());
    h
}

pub fn openai_chat_probe_body(model: &str) -> Value {
    json!({
        "model": model,
        "messages": [{"role": "user", "content": "Reply with exactly: pong"}],
        "max_tokens": 16,
        "temperature": 0
    })
}

/// OpenAI Responses API probe body (distinct from Chat Completions).
pub fn openai_responses_probe_body(model: &str) -> Value {
    json!({
        "model": model,
        "input": "Reply with exactly: pong",
        "max_output_tokens": 16
    })
}

pub fn anthropic_messages_probe_body(model: &str) -> Value {
    json!({
        "model": model,
        "max_tokens": 16,
        "messages": [{"role": "user", "content": "Reply with exactly: pong"}]
    })
}

fn truncate_detail(body: &str) -> String {
    body.chars().take(200).collect()
}

/// Map non-success HTTP statuses to structured adapter errors (shared by probe + chat).
pub fn http_status_error_pub(
    provider_id: &str,
    adapter_id: &str,
    status: u16,
    body: &str,
) -> Option<Box<AdapterError>> {
    http_status_error(provider_id, adapter_id, status, body)
}

fn http_status_error(
    provider_id: &str,
    adapter_id: &str,
    status: u16,
    body: &str,
) -> Option<Box<AdapterError>> {
    if status == 401 || status == 403 {
        return Some(Box::new(AdapterError {
            category: "authentication".into(),
            message: "authentication failed".into(),
            provider_id: provider_id.into(),
            adapter_id: adapter_id.into(),
            retryable: false,
            status: Some(status),
            detail: Some(truncate_detail(body)),
        }));
    }
    if !(200..300).contains(&status) {
        return Some(Box::new(AdapterError {
            category: "provider_error".into(),
            message: format!("provider HTTP {status}"),
            provider_id: provider_id.into(),
            adapter_id: adapter_id.into(),
            retryable: status == 429 || status >= 500,
            status: Some(status),
            detail: Some(truncate_detail(body)),
        }));
    }
    None
}

pub fn parse_openai_chat_completion(
    provider_id: &str,
    adapter_id: &str,
    protocol: Protocol,
    status: u16,
    body: &str,
    fallback_model: &str,
) -> Result<NormalizedText, Box<AdapterError>> {
    if let Some(err) = http_status_error(provider_id, adapter_id, status, body) {
        return Err(err);
    }
    let v: Value = serde_json::from_str(body).map_err(|e| {
        Box::new(AdapterError {
            category: "schema_validation".into(),
            message: format!("invalid JSON: {e}"),
            provider_id: provider_id.into(),
            adapter_id: adapter_id.into(),
            retryable: false,
            status: Some(status),
            detail: None,
        })
    })?;
    let text = v
        .pointer("/choices/0/message/content")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
    if text.is_empty() {
        return Err(Box::new(AdapterError {
            category: "model_output".into(),
            message: "empty assistant content".into(),
            provider_id: provider_id.into(),
            adapter_id: adapter_id.into(),
            retryable: false,
            status: Some(status),
            detail: Some(truncate_detail(body)),
        }));
    }
    let model = v
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(fallback_model)
        .to_string();
    Ok(NormalizedText {
        text,
        model: model.clone(),
        provider_id: provider_id.into(),
        adapter_id: adapter_id.into(),
        protocol: protocol.as_str().into(),
        finish_reason: v
            .pointer("/choices/0/finish_reason")
            .and_then(|f| f.as_str())
            .map(str::to_string),
        usage: v.get("usage").cloned(),
        provenance: json!({
            "adapterId": adapter_id,
            "providerId": provider_id,
            "protocol": protocol.as_str(),
            "model": model,
            "endpointClass": "chat.completions",
            "catalogExecutable": false,
        }),
    })
}

/// Parse OpenAI Responses API JSON (`/v1/responses`) — not Chat Completions.
pub fn parse_openai_responses(
    provider_id: &str,
    adapter_id: &str,
    status: u16,
    body: &str,
    fallback_model: &str,
) -> Result<NormalizedText, Box<AdapterError>> {
    if let Some(err) = http_status_error(provider_id, adapter_id, status, body) {
        return Err(err);
    }
    let v: Value = serde_json::from_str(body).map_err(|e| {
        Box::new(AdapterError {
            category: "schema_validation".into(),
            message: format!("invalid JSON: {e}"),
            provider_id: provider_id.into(),
            adapter_id: adapter_id.into(),
            retryable: false,
            status: Some(status),
            detail: None,
        })
    })?;

    // Responses API: output[].content[].text (type output_text) or output_text top-level
    let text = extract_responses_text(&v);
    if text.is_empty() {
        return Err(Box::new(AdapterError {
            category: "model_output".into(),
            message: "empty responses output_text".into(),
            provider_id: provider_id.into(),
            adapter_id: adapter_id.into(),
            retryable: false,
            status: Some(status),
            detail: Some(truncate_detail(body)),
        }));
    }
    let model = v
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(fallback_model)
        .to_string();
    Ok(NormalizedText {
        text,
        model: model.clone(),
        provider_id: provider_id.into(),
        adapter_id: adapter_id.into(),
        protocol: Protocol::OpenaiResponses.as_str().into(),
        finish_reason: v
            .get("status")
            .and_then(|s| s.as_str())
            .map(str::to_string),
        usage: v.get("usage").cloned(),
        provenance: json!({
            "adapterId": adapter_id,
            "providerId": provider_id,
            "protocol": Protocol::OpenaiResponses.as_str(),
            "model": model,
            "endpointClass": "responses",
            "catalogExecutable": false,
        }),
    })
}

fn extract_responses_text(v: &Value) -> String {
    if let Some(t) = v.get("output_text").and_then(|x| x.as_str()) {
        return t.to_string();
    }
    if let Some(arr) = v.get("output").and_then(|o| o.as_array()) {
        for item in arr {
            if let Some(content) = item.get("content").and_then(|c| c.as_array()) {
                for part in content {
                    if part.get("type").and_then(|t| t.as_str()) == Some("output_text") {
                        if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                            return t.to_string();
                        }
                    }
                    // Some shapes use plain text field
                    if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                        return t.to_string();
                    }
                }
            }
        }
    }
    String::new()
}

pub fn parse_anthropic_message(
    provider_id: &str,
    adapter_id: &str,
    status: u16,
    body: &str,
    fallback_model: &str,
) -> Result<NormalizedText, Box<AdapterError>> {
    if let Some(err) = http_status_error(provider_id, adapter_id, status, body) {
        return Err(err);
    }
    let v: Value = serde_json::from_str(body).map_err(|e| {
        Box::new(AdapterError {
            category: "schema_validation".into(),
            message: format!("invalid JSON: {e}"),
            provider_id: provider_id.into(),
            adapter_id: adapter_id.into(),
            retryable: false,
            status: Some(status),
            detail: None,
        })
    })?;
    let text = v
        .pointer("/content/0/text")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    if text.is_empty() {
        return Err(Box::new(AdapterError {
            category: "model_output".into(),
            message: "empty assistant content".into(),
            provider_id: provider_id.into(),
            adapter_id: adapter_id.into(),
            retryable: false,
            status: Some(status),
            detail: Some(truncate_detail(body)),
        }));
    }
    let model = v
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(fallback_model)
        .to_string();
    Ok(NormalizedText {
        text,
        model: model.clone(),
        provider_id: provider_id.into(),
        adapter_id: adapter_id.into(),
        protocol: Protocol::AnthropicMessages.as_str().into(),
        finish_reason: v
            .get("stop_reason")
            .and_then(|s| s.as_str())
            .map(str::to_string),
        usage: v.get("usage").cloned(),
        provenance: json!({
            "adapterId": adapter_id,
            "providerId": provider_id,
            "protocol": Protocol::AnthropicMessages.as_str(),
            "model": model,
            "endpointClass": "messages",
            "catalogExecutable": false,
        }),
    })
}
