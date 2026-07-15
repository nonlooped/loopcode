//! Provider/tool retry policy — bounded auto-retry for transport/rate-limit only.

use crate::runtime::error::{ErrorCategory, Retryability, RuntimeError};
use serde::{Deserialize, Serialize};

/// Maximum automatic retries for transport/rate-limit (attempts after first = this cap).
pub const MAX_AUTO_RETRIES: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryDecision {
    /// Do not auto-retry (auth/quota/config/policy/etc.).
    FailImmediately,
    /// Auto-retry allowed with backoff.
    AutoRetry { attempt: u32, backoff_ms: u64 },
    /// Exhausted auto-retry budget.
    Exhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryPlan {
    pub decision: RetryDecision,
    pub category: String,
    pub retryability: String,
    pub next_action: String,
    pub message: String,
}

/// Whether this error category may be auto-retried (transport/rate-limit only).
pub fn should_auto_retry(category: ErrorCategory, attempts_so_far: u32) -> RetryDecision {
    match category.default_retryability() {
        Retryability::TransportOnly => {
            if attempts_so_far >= MAX_AUTO_RETRIES {
                RetryDecision::Exhausted
            } else {
                let attempt = attempts_so_far + 1;
                let backoff_ms = 250u64 * 2u64.saturating_pow(attempts_so_far);
                RetryDecision::AutoRetry {
                    attempt,
                    backoff_ms,
                }
            }
        }
        Retryability::Never | Retryability::UserInitiated => RetryDecision::FailImmediately,
    }
}

/// Honor Retry-After seconds when present (rate limit); otherwise exponential backoff.
pub fn backoff_ms_for(
    category: ErrorCategory,
    attempts_so_far: u32,
    retry_after_secs: Option<u64>,
) -> u64 {
    if matches!(category, ErrorCategory::RateLimit) {
        if let Some(s) = retry_after_secs {
            return s.saturating_mul(1000);
        }
    }
    match should_auto_retry(category, attempts_so_far) {
        RetryDecision::AutoRetry { backoff_ms, .. } => backoff_ms,
        _ => 0,
    }
}

/// Build a UI-facing retry plan from a real RuntimeError.
pub fn plan_provider_retry(
    err: &RuntimeError,
    attempts_so_far: u32,
    retry_after_secs: Option<u64>,
) -> RetryPlan {
    let mut decision = should_auto_retry(err.category, attempts_so_far);
    if let RetryDecision::AutoRetry { attempt, .. } = decision {
        let backoff_ms = backoff_ms_for(err.category, attempts_so_far, retry_after_secs);
        decision = RetryDecision::AutoRetry {
            attempt,
            backoff_ms,
        };
    }
    let next_action = match decision {
        RetryDecision::AutoRetry { attempt, backoff_ms } => {
            format!("auto_retry attempt {attempt} after {backoff_ms}ms")
        }
        RetryDecision::Exhausted => "show_error_stop_auto_retry".into(),
        RetryDecision::FailImmediately => match err.category {
            ErrorCategory::RateLimit => "wait_or_change_plan".into(),
            ErrorCategory::Network | ErrorCategory::Timeout => "check_network_retry_manual".into(),
            ErrorCategory::PolicyDenial => "adjust_mode_or_approval".into(),
            ErrorCategory::SchemaValidation | ErrorCategory::ToolValidation => {
                "fix_input_or_model".into()
            }
            _ => "show_error_user_action".into(),
        },
    };
    RetryPlan {
        decision,
        category: err.category.as_str().into(),
        retryability: format!("{:?}", err.retryability).to_ascii_lowercase(),
        next_action,
        message: err.message.clone(),
    }
}

/// Tool timeout class defaults (seconds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolTimeoutClass {
    ShortRead,
    MediumEdit,
    LongShell,
}

impl ToolTimeoutClass {
    pub fn timeout_secs(self) -> u64 {
        match self {
            Self::ShortRead => 15,
            Self::MediumEdit => 60,
            Self::LongShell => 120,
        }
    }

    pub fn for_tool(tool_name: &str) -> Self {
        if tool_name.starts_with("mcp_") {
            return Self::LongShell;
        }
        match tool_name {
            "glob" | "read" | "grep" | "skill" | "context.attach" => Self::ShortRead,
            "write" | "edit" | "patch" | "plan_write" | "plan_update" => Self::MediumEdit,
            "exec" | "webfetch" => Self::LongShell,
            _ => Self::MediumEdit,
        }
    }
}

/// Classify hung/timeout outcome: must not be treated as completed.
pub fn timeout_run_status_label() -> &'static str {
    "failed" // terminal failed, not completed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::error::{ErrorOrigin, ErrorStage, SideEffectCertainty};

    #[test]
    fn transport_retries_capped_auth_never() {
        assert!(matches!(
            should_auto_retry(ErrorCategory::RateLimit, 0),
            RetryDecision::AutoRetry { attempt: 1, .. }
        ));
        assert!(matches!(
            should_auto_retry(ErrorCategory::Network, 1),
            RetryDecision::AutoRetry { attempt: 2, .. }
        ));
        assert_eq!(
            should_auto_retry(ErrorCategory::Network, 2),
            RetryDecision::Exhausted
        );
        assert_eq!(
            should_auto_retry(ErrorCategory::PolicyDenial, 0),
            RetryDecision::FailImmediately
        );
        // Auth-like: schema/policy never
        let auth = RuntimeError::new(
            ErrorCategory::SchemaValidation,
            ErrorOrigin::Provider,
            ErrorStage::Model,
            SideEffectCertainty::None,
            "invalid api key",
        );
        let plan = plan_provider_retry(&auth, 0, None);
        assert_eq!(plan.decision, RetryDecision::FailImmediately);

        let rl = RuntimeError::new(
            ErrorCategory::RateLimit,
            ErrorOrigin::Provider,
            ErrorStage::Model,
            SideEffectCertainty::None,
            "429",
        );
        let plan = plan_provider_retry(&rl, 0, Some(3));
        match plan.decision {
            RetryDecision::AutoRetry { backoff_ms, .. } => assert_eq!(backoff_ms, 3000),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn tool_timeout_classes() {
        assert_eq!(
            ToolTimeoutClass::for_tool("read").timeout_secs(),
            15
        );
        assert_eq!(
            ToolTimeoutClass::for_tool("exec").timeout_secs(),
            120
        );
        assert_ne!(timeout_run_status_label(), "completed");
    }
}
