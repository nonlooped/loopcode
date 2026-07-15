//! Fixed approval matrix (no YOLO / no user-authored rules).

use crate::runtime::mode::{Mode, ToolEffect};
use crate::security::path::{PathClass, PathClassification};
use serde::{Deserialize, Serialize};

/// Snapshotted on every run — fixed policy identity.
pub const SECURITY_POLICY_VERSION: &str = "security-v1";

/// Action classes in the fixed host approval matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionClass {
    WorkspaceRead,
    WorkspaceWrite,
    ProtectedPathRead,
    ProtectedPathWrite,
    Shell,
    Network,
    OutsideWorkspaceFs,
    Secrets,
    Mcp,
    SkillScript,
    PackageInstall,
    Privilege,
    Destructive,
    Publish,
}

impl ActionClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorkspaceRead => "workspace_read",
            Self::WorkspaceWrite => "workspace_write",
            Self::ProtectedPathRead => "protected_path_read",
            Self::ProtectedPathWrite => "protected_path_write",
            Self::Shell => "shell",
            Self::Network => "network",
            Self::OutsideWorkspaceFs => "outside_workspace_fs",
            Self::Secrets => "secrets",
            Self::Mcp => "mcp",
            Self::SkillScript => "skill_script",
            Self::PackageInstall => "package_install",
            Self::Privilege => "privilege",
            Self::Destructive => "destructive",
            Self::Publish => "publish",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "workspace_read" => Some(Self::WorkspaceRead),
            "workspace_write" => Some(Self::WorkspaceWrite),
            "protected_path_read" => Some(Self::ProtectedPathRead),
            "protected_path_write" => Some(Self::ProtectedPathWrite),
            "shell" => Some(Self::Shell),
            "network" => Some(Self::Network),
            "outside_workspace_fs" => Some(Self::OutsideWorkspaceFs),
            "secrets" => Some(Self::Secrets),
            "mcp" => Some(Self::Mcp),
            "skill_script" => Some(Self::SkillScript),
            "package_install" => Some(Self::PackageInstall),
            "privilege" => Some(Self::Privilege),
            "destructive" => Some(Self::Destructive),
            "publish" => Some(Self::Publish),
            _ => None,
        }
    }

    /// Publish and privilege grants are once-only (no this-chat).
    pub fn once_only_grant(self) -> bool {
        matches!(self, Self::Publish | Self::Privilege)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// May proceed without a user prompt under the fixed matrix.
    Allow,
    /// Must obtain an explicit grant before execution.
    RequireApproval,
    /// Hard deny (e.g. mode forbids, or unknown class).
    Deny,
}

impl ApprovalDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::RequireApproval => "require_approval",
            Self::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantScope {
    Deny,
    AllowOnce,
    /// Matching action for this chat (not allowed for publish/privilege).
    AllowMatchingForChat,
}

/// Map a tool name (+ optional path class + effect) to an action class.
pub fn action_class_for_tool(
    tool_name: &str,
    effect: ToolEffect,
    path: Option<&PathClassification>,
) -> ActionClass {
    if tool_name.starts_with("mcp_") {
        return ActionClass::Mcp;
    }
    match tool_name {
        "exec" => ActionClass::Shell,
        "webfetch" => ActionClass::Network,
        "secrets.reveal" | "secrets.get" => ActionClass::Secrets,
        "mcp.enable" | "mcp.list_tools" => ActionClass::Mcp,
        "package.install" => ActionClass::PackageInstall,
        "privilege.escalate" => ActionClass::Privilege,
        "fs.destroy" => ActionClass::Destructive,
        "publish.release" => ActionClass::Publish,
        "glob" | "read" | "grep" | "context.attach" | "skill" | "plan_write" | "plan_update" => {
            if let Some(p) = path {
                match p.class {
                    PathClass::OutsideWorkspace | PathClass::Rejected => {
                        return ActionClass::OutsideWorkspaceFs;
                    }
                    PathClass::ProtectedInWorkspace => return ActionClass::ProtectedPathRead,
                    PathClass::InWorkspace => {}
                }
            }
            ActionClass::WorkspaceRead
        }
        "write" | "edit" | "patch" => {
            if let Some(p) = path {
                match p.class {
                    PathClass::OutsideWorkspace => ActionClass::OutsideWorkspaceFs,
                    PathClass::ProtectedInWorkspace => ActionClass::ProtectedPathWrite,
                    PathClass::InWorkspace => ActionClass::WorkspaceWrite,
                    PathClass::Rejected => ActionClass::OutsideWorkspaceFs,
                }
            } else if matches!(effect, ToolEffect::Mutating) {
                ActionClass::WorkspaceWrite
            } else {
                ActionClass::WorkspaceRead
            }
        }
        _ => {
            // Unknown tools: treat mutating as write, else read; host still mode-gates.
            if matches!(effect, ToolEffect::Mutating) {
                ActionClass::WorkspaceWrite
            } else {
                ActionClass::WorkspaceRead
            }
        }
    }
}

/// Fixed approval matrix decision (no mode applied — call after mode allows).
///
/// This is the shipped policy function tests must drive.
pub fn approval_decision(class: ActionClass) -> ApprovalDecision {
    match class {
        ActionClass::WorkspaceRead | ActionClass::WorkspaceWrite => ApprovalDecision::Allow,
        ActionClass::ProtectedPathRead
        | ActionClass::ProtectedPathWrite
        | ActionClass::Shell
        | ActionClass::Network
        | ActionClass::OutsideWorkspaceFs
        | ActionClass::Secrets
        | ActionClass::Mcp
        | ActionClass::SkillScript
        | ActionClass::PackageInstall
        | ActionClass::Privilege
        | ActionClass::Destructive
        | ActionClass::Publish => ApprovalDecision::RequireApproval,
    }
}

/// Combined host decision: mode mutation + fixed matrix.
pub fn security_gate_decision(
    mode: Mode,
    effect: ToolEffect,
    class: ActionClass,
) -> ApprovalDecision {
    // Ask/Plan cannot write (mutating) — hard deny at security layer too.
    if matches!(effect, ToolEffect::Mutating)
        && !matches!(mode, Mode::Build)
    {
        return ApprovalDecision::Deny;
    }
    // Debug cannot source-mutate
    if matches!(effect, ToolEffect::Mutating) && matches!(mode, Mode::Debug) {
        return ApprovalDecision::Deny;
    }
    approval_decision(class)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_and_network_always_prompt() {
        assert_eq!(
            approval_decision(ActionClass::Shell),
            ApprovalDecision::RequireApproval
        );
        assert_eq!(
            approval_decision(ActionClass::Network),
            ApprovalDecision::RequireApproval
        );
        assert_eq!(
            approval_decision(ActionClass::Secrets),
            ApprovalDecision::RequireApproval
        );
        assert_eq!(
            approval_decision(ActionClass::OutsideWorkspaceFs),
            ApprovalDecision::RequireApproval
        );
    }

    #[test]
    fn workspace_read_and_write_free() {
        assert_eq!(
            approval_decision(ActionClass::WorkspaceRead),
            ApprovalDecision::Allow
        );
        assert_eq!(
            approval_decision(ActionClass::WorkspaceWrite),
            ApprovalDecision::Allow
        );
    }

    #[test]
    fn ask_mutating_denied() {
        assert_eq!(
            security_gate_decision(Mode::Ask, ToolEffect::Mutating, ActionClass::WorkspaceWrite),
            ApprovalDecision::Deny
        );
        assert_eq!(
            security_gate_decision(Mode::Plan, ToolEffect::Mutating, ActionClass::WorkspaceWrite),
            ApprovalDecision::Deny
        );
        assert_eq!(
            security_gate_decision(Mode::Build, ToolEffect::Mutating, ActionClass::WorkspaceWrite),
            ApprovalDecision::Allow
        );
    }

    #[test]
    fn mcp_prefixed_tools_require_approval() {
        assert_eq!(
            action_class_for_tool("mcp_server_search", ToolEffect::Mutating, None),
            ActionClass::Mcp
        );
    }
}
