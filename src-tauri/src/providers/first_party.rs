//! Four signed first-party adapters.

use crate::providers::adapter::{
    anthropic_headers, anthropic_messages_probe_body, bearer_headers, openai_chat_probe_body,
    parse_anthropic_message, parse_openai_chat_completion, AdapterError, HttpRequestSpec,
    NormalizedText, Protocol, ProviderAdapter,
};
use std::collections::HashMap;

pub struct OpenAiAdapter;
pub struct AnthropicAdapter;
pub struct OpenRouterAdapter;
pub struct OpenCodeZenAdapter;

impl ProviderAdapter for OpenAiAdapter {
    fn id(&self) -> &'static str {
        "openai"
    }
    fn display_name(&self) -> &'static str {
        "OpenAI"
    }
    fn protocol(&self) -> Protocol {
        Protocol::OpenaiChatCompletions
    }
    fn default_base_url(&self) -> &'static str {
        "https://api.openai.com/v1"
    }
    fn build_probe_request(&self, model: &str, api_key: &str) -> HttpRequestSpec {
        HttpRequestSpec {
            method: "POST".into(),
            url: format!("{}/chat/completions", self.default_base_url()),
            headers: bearer_headers(api_key),
            body: openai_chat_probe_body(model),
        }
    }
    fn parse_probe_response(
        &self,
        status: u16,
        body: &str,
        model: &str,
    ) -> Result<NormalizedText, Box<AdapterError>> {
        parse_openai_chat_completion(
            self.id(),
            "openai.v1",
            Protocol::OpenaiChatCompletions,
            status,
            body,
            model,
        )
    }
}

impl ProviderAdapter for AnthropicAdapter {
    fn id(&self) -> &'static str {
        "anthropic"
    }
    fn display_name(&self) -> &'static str {
        "Anthropic"
    }
    fn protocol(&self) -> Protocol {
        Protocol::AnthropicMessages
    }
    fn default_base_url(&self) -> &'static str {
        "https://api.anthropic.com/v1"
    }
    fn build_probe_request(&self, model: &str, api_key: &str) -> HttpRequestSpec {
        HttpRequestSpec {
            method: "POST".into(),
            url: format!("{}/messages", self.default_base_url()),
            headers: anthropic_headers(api_key),
            body: anthropic_messages_probe_body(model),
        }
    }
    fn parse_probe_response(
        &self,
        status: u16,
        body: &str,
        model: &str,
    ) -> Result<NormalizedText, Box<AdapterError>> {
        parse_anthropic_message(self.id(), "anthropic.v1", status, body, model)
    }
}

impl ProviderAdapter for OpenRouterAdapter {
    fn id(&self) -> &'static str {
        "openrouter"
    }
    fn display_name(&self) -> &'static str {
        "OpenRouter"
    }
    fn protocol(&self) -> Protocol {
        Protocol::OpenaiChatCompletions
    }
    fn default_base_url(&self) -> &'static str {
        "https://openrouter.ai/api/v1"
    }
    fn extra_headers(&self) -> HashMap<String, String> {
        // Hard-coded attribution — never injected from catalog
        let mut h = HashMap::new();
        h.insert("HTTP-Referer".into(), "https://loopcode.local".into());
        h.insert("X-Title".into(), "LoopCode".into());
        h
    }
    fn build_probe_request(&self, model: &str, api_key: &str) -> HttpRequestSpec {
        let mut headers = bearer_headers(api_key);
        headers.extend(self.extra_headers());
        HttpRequestSpec {
            method: "POST".into(),
            url: format!("{}/chat/completions", self.default_base_url()),
            headers,
            body: openai_chat_probe_body(model),
        }
    }
    fn parse_probe_response(
        &self,
        status: u16,
        body: &str,
        model: &str,
    ) -> Result<NormalizedText, Box<AdapterError>> {
        parse_openai_chat_completion(
            self.id(),
            "openrouter.v1",
            Protocol::OpenaiChatCompletions,
            status,
            body,
            model,
        )
    }
}

impl ProviderAdapter for OpenCodeZenAdapter {
    fn id(&self) -> &'static str {
        "opencode"
    }
    fn display_name(&self) -> &'static str {
        "OpenCode Zen"
    }
    fn protocol(&self) -> Protocol {
        Protocol::OpenaiChatCompletions
    }
    fn default_base_url(&self) -> &'static str {
        "https://opencode.ai/zen/v1"
    }
    fn build_probe_request(&self, model: &str, api_key: &str) -> HttpRequestSpec {
        HttpRequestSpec {
            method: "POST".into(),
            url: format!("{}/chat/completions", self.default_base_url()),
            headers: bearer_headers(api_key),
            body: openai_chat_probe_body(model),
        }
    }
    fn parse_probe_response(
        &self,
        status: u16,
        body: &str,
        model: &str,
    ) -> Result<NormalizedText, Box<AdapterError>> {
        parse_openai_chat_completion(
            self.id(),
            "opencode.zen.v1",
            Protocol::OpenaiChatCompletions,
            status,
            body,
            model,
        )
    }
}

/// Resolve first-party adapter by provider id.
pub fn first_party_adapter(id: &str) -> Option<Box<dyn ProviderAdapter>> {
    match id {
        "openai" => Some(Box::new(OpenAiAdapter)),
        "anthropic" => Some(Box::new(AnthropicAdapter)),
        "openrouter" => Some(Box::new(OpenRouterAdapter)),
        "opencode" => Some(Box::new(OpenCodeZenAdapter)),
        _ => None,
    }
}

pub fn hero_provider_ids() -> &'static [&'static str] {
    &["openai", "anthropic", "openrouter", "opencode"]
}
