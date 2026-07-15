//! Chat/agent request builders and response parsers (text + tool calls).
//! Pure mapping — transport is injected separately.

use crate::providers::adapter::{
    anthropic_headers, bearer_headers, http_status_error_pub, AdapterError, HttpRequestSpec,
    Protocol, ProviderAdapter,
};
use crate::runtime::tools::ToolDefinition;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

/// Canonical in-memory message for multi-round agent turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// JSON object arguments (already parsed when possible).
    pub arguments: Value,
}

/// Normalized assistant turn after one model request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantTurn {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub model: String,
    pub provider_id: String,
    pub adapter_id: String,
    pub protocol: String,
    pub finish_reason: Option<String>,
    pub usage: Option<Value>,
    /// True when this turn is a partial stream chunk (not yet durable final).
    pub partial: bool,
}

impl AssistantTurn {
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// User-selected reasoning settings for a run (composer → run payload).
/// `enabled: false` means the user turned reasoning off; `effort: None` with
/// `enabled: true` means "on, provider default depth".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningConfig {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
}

/// OpenAI / Anthropic tool function names must match `^[a-zA-Z0-9_-]{1,64}$`.
/// Flat catalog names are already safe; map remaining dots for any legacy ids.
pub fn to_provider_tool_name(name: &str) -> String {
    name.replace('.', "_")
}

/// Map a provider-returned tool name back to LoopCode's internal catalog id.
pub fn from_provider_tool_name(wire: &str) -> String {
    let catalog = crate::tools::catalog::builtin_catalog();
    if catalog.iter().any(|d| d.name == wire) {
        return wire.to_string();
    }
    for d in &catalog {
        if to_provider_tool_name(&d.name) == wire {
            return d.name.clone();
        }
    }
    // MCP merged tools and unknown names pass through.
    wire.to_string()
}

/// Minimal JSON Schema stubs for built-in tools (model wire format only).
pub fn tool_wire_schema(def: &ToolDefinition) -> Value {
    if let Some(schema) = &def.input_schema {
        let description = def.description.as_str();
        let properties = schema
            .get("properties")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let required = schema.get("required").cloned().unwrap_or_else(|| json!([]));
        return json!({
            "type": "function",
            "function": {
                "name": to_provider_tool_name(&def.name),
                "description": description,
                "parameters": {
                    "type": "object",
                    "properties": properties,
                    "required": required,
                }
            }
        });
    }

    let (description, properties, required) = match def.name.as_str() {
        "glob" => (
            "Find files by glob pattern relative to the project root (e.g. **/*.rs).",
            json!({
                "pattern": {"type": "string", "description": "Glob pattern"},
                "path": {"type": "string", "description": "Optional subdirectory to search under"}
            }),
            json!(["pattern"]),
        ),
        "read" => (
            def.description.as_str(),
            json!({"path": {"type": "string", "description": "Path relative to project root"}}),
            json!(["path"]),
        ),
        "grep" => (
            "Search file contents (bounded). If capped/nextOffset is set, page with offset.",
            json!({
                "query": {"type": "string"},
                "path": {"type": "string", "description": "Optional subdirectory"},
                "maxMatches": {"type": "integer", "description": "Page size (default 200, max 1000)"},
                "offset": {"type": "integer", "description": "Skip this many matches for pagination"}
            }),
            json!(["query"]),
        ),
        "write" => (
            def.description.as_str(),
            json!({
                "path": {"type": "string"},
                "content": {"type": "string"}
            }),
            json!(["path", "content"]),
        ),
        "edit" => (
            "Exact string replacement in a file. Prefer for small surgical edits. Use expectedHash from read when the file already exists.",
            json!({
                "path": {"type": "string", "description": "Path relative to project root"},
                "oldString": {"type": "string", "description": "Exact text to replace"},
                "newString": {"type": "string", "description": "Replacement text (must differ)"},
                "replaceAll": {"type": "boolean", "description": "Replace every occurrence (default false)"},
                "expectedHash": {"type": "string", "description": "Optional pre-image hash from read"}
            }),
            json!(["path", "oldString", "newString"]),
        ),
        "patch" => (
            "Replace file contents, or delete a file with delete:true. Prefer `content` (full new file body) with `expectedHash` from read. `newContent` is accepted as an alias for `content`. To delete a file, pass delete:true with the file's `expectedHash` (no content) — never delete files via exec/shell.",
            json!({
                "path": {"type": "string", "description": "Path relative to project root"},
                "content": {"type": "string", "description": "Full new file contents (omit when delete is true)"},
                "newContent": {"type": "string", "description": "Alias for content"},
                "expectedHash": {"type": "string", "description": "Pre-image hash from read"},
                "delete": {"type": "boolean", "description": "Delete the file instead of writing it (requires expectedHash)"}
            }),
            json!(["path"]),
        ),
        "exec" => (
            "Run a shell command in the project root. On Windows the host prefers pwsh, then powershell, then cmd — use that shell's syntax (not bash-only constructs unless Git Bash is the resolved host). Prefer edit/patch/write for file edits. This action pauses for user approval; after approval the host runs this exact call.",
            json!({
                "command": {"type": "string", "description": "Shell command using the host shell's syntax"},
                "cwd": {"type": "string", "description": "Optional subdirectory of the project root"}
            }),
            json!(["command"]),
        ),
        "webfetch" => (
            "Fetch a URL. This action pauses for user approval; after approval the host runs this exact call.",
            json!({"url": {"type": "string"}}),
            json!(["url"]),
        ),
        "plan_write" | "plan_update" => (
            def.description.as_str(),
            json!({
                "title": {"type": "string"},
                "body": {"type": "string"}
            }),
            json!(["title", "body"]),
        ),
        "context.attach" => (
            def.description.as_str(),
            json!({"path": {"type": "string"}}),
            json!(["path"]),
        ),
        "skill" => (
            "Load a skill body into context. Available skills are listed in the system prompt — pass the skill path from that list.",
            json!({"path": {"type": "string", "description": "Path to SKILL.md from the skills list"}}),
            json!(["path"]),
        ),
        _ => (
            def.description.as_str(),
            json!({"input": {"type": "object"}}),
            json!([]),
        ),
    };
    json!({
        "type": "function",
        "function": {
            "name": to_provider_tool_name(&def.name),
            "description": description,
            "parameters": {
                "type": "object",
                "properties": properties,
                "required": required,
            }
        }
    })
}

pub fn openai_tools_array(defs: &[ToolDefinition]) -> Value {
    Value::Array(defs.iter().map(tool_wire_schema).collect())
}

pub fn anthropic_tools_array(defs: &[ToolDefinition]) -> Value {
    Value::Array(
        defs.iter()
            .map(|d| {
                let schema = tool_wire_schema(d);
                let func = schema.get("function").cloned().unwrap_or(json!({}));
                json!({
                    "name": func.get("name").and_then(|n| n.as_str()).unwrap_or(&d.name),
                    "description": func.get("description").and_then(|x| x.as_str()).unwrap_or(""),
                    "input_schema": func.get("parameters").cloned().unwrap_or(json!({"type":"object"})),
                })
            })
            .collect(),
    )
}

fn openai_messages_wire(messages: &[ChatMessage]) -> Value {
    let arr: Vec<Value> = messages
        .iter()
        .map(|m| match m.role.as_str() {
            "tool" => json!({
                "role": "tool",
                "tool_call_id": m.tool_call_id,
                "content": m.content.clone().unwrap_or_default(),
            }),
            "assistant" if !m.tool_calls.is_empty() => {
                let tcs: Vec<Value> = m
                    .tool_calls
                    .iter()
                    .map(|tc| {
                        json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": to_provider_tool_name(&tc.name),
                                "arguments": match &tc.arguments {
                                    Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                }
                            }
                        })
                    })
                    .collect();
                json!({
                    "role": "assistant",
                    "content": m.content.clone().unwrap_or_default(),
                    "tool_calls": tcs,
                })
            }
            _ => json!({
                "role": m.role,
                "content": m.content.clone().unwrap_or_default(),
            }),
        })
        .collect();
    Value::Array(arr)
}

fn anthropic_messages_wire(messages: &[ChatMessage]) -> (Option<String>, Value) {
    let mut system = None;
    let mut out = Vec::new();
    for m in messages {
        match m.role.as_str() {
            "system" => {
                system = m.content.clone();
            }
            "user" => {
                out.push(json!({
                    "role": "user",
                    "content": m.content.clone().unwrap_or_default(),
                }));
            }
            "assistant" => {
                let mut content = Vec::new();
                if let Some(t) = &m.content {
                    if !t.is_empty() {
                        content.push(json!({"type": "text", "text": t}));
                    }
                }
                for tc in &m.tool_calls {
                    content.push(json!({
                        "type": "tool_use",
                        "id": tc.id,
                        "name": to_provider_tool_name(&tc.name),
                        "input": tc.arguments,
                    }));
                }
                if content.is_empty() {
                    content.push(json!({"type": "text", "text": ""}));
                }
                out.push(json!({"role": "assistant", "content": content}));
            }
            "tool" => {
                // Anthropic expects tool_result blocks inside a user message.
                let block = json!({
                    "type": "tool_result",
                    "tool_use_id": m.tool_call_id,
                    "content": m.content.clone().unwrap_or_default(),
                });
                if let Some(last) = out.last_mut() {
                    if last.get("role").and_then(|r| r.as_str()) == Some("user") {
                        if let Some(c) = last.get_mut("content") {
                            if let Some(arr) = c.as_array_mut() {
                                arr.push(block);
                                continue;
                            }
                            // content was a string — promote to array
                            let prev = c.as_str().unwrap_or("").to_string();
                            *c = json!([
                                {"type": "text", "text": prev},
                                block
                            ]);
                            continue;
                        }
                    }
                }
                out.push(json!({
                    "role": "user",
                    "content": [block],
                }));
            }
            _ => {}
        }
    }
    (system, Value::Array(out))
}

/// Build a chat/agent completion request for the given adapter protocol.
pub fn build_chat_request(
    adapter: &dyn ProviderAdapter,
    model: &str,
    api_key: &str,
    messages: &[ChatMessage],
    tools: &[ToolDefinition],
    base_url_override: Option<&str>,
    reasoning: Option<&ReasoningConfig>,
) -> HttpRequestSpec {
    // Only an explicit "on" changes the wire request; off/unset keeps the
    // legacy body so models without reasoning support are unaffected.
    let effort = reasoning
        .filter(|r| r.enabled)
        .map(|r| r.effort.as_deref().unwrap_or("medium").to_string());
    let reasoning_on = reasoning.map(|r| r.enabled).unwrap_or(false);
    let base = base_url_override
        .unwrap_or_else(|| adapter.default_base_url())
        .trim_end_matches('/');
    match adapter.protocol() {
        Protocol::OpenaiChatCompletions => {
            let mut headers = bearer_headers(api_key);
            headers.extend(adapter.extra_headers());
            let mut body = json!({
                "model": model,
                "messages": openai_messages_wire(messages),
            });
            if let Some(e) = &effort {
                // Reasoning models reject sampling params — send effort only.
                body.as_object_mut()
                    .unwrap()
                    .insert("reasoning_effort".into(), json!(e));
            } else {
                body.as_object_mut()
                    .unwrap()
                    .insert("temperature".into(), json!(0.2));
            }
            if !tools.is_empty() {
                body.as_object_mut()
                    .unwrap()
                    .insert("tools".into(), openai_tools_array(tools));
            }
            HttpRequestSpec {
                method: "POST".into(),
                url: format!("{base}/chat/completions"),
                headers,
                body,
            }
        }
        Protocol::OpenaiResponses => {
            // Map to Responses API shape (input items); tools when present.
            let mut headers = bearer_headers(api_key);
            headers.extend(adapter.extra_headers());
            let input: Vec<Value> = messages
                .iter()
                .filter(|m| m.role != "system")
                .map(|m| {
                    json!({
                        "role": m.role,
                        "content": m.content.clone().unwrap_or_default(),
                    })
                })
                .collect();
            let system = messages
                .iter()
                .find(|m| m.role == "system")
                .and_then(|m| m.content.clone());
            let mut body = json!({
                "model": model,
                "input": input,
                "max_output_tokens": 4096,
            });
            if reasoning_on {
                let mut r = serde_json::Map::new();
                if let Some(e) = &effort {
                    r.insert("effort".into(), json!(e));
                }
                body.as_object_mut()
                    .unwrap()
                    .insert("reasoning".into(), Value::Object(r));
            }
            if let Some(s) = system {
                body.as_object_mut()
                    .unwrap()
                    .insert("instructions".into(), json!(s));
            }
            if !tools.is_empty() {
                body.as_object_mut()
                    .unwrap()
                    .insert("tools".into(), openai_tools_array(tools));
            }
            HttpRequestSpec {
                method: "POST".into(),
                url: format!("{base}/responses"),
                headers,
                body,
            }
        }
        Protocol::AnthropicMessages => {
            let (system, msgs) = anthropic_messages_wire(messages);
            let mut body = json!({
                "model": model,
                "max_tokens": 4096,
                "messages": msgs,
            });
            if reasoning_on {
                // Adaptive thinking + output_config.effort (current Anthropic
                // API; budget_tokens is deprecated/rejected on new models).
                body.as_object_mut()
                    .unwrap()
                    .insert("thinking".into(), json!({ "type": "adaptive" }));
                if let Some(e) = &effort {
                    body.as_object_mut()
                        .unwrap()
                        .insert("output_config".into(), json!({ "effort": e }));
                }
            }
            if let Some(s) = system {
                body.as_object_mut()
                    .unwrap()
                    .insert("system".into(), json!(s));
            }
            if !tools.is_empty() {
                body.as_object_mut()
                    .unwrap()
                    .insert("tools".into(), anthropic_tools_array(tools));
            }
            HttpRequestSpec {
                method: "POST".into(),
                url: format!("{base}/messages"),
                headers: anthropic_headers(api_key),
                body,
            }
        }
    }
}

fn parse_args_json(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| json!({ "raw": raw }))
}

/// Parse OpenAI Chat Completions response into an assistant turn (text and/or tools).
pub fn parse_openai_chat_turn(
    provider_id: &str,
    adapter_id: &str,
    status: u16,
    body: &str,
    fallback_model: &str,
) -> Result<AssistantTurn, Box<AdapterError>> {
    if let Some(err) = http_status_error_pub(provider_id, adapter_id, status, body) {
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
    let message = v
        .pointer("/choices/0/message")
        .cloned()
        .unwrap_or(Value::Null);
    let text = message
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
    let mut tool_calls = Vec::new();
    if let Some(arr) = message.get("tool_calls").and_then(|t| t.as_array()) {
        for tc in arr {
            let id = tc
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string();
            let name = tc
                .pointer("/function/name")
                .and_then(|n| n.as_str())
                .map(from_provider_tool_name)
                .unwrap_or_default();
            let args_raw = tc
                .pointer("/function/arguments")
                .and_then(|a| a.as_str())
                .unwrap_or("{}");
            if !name.is_empty() {
                tool_calls.push(ToolCall {
                    id: if id.is_empty() {
                        Uuid::new_v4().to_string()
                    } else {
                        id
                    },
                    name,
                    arguments: parse_args_json(args_raw),
                });
            }
        }
    }
    if text.is_empty() && tool_calls.is_empty() {
        return Err(Box::new(AdapterError {
            category: "model_output".into(),
            message: "empty assistant content".into(),
            provider_id: provider_id.into(),
            adapter_id: adapter_id.into(),
            retryable: false,
            status: Some(status),
            detail: Some(body.chars().take(200).collect()),
        }));
    }
    let model = v
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(fallback_model)
        .to_string();
    Ok(AssistantTurn {
        text,
        tool_calls,
        model,
        provider_id: provider_id.into(),
        adapter_id: adapter_id.into(),
        protocol: Protocol::OpenaiChatCompletions.as_str().into(),
        finish_reason: v
            .pointer("/choices/0/finish_reason")
            .and_then(|f| f.as_str())
            .map(str::to_string),
        usage: v.get("usage").cloned(),
        partial: false,
    })
}

/// Parse Anthropic Messages API response.
pub fn parse_anthropic_turn(
    provider_id: &str,
    adapter_id: &str,
    status: u16,
    body: &str,
    fallback_model: &str,
) -> Result<AssistantTurn, Box<AdapterError>> {
    if let Some(err) = http_status_error_pub(provider_id, adapter_id, status, body) {
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
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    if let Some(arr) = v.get("content").and_then(|c| c.as_array()) {
        for part in arr {
            match part.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                        if !text.is_empty() {
                            text.push('\n');
                        }
                        text.push_str(t);
                    }
                }
                Some("tool_use") => {
                    let id = part
                        .get("id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = part
                        .get("name")
                        .and_then(|n| n.as_str())
                        .map(from_provider_tool_name)
                        .unwrap_or_default();
                    let input = part.get("input").cloned().unwrap_or(json!({}));
                    if !name.is_empty() {
                        tool_calls.push(ToolCall {
                            id: if id.is_empty() {
                                Uuid::new_v4().to_string()
                            } else {
                                id
                            },
                            name,
                            arguments: input,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    if text.is_empty() && tool_calls.is_empty() {
        return Err(Box::new(AdapterError {
            category: "model_output".into(),
            message: "empty assistant content".into(),
            provider_id: provider_id.into(),
            adapter_id: adapter_id.into(),
            retryable: false,
            status: Some(status),
            detail: Some(body.chars().take(200).collect()),
        }));
    }
    let model = v
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(fallback_model)
        .to_string();
    Ok(AssistantTurn {
        text,
        tool_calls,
        model,
        provider_id: provider_id.into(),
        adapter_id: adapter_id.into(),
        protocol: Protocol::AnthropicMessages.as_str().into(),
        finish_reason: v
            .get("stop_reason")
            .and_then(|s| s.as_str())
            .map(str::to_string),
        usage: v.get("usage").cloned(),
        partial: false,
    })
}

/// Parse OpenAI Responses API into an assistant turn (text-focused; tools when present).
pub fn parse_responses_turn(
    provider_id: &str,
    adapter_id: &str,
    status: u16,
    body: &str,
    fallback_model: &str,
) -> Result<AssistantTurn, Box<AdapterError>> {
    if let Some(err) = http_status_error_pub(provider_id, adapter_id, status, body) {
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
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    if let Some(t) = v.get("output_text").and_then(|x| x.as_str()) {
        text = t.to_string();
    }
    if let Some(arr) = v.get("output").and_then(|o| o.as_array()) {
        for item in arr {
            if let Some(content) = item.get("content").and_then(|c| c.as_array()) {
                for part in content {
                    if part.get("type").and_then(|t| t.as_str()) == Some("output_text") {
                        if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                            if text.is_empty() {
                                text = t.to_string();
                            }
                        }
                    }
                }
            }
            if item.get("type").and_then(|t| t.as_str()) == Some("function_call") {
                let name = item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(from_provider_tool_name)
                    .unwrap_or_default();
                let id = item
                    .get("call_id")
                    .or_else(|| item.get("id"))
                    .and_then(|i| i.as_str())
                    .unwrap_or("")
                    .to_string();
                let args_raw = item
                    .get("arguments")
                    .and_then(|a| a.as_str())
                    .unwrap_or("{}");
                if !name.is_empty() {
                    tool_calls.push(ToolCall {
                        id: if id.is_empty() {
                            Uuid::new_v4().to_string()
                        } else {
                            id
                        },
                        name,
                        arguments: parse_args_json(args_raw),
                    });
                }
            }
        }
    }
    if text.is_empty() && tool_calls.is_empty() {
        return Err(Box::new(AdapterError {
            category: "model_output".into(),
            message: "empty responses output".into(),
            provider_id: provider_id.into(),
            adapter_id: adapter_id.into(),
            retryable: false,
            status: Some(status),
            detail: Some(body.chars().take(200).collect()),
        }));
    }
    let model = v
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or(fallback_model)
        .to_string();
    Ok(AssistantTurn {
        text,
        tool_calls,
        model,
        provider_id: provider_id.into(),
        adapter_id: adapter_id.into(),
        protocol: Protocol::OpenaiResponses.as_str().into(),
        finish_reason: v
            .get("status")
            .and_then(|s| s.as_str())
            .map(str::to_string),
        usage: v.get("usage").cloned(),
        partial: false,
    })
}

pub fn parse_chat_turn(
    adapter: &dyn ProviderAdapter,
    status: u16,
    body: &str,
    model: &str,
) -> Result<AssistantTurn, Box<AdapterError>> {
    match adapter.protocol() {
        Protocol::OpenaiChatCompletions => parse_openai_chat_turn(
            adapter.id(),
            &format!("{}.v1", adapter.id()),
            status,
            body,
            model,
        ),
        Protocol::OpenaiResponses => parse_responses_turn(
            adapter.id(),
            &format!("{}.v1", adapter.id()),
            status,
            body,
            model,
        ),
        Protocol::AnthropicMessages => parse_anthropic_turn(
            adapter.id(),
            &format!("{}.v1", adapter.id()),
            status,
            body,
            model,
        ),
    }
}

/// Execute one model round via transport.
#[allow(clippy::too_many_arguments)]
pub fn complete_chat_turn(
    adapter: &dyn ProviderAdapter,
    model: &str,
    api_key: &str,
    messages: &[ChatMessage],
    tools: &[ToolDefinition],
    transport: &dyn crate::providers::transport::HttpTransport,
    base_url_override: Option<&str>,
    reasoning: Option<&ReasoningConfig>,
) -> Result<AssistantTurn, Box<AdapterError>> {
    let req = build_chat_request(
        adapter,
        model,
        api_key,
        messages,
        tools,
        base_url_override,
        reasoning,
    );
    debug_assert!(!req.url.contains(api_key));
    match transport.execute(&req) {
        Ok((status, body)) => parse_chat_turn(adapter, status, &body, model),
        Err(e) => {
            let cancelled = e.contains("cancel");
            Err(Box::new(AdapterError {
                category: if cancelled {
                    "cancellation".into()
                } else {
                    "network".into()
                },
                message: e,
                provider_id: adapter.id().into(),
                adapter_id: adapter.id().into(),
                retryable: !cancelled,
                status: None,
                detail: None,
            }))
        }
    }
}

/// Build system + user messages from compiled context text blocks.
pub fn messages_from_context(
    system_prompt: &str,
    user_turn: &str,
    prior: &[ChatMessage],
) -> Vec<ChatMessage> {
    let mut msgs = Vec::new();
    if !system_prompt.is_empty() {
        msgs.push(ChatMessage {
            role: "system".into(),
            content: Some(system_prompt.to_string()),
            tool_calls: vec![],
            tool_call_id: None,
            name: None,
        });
    }
    msgs.extend(prior.iter().cloned());
    // Ensure the latest user turn is present if prior does not already end with it.
    let need_user = prior
        .last()
        .map(|m| m.role != "user" || m.content.as_deref() != Some(user_turn))
        .unwrap_or(true);
    if need_user {
        msgs.push(ChatMessage {
            role: "user".into(),
            content: Some(user_turn.to_string()),
            tool_calls: vec![],
            tool_call_id: None,
            name: None,
        });
    }
    msgs
}

/// Host system prompt for agent turns (mode + tool protocol + skills index).
pub fn host_system_prompt(
    mode: &str,
    project_hint: &str,
    skills: &[crate::tools::skills::SkillMeta],
) -> String {
    let shell = crate::tools::shell::resolve_shell_host();
    let skills_block = if skills.is_empty() {
        "Skills: none discovered in this project.".to_string()
    } else {
        let mut lines = vec![
            "Skills (metadata only — call `skill` with path to load a body):".to_string(),
        ];
        for s in skills {
            let desc = if s.description.is_empty() {
                String::new()
            } else {
                format!(" — {}", s.description.chars().take(120).collect::<String>())
            };
            lines.push(format!("- {} ({}){} [{}]", s.name, s.path, desc, s.origin));
        }
        lines.join("\n")
    };
    format!(
        "You are LoopCode, a local AI coding agent.\n\
Mode: {mode}. Respect mode boundaries: Ask/Plan are read-only; Build may edit files; Debug is investigation-oriented.\n\
\n\
Tool protocol:\n\
- Project instructions already appear below as [agents.md] when present — use them; do not re-read AGENTS.md/README/package.json just to explain what the project is.\n\
- Prefer glob / grep / read over guessing file contents when you need details beyond those instructions.\n\
- For small edits use edit (exact string replace). For whole-file rewrites use patch or write with expectedHash from read. Do not use exec for bulk search-and-replace across the tree.\n\
- To delete a file use patch with delete:true and the file's expectedHash from read — never delete files via exec/shell.\n\
- grep is paginated (offset/nextOffset). If capped, page again instead of re-scanning blindly.\n\
- context.attach loads a file body into the tool result for this turn — use it when you need a file in context.\n\
- exec and webfetch (and MCP tools) may pause for user approval. The host executes the approved call — do not treat approval pauses as failures, and do not switch tools just because an action needed approval.\n\
- exec runs via host shell `{shell_kind}` ({shell_program}). Use that shell's syntax. Avoid unbounded recursive walks of node_modules/target/.git.\n\
- Call multiple independent read-only tools in one turn when possible.\n\
- If a tool actually fails (error/status deny/path_rejected/hash_conflict/nonzero exit), change approach — do not repeat the identical failing call with the same arguments.\n\
- When finished, answer clearly without unnecessary further tool calls.\n\
\n\
{skills_block}\n\
\n\
Project: {project_hint}",
        shell_kind = shell.kind,
        shell_program = shell.program,
    )
}

/// Injected before the final allowed tool round so the model stops cleanly.
pub const ROUND_BUDGET_NUDGE: &str = "\
You have reached the tool-round budget for this turn. \
Do not call any more tools. Provide your best final answer now based on what you already know. \
If work remains, summarize what is left so the user can continue in a follow-up message.";

/// Returned to the model when identical tool calls repeat (doom loop).
pub const DOOM_LOOP_MESSAGE: &str = "\
Repeated identical tool call detected and blocked. \
Change arguments or switch tools; do not retry the same call.";

/// Emit SSE-style OpenAI chunks as a single final body helper for fixtures.
pub fn openai_assistant_text_body(text: &str, model: &str) -> String {
    json!({
        "id": "chatcmpl-fixture",
        "object": "chat.completion",
        "model": model,
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": text },
            "finish_reason": "stop"
        }],
        "usage": { "prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30 }
    })
    .to_string()
}

pub fn openai_assistant_tool_body(
    tool_name: &str,
    arguments: &Value,
    model: &str,
    call_id: &str,
) -> String {
    json!({
        "id": "chatcmpl-fixture-tools",
        "object": "chat.completion",
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": call_id,
                    "type": "function",
                    "function": {
                        "name": tool_name,
                        "arguments": arguments.to_string()
                    }
                }]
            },
            "finish_reason": "tool_calls"
        }],
        "usage": { "prompt_tokens": 12, "completion_tokens": 8, "total_tokens": 20 }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::first_party::OpenAiAdapter;

    #[test]
    fn parse_text_and_tool_calls() {
        let body = openai_assistant_tool_body(
            "glob",
            &json!({"pattern": "**/*"}),
            "gpt-test",
            "call_1",
        );
        let turn = parse_openai_chat_turn("openai", "openai.v1", 200, &body, "gpt-test").unwrap();
        assert!(turn.text.is_empty());
        assert_eq!(turn.tool_calls.len(), 1);
        assert_eq!(turn.tool_calls[0].name, "glob");
    }

    #[test]
    fn parse_wire_tool_name_maps_to_internal() {
        let body = openai_assistant_tool_body(
            "plan_write",
            &json!({"title": "Plan", "body": "steps"}),
            "gpt-test",
            "call_1",
        );
        let turn = parse_openai_chat_turn("openai", "openai.v1", 200, &body, "gpt-test").unwrap();
        assert_eq!(turn.tool_calls[0].name, "plan_write");
    }

    #[test]
    fn provider_tool_names_are_openai_safe() {
        assert_eq!(to_provider_tool_name("glob"), "glob");
        assert_eq!(to_provider_tool_name("patch"), "patch");
        assert_eq!(from_provider_tool_name("glob"), "glob");
        assert_eq!(from_provider_tool_name("context_attach"), "context.attach");
        // Already-internal names stay internal
        assert_eq!(from_provider_tool_name("exec"), "exec");
    }

    #[test]
    fn build_request_includes_tools() {
        let adapter = OpenAiAdapter;
        let tools = crate::tools::catalog::builtin_catalog();
        let msgs = messages_from_context("sys", "hi", &[]);
        let req = build_chat_request(&adapter, "gpt-4o-mini", "sk-test", &msgs, &tools, None, None);
        assert!(req.url.ends_with("/chat/completions"));
        assert!(req.body.get("tools").is_some());
        // No reasoning selection → legacy body with temperature, no effort.
        assert!(req.body.get("temperature").is_some());
        assert!(req.body.get("reasoning_effort").is_none());

        let cfg = ReasoningConfig { enabled: true, effort: Some("high".into()) };
        let req_effort = build_chat_request(
            &adapter, "gpt-5.4", "sk-test", &msgs, &tools, None, Some(&cfg),
        );
        assert_eq!(req_effort.body.get("reasoning_effort"), Some(&json!("high")));
        assert!(req_effort.body.get("temperature").is_none());

        let anthropic = crate::providers::first_party::AnthropicAdapter;
        let req_think = build_chat_request(
            &anthropic, "claude-sonnet-5", "sk-a", &msgs, &tools, None, Some(&cfg),
        );
        assert_eq!(req_think.body.pointer("/thinking/type"), Some(&json!("adaptive")));
        assert_eq!(
            req_think.body.pointer("/output_config/effort"),
            Some(&json!("high"))
        );
        let off = ReasoningConfig { enabled: false, effort: None };
        let req_off = build_chat_request(
            &anthropic, "claude-sonnet-5", "sk-a", &msgs, &tools, None, Some(&off),
        );
        assert!(req_off.body.get("thinking").is_none());
        assert!(!req.url.contains("sk-test"));
        // Provider wire names must not contain '.' (DeepSeek Console rejects them).
        let tools_arr = req.body.get("tools").and_then(|t| t.as_array()).unwrap();
        for t in tools_arr {
            let name = t
                .pointer("/function/name")
                .and_then(|n| n.as_str())
                .unwrap_or("");
            assert!(
                !name.contains('.'),
                "tool wire name must be OpenAI-safe, got {name}"
            );
            assert!(name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
        }
        let names: Vec<&str> = tools_arr
            .iter()
            .filter_map(|t| t.pointer("/function/name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"glob"));
        assert!(names.contains(&"exec"));
    }
}
