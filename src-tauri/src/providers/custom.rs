//! Explicit custom protocol profiles (not catalog-executable).

use crate::providers::adapter::{
    anthropic_headers, anthropic_messages_probe_body, bearer_headers, openai_chat_probe_body,
    openai_responses_probe_body, parse_anthropic_message, parse_openai_chat_completion,
    parse_openai_responses, AdapterError, HttpRequestSpec, NormalizedText, Protocol,
    ProviderAdapter,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomProfile {
    pub id: String,
    pub display_name: String,
    pub protocol: Protocol,
    pub base_url: String,
    pub default_model: Option<String>,
}

pub struct CustomProfileAdapter {
    pub profile: CustomProfile,
}

impl ProviderAdapter for CustomProfileAdapter {
    fn id(&self) -> &'static str {
        // Trait requires 'static — custom profiles use dynamic ids via profile_id()
        "custom"
    }
    fn display_name(&self) -> &'static str {
        "Custom"
    }
    fn protocol(&self) -> Protocol {
        self.profile.protocol
    }
    fn default_base_url(&self) -> &'static str {
        ""
    }
    fn build_probe_request(&self, model: &str, api_key: &str) -> HttpRequestSpec {
        let base = self.profile.base_url.trim_end_matches('/');
        match self.profile.protocol {
            Protocol::AnthropicMessages => HttpRequestSpec {
                method: "POST".into(),
                url: format!("{base}/messages"),
                headers: anthropic_headers(api_key),
                body: anthropic_messages_probe_body(model),
            },
            Protocol::OpenaiChatCompletions => HttpRequestSpec {
                method: "POST".into(),
                url: format!("{base}/chat/completions"),
                headers: bearer_headers(api_key),
                body: openai_chat_probe_body(model),
            },
            Protocol::OpenaiResponses => HttpRequestSpec {
                method: "POST".into(),
                url: format!("{base}/responses"),
                headers: bearer_headers(api_key),
                body: openai_responses_probe_body(model),
            },
        }
    }
    fn parse_probe_response(
        &self,
        status: u16,
        body: &str,
        model: &str,
    ) -> Result<NormalizedText, Box<AdapterError>> {
        let provider_id = self.profile.id.as_str();
        match self.profile.protocol {
            Protocol::AnthropicMessages => {
                parse_anthropic_message(provider_id, "custom.anthropic", status, body, model)
            }
            Protocol::OpenaiChatCompletions => parse_openai_chat_completion(
                provider_id,
                "custom.openai.chat",
                Protocol::OpenaiChatCompletions,
                status,
                body,
                model,
            ),
            Protocol::OpenaiResponses => {
                parse_openai_responses(provider_id, "custom.openai.responses", status, body, model)
            }
        }
    }
}

impl CustomProfileAdapter {
    pub fn new(profile: CustomProfile) -> Self {
        Self { profile }
    }

    pub fn profile_id(&self) -> &str {
        &self.profile.id
    }
}
