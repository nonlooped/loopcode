//! Semantic event kinds emitted by the agent runtime.

/// Stable event kind strings stored in the append-only event stream.
pub mod kinds {
    pub const RUN_QUEUED: &str = "run.lifecycle.queued";
    pub const RUN_PREPARING: &str = "run.lifecycle.preparing";
    pub const RUN_MODEL_ACTIVE: &str = "run.lifecycle.model_active";
    pub const RUN_TOOL_ACTIVE: &str = "run.lifecycle.tool_active";
    pub const RUN_APPROVAL_WAITING: &str = "run.lifecycle.approval_waiting";
    pub const RUN_CANCELLING: &str = "run.lifecycle.cancelling";
    pub const RUN_SUSPENDED: &str = "run.lifecycle.suspended";
    pub const RUN_COMPLETED: &str = "run.lifecycle.completed";
    pub const RUN_FAILED: &str = "run.lifecycle.failed";
    pub const RUN_CANCELLED: &str = "run.lifecycle.cancelled";

    pub const TOOL_PROPOSAL: &str = "tool.proposal";
    pub const TOOL_RESULT: &str = "tool.result";
    pub const TOOL_REJECTED: &str = "tool.rejected";

    pub const ERROR: &str = "run.error";
    pub const CANCEL_REQUESTED: &str = "run.cancel_requested";
}

/// Map a durable run status string to a lifecycle event kind.
pub fn lifecycle_kind_for_status(status: &str) -> Option<&'static str> {
    match status {
        "queued" | "pending" => Some(kinds::RUN_QUEUED),
        "preparing" => Some(kinds::RUN_PREPARING),
        "model_active" | "running" => Some(kinds::RUN_MODEL_ACTIVE),
        "tool_active" => Some(kinds::RUN_TOOL_ACTIVE),
        "approval_waiting" => Some(kinds::RUN_APPROVAL_WAITING),
        "cancelling" => Some(kinds::RUN_CANCELLING),
        "suspended" => Some(kinds::RUN_SUSPENDED),
        "completed" => Some(kinds::RUN_COMPLETED),
        "failed" => Some(kinds::RUN_FAILED),
        "cancelled" => Some(kinds::RUN_CANCELLED),
        _ => None,
    }
}
