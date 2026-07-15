//! Security spine (path boundary, approvals, secrets, audit, trust).

pub mod approval;
pub mod audit;
pub mod gate;
pub mod path;
pub mod redact;
pub mod secrets;
pub mod trust;

pub use approval::{
    approval_decision, security_gate_decision, ActionClass, ApprovalDecision, GrantScope,
    SECURITY_POLICY_VERSION,
};
pub use gate::{evaluate_gate, apply_grants, GateResult, GrantTable};
pub use path::{classify_path, PathClass, PathClassification};
pub use redact::{
    redact_json_value, redact_payload_json, redact_text, redact_tool_result_for_event,
};
pub use secrets::{SecretBackendKind, SharedSecretStore};
pub use trust::{content_hash, grant_trust, invalidate_trust, is_trusted, TrustKind};
