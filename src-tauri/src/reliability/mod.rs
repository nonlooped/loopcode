//! Reliability: backup, limits, logs, retry, updates.

pub mod backup;
pub mod limits;
pub mod logs;
pub mod retry;
pub mod updates;

pub use backup::{
    backup_database, integrity_check, open_with_integrity, restore_from_backup, IntegrityReport,
};
pub use limits::{
    default_limits, evaluate_run_limits, LimitDecision, LimitKind, ResourceLimits, RunUsageSnapshot,
};
pub use logs::{append_local_log, LocalLogAppend};
pub use retry::{
    plan_provider_retry, should_auto_retry, RetryDecision, RetryPlan, ToolTimeoutClass,
    MAX_AUTO_RETRIES,
};
pub use updates::{
    run_update_check, update_check_allowed, verify_update_signature, UpdateCheckConfig,
    VerifyResult,
};
