//! Stable LoopCode error taxonomy (runtime contract).

use serde::{Deserialize, Serialize};

/// Origin of a failure for diagnostics and UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorOrigin {
    Runtime,
    ModePolicy,
    Tool,
    Provider,
    Persistence,
    User,
    Internal,
}

/// Stage within the run when the error occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorStage {
    Queued,
    Preparing,
    Model,
    ToolValidation,
    Approval,
    ToolExecution,
    Cancellation,
    Reconciliation,
    Terminal,
}

/// Whether automatic retry is permitted for this failure class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Retryability {
    Never,
    /// Transport/rate-limit before semantic output (may auto-retry).
    TransportOnly,
    /// User or recovery loop may retry as a new attempt.
    UserInitiated,
}

/// Certainty about side effects after a failure or cancel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectCertainty {
    None,
    Completed,
    Unknown,
}

/// Stable error category for the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    Cancellation,
    PolicyDenial,
    UnsupportedCapability,
    ToolValidation,
    ToolPermission,
    ToolTimeout,
    ToolExit,
    SchemaValidation,
    ContextLimit,
    RateLimit,
    Network,
    Timeout,
    Persistence,
    Runtime,
    Internal,
}

impl ErrorCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cancellation => "cancellation",
            Self::PolicyDenial => "policy_denial",
            Self::UnsupportedCapability => "unsupported_capability",
            Self::ToolValidation => "tool_validation",
            Self::ToolPermission => "tool_permission",
            Self::ToolTimeout => "tool_timeout",
            Self::ToolExit => "tool_exit",
            Self::SchemaValidation => "schema_validation",
            Self::ContextLimit => "context_limit",
            Self::RateLimit => "rate_limit",
            Self::Network => "network",
            Self::Timeout => "timeout",
            Self::Persistence => "persistence",
            Self::Runtime => "runtime",
            Self::Internal => "internal",
        }
    }

    pub fn default_retryability(self) -> Retryability {
        match self {
            Self::RateLimit | Self::Network | Self::Timeout => Retryability::TransportOnly,
            Self::Cancellation | Self::PolicyDenial | Self::UnsupportedCapability
            | Self::ToolPermission | Self::SchemaValidation => Retryability::Never,
            _ => Retryability::UserInitiated,
        }
    }
}

/// Structured error attached to failed/cancelled/suspended paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeError {
    pub category: ErrorCategory,
    pub origin: ErrorOrigin,
    pub stage: ErrorStage,
    pub retryability: Retryability,
    pub side_effect_certainty: SideEffectCertainty,
    pub message: String,
    /// Optional redacted provider/tool detail (never secrets).
    pub detail: Option<String>,
}

impl RuntimeError {
    pub fn new(
        category: ErrorCategory,
        origin: ErrorOrigin,
        stage: ErrorStage,
        side_effect_certainty: SideEffectCertainty,
        message: impl Into<String>,
    ) -> Self {
        Self {
            retryability: category.default_retryability(),
            category,
            origin,
            stage,
            side_effect_certainty,
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn policy_denial(stage: ErrorStage, message: impl Into<String>) -> Self {
        Self::new(
            ErrorCategory::PolicyDenial,
            ErrorOrigin::ModePolicy,
            stage,
            SideEffectCertainty::None,
            message,
        )
    }

    pub fn cancellation(certainty: SideEffectCertainty, message: impl Into<String>) -> Self {
        Self::new(
            ErrorCategory::Cancellation,
            ErrorOrigin::User,
            ErrorStage::Cancellation,
            certainty,
            message,
        )
    }

    pub fn tool_validation(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCategory::ToolValidation,
            ErrorOrigin::Tool,
            ErrorStage::ToolValidation,
            SideEffectCertainty::None,
            message,
        )
    }
}
