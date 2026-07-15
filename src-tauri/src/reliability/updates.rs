//! Update check gate (default off) + signature verification contract.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

static UPDATE_HTTP_CALLS: AtomicUsize = AtomicUsize::new(0);

pub fn reset_update_http_calls() {
    UPDATE_HTTP_CALLS.store(0, Ordering::SeqCst);
}

pub fn update_http_call_count() -> usize {
    UPDATE_HTTP_CALLS.load(Ordering::SeqCst)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckConfig {
    /// When false (default), no update-channel HTTP is issued.
    pub check_enabled: bool,
}

pub fn update_check_allowed(cfg: &UpdateCheckConfig) -> bool {
    cfg.check_enabled
}

/// Perform update check only when enabled. Returns Err listing that no HTTP was made when disabled.
pub fn run_update_check(
    cfg: &UpdateCheckConfig,
    transport: &mut dyn UpdateCheckTransport,
) -> Result<String, String> {
    if !update_check_allowed(cfg) {
        return Err(format!(
            "update check disabled — no HTTP (calls={})",
            update_http_call_count()
        ));
    }
    UPDATE_HTTP_CALLS.fetch_add(1, Ordering::SeqCst);
    transport.fetch_manifest()
}

pub trait UpdateCheckTransport {
    fn fetch_manifest(&mut self) -> Result<String, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResult {
    pub accepted: bool,
    pub reason: String,
}

/// Update verification is intentionally fail-closed until a real asymmetric
/// verifier and a pinned release key are configured.
pub fn verify_update_signature(_public_key: &str, _body: &[u8], _signature: &str) -> VerifyResult {
    VerifyResult {
        accepted: false,
        reason: "update verification is unavailable: no pinned asymmetric verifier is configured".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CountingHttp {
        calls: usize,
    }
    impl UpdateCheckTransport for CountingHttp {
        fn fetch_manifest(&mut self) -> Result<String, String> {
            self.calls += 1;
            Ok("{\"version\":\"0.1.1\"}".into())
        }
    }

    #[test]
    fn disabled_check_no_http() {
        reset_update_http_calls();
        let cfg = UpdateCheckConfig {
            check_enabled: false,
        };
        let mut t = CountingHttp { calls: 0 };
        let err = run_update_check(&cfg, &mut t).unwrap_err();
        assert!(err.contains("disabled"));
        assert_eq!(t.calls, 0);
        assert_eq!(update_http_call_count(), 0);
    }

    #[test]
    fn verifier_fails_closed_until_a_pinned_asymmetric_scheme_exists() {
        let result = verify_update_signature("untrusted-input", b"artifact", "forged");
        assert!(!result.accepted);
        assert!(result.reason.contains("unavailable"));
    }
}
