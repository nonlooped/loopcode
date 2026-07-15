//! Per-run wall-time and estimated cost soft limits (never weaken security).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceLimits {
    /// Max wall time per run (ms).
    pub max_wall_ms: u64,
    /// Estimated cost ceiling in micros (1e-6 currency units).
    pub max_cost_micros: i64,
    /// Warn ratio (0.0–1.0), default 0.8.
    pub warn_ratio: f64,
}

pub fn default_limits() -> ResourceLimits {
    ResourceLimits {
        max_wall_ms: 30 * 60 * 1000, // 30 min
        max_cost_micros: 5_000_000,  // $5.00
        warn_ratio: 0.8,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunUsageSnapshot {
    pub elapsed_ms: u64,
    pub cost_micros: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitKind {
    None,
    WallTimeWarn,
    WallTimeStop,
    CostWarn,
    CostStop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitDecision {
    pub kind: LimitKind,
    pub should_stop: bool,
    pub should_warn: bool,
    pub message: String,
    /// Explicit: limits never disable approvals or security policy.
    pub security_policy_intact: bool,
}

/// Evaluate soft limits. Warn at warn_ratio, stop at 100%.
pub fn evaluate_run_limits(limits: &ResourceLimits, usage: &RunUsageSnapshot) -> LimitDecision {
    let warn_ratio = if limits.warn_ratio <= 0.0 || limits.warn_ratio >= 1.0 {
        0.8
    } else {
        limits.warn_ratio
    };

    if usage.elapsed_ms >= limits.max_wall_ms {
        return LimitDecision {
            kind: LimitKind::WallTimeStop,
            should_stop: true,
            should_warn: true,
            message: format!(
                "per-run wall time limit reached ({} ms)",
                limits.max_wall_ms
            ),
            security_policy_intact: true,
        };
    }
    if usage.cost_micros >= limits.max_cost_micros {
        return LimitDecision {
            kind: LimitKind::CostStop,
            should_stop: true,
            should_warn: true,
            message: format!(
                "estimated cost ceiling reached ({} micros)",
                limits.max_cost_micros
            ),
            security_policy_intact: true,
        };
    }

    let wall_warn = (limits.max_wall_ms as f64 * warn_ratio) as u64;
    let cost_warn = (limits.max_cost_micros as f64 * warn_ratio) as i64;

    if usage.elapsed_ms >= wall_warn {
        return LimitDecision {
            kind: LimitKind::WallTimeWarn,
            should_stop: false,
            should_warn: true,
            message: format!(
                "approaching wall time limit ({}/{} ms)",
                usage.elapsed_ms, limits.max_wall_ms
            ),
            security_policy_intact: true,
        };
    }
    if usage.cost_micros >= cost_warn {
        return LimitDecision {
            kind: LimitKind::CostWarn,
            should_stop: false,
            should_warn: true,
            message: format!(
                "approaching cost ceiling ({}/{} micros)",
                usage.cost_micros, limits.max_cost_micros
            ),
            security_policy_intact: true,
        };
    }

    LimitDecision {
        kind: LimitKind::None,
        should_stop: false,
        should_warn: false,
        message: "within limits".into(),
        security_policy_intact: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warn_then_stop_preserves_security_flag() {
        let lim = ResourceLimits {
            max_wall_ms: 1000,
            max_cost_micros: 1000,
            warn_ratio: 0.8,
        };
        let warn = evaluate_run_limits(
            &lim,
            &RunUsageSnapshot {
                elapsed_ms: 850,
                cost_micros: 0,
            },
        );
        assert!(warn.should_warn && !warn.should_stop);
        assert!(warn.security_policy_intact);

        let stop = evaluate_run_limits(
            &lim,
            &RunUsageSnapshot {
                elapsed_ms: 1000,
                cost_micros: 0,
            },
        );
        assert!(stop.should_stop);
        assert!(stop.security_policy_intact);
        assert_eq!(stop.kind, LimitKind::WallTimeStop);

        let cost_stop = evaluate_run_limits(
            &lim,
            &RunUsageSnapshot {
                elapsed_ms: 0,
                cost_micros: 1000,
            },
        );
        assert!(cost_stop.should_stop);
        assert_eq!(cost_stop.kind, LimitKind::CostStop);
    }
}
