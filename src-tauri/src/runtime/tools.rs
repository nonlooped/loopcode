//! Tool registry and proposal validation.

use crate::runtime::mode::{
    mode_allows_tool_effect, mode_rejection_reason, Mode, ToolEffect,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

/// Canonical tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: String,
    pub version: String,
    pub description: String,
    pub effect: ToolEffect,
    /// When true, tool may run concurrently with other read-only tools.
    pub concurrency_safe: bool,
    pub idempotent: bool,
    /// When set, this catalog entry is an MCP tool (merged into the agent tool list).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_server_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_tool_name: Option<String>,
    /// Optional JSON Schema from MCP `tools/list` (used for provider wire format).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,
}

impl ToolDefinition {
    pub fn new(
        name: impl Into<String>,
        effect: ToolEffect,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: "1".into(),
            description: description.into(),
            concurrency_safe: matches!(effect, ToolEffect::ReadOnly),
            idempotent: matches!(effect, ToolEffect::ReadOnly | ToolEffect::Diagnostic),
            effect,
            mcp_server_id: None,
            mcp_tool_name: None,
            input_schema: None,
        }
    }

    pub fn mcp(
        catalog_name: impl Into<String>,
        server_id: impl Into<String>,
        tool_name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Option<Value>,
    ) -> Self {
        Self {
            name: catalog_name.into(),
            version: "1".into(),
            description: description.into(),
            concurrency_safe: false,
            idempotent: false,
            effect: ToolEffect::Mutating,
            mcp_server_id: Some(server_id.into()),
            mcp_tool_name: Some(tool_name.into()),
            input_schema,
        }
    }

    pub fn is_mcp(&self) -> bool {
        self.mcp_server_id.is_some()
    }
}

/// Model-proposed tool call before host validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolProposal {
    pub call_id: String,
    pub tool_name: String,
    pub arguments: Value,
}

impl ToolProposal {
    pub fn new(tool_name: impl Into<String>, arguments: Value) -> Self {
        Self {
            call_id: Uuid::new_v4().to_string(),
            tool_name: tool_name.into(),
            arguments,
        }
    }
}

/// Outcome of validating/executing a proposal under mode policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutcomeKind {
    Executed,
    /// Forbidden by mode or unknown tool — structured result, not a fatal crash.
    Rejected,
    /// Gate requires an explicit user grant; run is paused (not a model-facing failure).
    PendingApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub call_id: String,
    pub tool_name: String,
    pub kind: ToolOutcomeKind,
    pub executed: bool,
    pub content: Value,
    pub error_message: Option<String>,
}

/// In-process mock tool runner with a fixed registry.
///
/// Cancel is **not** tracked here — the engine scopes cancel to `run_id` so
/// concurrent runs in other chats are not poisoned.
#[derive(Debug, Clone)]
pub struct MockToolRunner {
    tools: HashMap<String, ToolDefinition>,
    /// Call IDs that have been executed (for cancel/scheduling tests).
    executed_call_ids: Vec<String>,
}

impl Default for MockToolRunner {
    fn default() -> Self {
        Self::with_default_tools()
    }
}

impl MockToolRunner {
    pub fn with_default_tools() -> Self {
        let mut tools = HashMap::new();
        // Built-in catalog plus security-class stubs.
        for d in crate::tools::catalog::builtin_catalog() {
            tools.insert(d.name.clone(), d);
        }
        for d in [
            ToolDefinition::new(
                "secrets.reveal",
                ToolEffect::ReadOnly,
                "Reveal secret to tool env (mock)",
            ),
            ToolDefinition::new(
                "package.install",
                ToolEffect::Mutating,
                "Package install (mock)",
            ),
            ToolDefinition::new(
                "publish.release",
                ToolEffect::Mutating,
                "Publish-class action (mock)",
            ),
        ] {
            tools.insert(d.name.clone(), d);
        }
        Self {
            tools,
            executed_call_ids: Vec::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    pub fn executed_call_ids(&self) -> &[String] {
        &self.executed_call_ids
    }

    /// Validate proposal against registry + mode; execute if allowed.
    ///
    /// Forbidden proposals return a structured reject (`executed: false`), never panic.
    /// Run-level cancel is enforced by [`crate::runtime::engine::AgentRuntime`], not here.
    pub fn handle_proposal(&mut self, mode: Mode, proposal: &ToolProposal) -> ToolResult {
        let Some(def) = self.tools.get(&proposal.tool_name).cloned() else {
            return ToolResult {
                call_id: proposal.call_id.clone(),
                tool_name: proposal.tool_name.clone(),
                kind: ToolOutcomeKind::Rejected,
                executed: false,
                content: json!({
                    "status": "unknown_tool",
                    "tool": proposal.tool_name,
                }),
                error_message: Some(format!("unknown tool: {}", proposal.tool_name)),
            };
        };

        if !mode_allows_tool_effect(mode, def.effect) {
            let reason = mode_rejection_reason(mode, def.effect);
            return ToolResult {
                call_id: proposal.call_id.clone(),
                tool_name: proposal.tool_name.clone(),
                kind: ToolOutcomeKind::Rejected,
                executed: false,
                content: json!({
                    "status": "mode_policy_denied",
                    "mode": mode.as_str(),
                    "effect": effect_str(def.effect),
                    "reason": reason,
                }),
                error_message: Some(reason),
            };
        }

        // Mock execution body
        self.executed_call_ids.push(proposal.call_id.clone());
        let content = match def.effect {
            ToolEffect::ReadOnly => json!({
                "status": "ok",
                "effect": "read_only",
                "echo_args": proposal.arguments,
            }),
            ToolEffect::Diagnostic => json!({
                "status": "ok",
                "effect": "diagnostic",
                "findings": [],
            }),
            ToolEffect::Mutating => json!({
                "status": "ok",
                "effect": "mutating",
                "applied": true,
                "echo_args": proposal.arguments,
            }),
        };

        ToolResult {
            call_id: proposal.call_id.clone(),
            tool_name: proposal.tool_name.clone(),
            kind: ToolOutcomeKind::Executed,
            executed: true,
            content,
            error_message: None,
        }
    }
}

fn effect_str(e: ToolEffect) -> &'static str {
    match e {
        ToolEffect::ReadOnly => "read_only",
        ToolEffect::Diagnostic => "diagnostic",
        ToolEffect::Mutating => "mutating",
    }
}

/// Pure validation helper used by tests and the engine (no execution side effects).
pub fn validate_proposal(
    mode: Mode,
    registry: &MockToolRunner,
    proposal: &ToolProposal,
) -> Result<ToolEffect, String> {
    let def = registry
        .get(&proposal.tool_name)
        .ok_or_else(|| format!("unknown tool: {}", proposal.tool_name))?;
    if !mode_allows_tool_effect(mode, def.effect) {
        return Err(mode_rejection_reason(mode, def.effect));
    }
    Ok(def.effect)
}
