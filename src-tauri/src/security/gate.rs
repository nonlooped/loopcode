//! Security gate for tool proposals: path boundary + fixed approval matrix + grants.

use crate::runtime::mode::{Mode, ToolEffect};
use crate::runtime::tools::ToolProposal;
use crate::security::approval::{
    action_class_for_tool, approval_decision, security_gate_decision, ActionClass,
    ApprovalDecision, GrantScope, SECURITY_POLICY_VERSION,
};
use crate::security::path::{classify_path, PathClass, PathClassification};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Result of evaluating the security gate before mock execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateResult {
    pub decision: ApprovalDecision,
    pub action_class: ActionClass,
    pub policy_version: String,
    pub path: Option<PathClassification>,
    pub reason: String,
}

/// Extract optional path argument from a tool proposal.
pub fn proposal_path(proposal: &ToolProposal) -> Option<String> {
    proposal
        .arguments
        .get("path")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// Classify path when present under project root.
pub fn classify_proposal_path(
    project_root: Option<&Path>,
    proposal: &ToolProposal,
) -> Option<PathClassification> {
    let path = proposal_path(proposal)?;
    let root = project_root?;
    Some(classify_path(root, &path))
}

/// Evaluate path + fixed matrix + mode (without chat grants).
pub fn evaluate_gate(
    mode: Mode,
    effect: ToolEffect,
    tool_name: &str,
    path: Option<&PathClassification>,
) -> GateResult {
    // Hard reject invalid paths
    if let Some(p) = path {
        if p.class == PathClass::Rejected {
            return GateResult {
                decision: ApprovalDecision::Deny,
                action_class: ActionClass::OutsideWorkspaceFs,
                policy_version: SECURITY_POLICY_VERSION.into(),
                path: Some(p.clone()),
                reason: p.reason.clone(),
            };
        }
    }

    let class = action_class_for_tool(tool_name, effect, path);
    let decision = security_gate_decision(mode, effect, class);
    let reason = match decision {
        ApprovalDecision::Allow => format!("allowed by fixed policy {}", SECURITY_POLICY_VERSION),
        ApprovalDecision::RequireApproval => {
            format!("action_class={} requires approval", class.as_str())
        }
        ApprovalDecision::Deny => format!(
            "denied by mode/security (mode={} class={})",
            mode.as_str(),
            class.as_str()
        ),
    };
    GateResult {
        decision,
        action_class: class,
        policy_version: SECURITY_POLICY_VERSION.into(),
        path: path.cloned(),
        reason,
    }
}

/// In-memory grants: once-tokens and chat-scoped allows.
#[derive(Debug, Default)]
pub struct GrantTable {
    /// One-shot tokens: (chat_id, action_class) counts remaining.
    once: HashMap<(String, String), u32>,
    /// Matching-for-this-chat: set of (chat_id, action_class).
    chat: HashSet<(String, String)>,
}

impl GrantTable {
    pub fn grant(&mut self, chat_id: &str, class: ActionClass, scope: GrantScope) {
        match scope {
            GrantScope::Deny => {
                self.once.remove(&(chat_id.into(), class.as_str().into()));
                self.chat.remove(&(chat_id.into(), class.as_str().into()));
            }
            GrantScope::AllowOnce => {
                *self
                    .once
                    .entry((chat_id.into(), class.as_str().into()))
                    .or_insert(0) += 1;
            }
            GrantScope::AllowMatchingForChat => {
                if !class.once_only_grant() {
                    self.chat.insert((chat_id.into(), class.as_str().into()));
                }
            }
        }
    }

    /// Consume a grant if present. Returns true if execution may proceed.
    pub fn try_consume(&mut self, chat_id: &str, class: ActionClass) -> bool {
        let key = (chat_id.to_string(), class.as_str().to_string());
        if self.chat.contains(&key) {
            return true;
        }
        if let Some(n) = self.once.get_mut(&key) {
            if *n > 0 {
                *n -= 1;
                return true;
            }
        }
        false
    }

    pub fn has_chat_grant(&self, chat_id: &str, class: ActionClass) -> bool {
        self.chat
            .contains(&(chat_id.to_string(), class.as_str().to_string()))
    }
}

/// Apply grants to a RequireApproval decision.
pub fn apply_grants(
    gate: &GateResult,
    grants: &mut GrantTable,
    chat_id: &str,
) -> ApprovalDecision {
    match gate.decision {
        ApprovalDecision::Allow | ApprovalDecision::Deny => gate.decision,
        ApprovalDecision::RequireApproval => {
            if grants.try_consume(chat_id, gate.action_class) {
                ApprovalDecision::Allow
            } else {
                ApprovalDecision::RequireApproval
            }
        }
    }
}

/// Pure helper used by tests for matrix cells.
pub fn matrix_decision_for_class(class: ActionClass) -> ApprovalDecision {
    approval_decision(class)
}
