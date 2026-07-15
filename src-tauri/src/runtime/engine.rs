//! Agent runtime engine: lifecycle, mode policy, mock tools, semantic events.

use crate::db::store::Database;
use crate::domain::{Event, Run, RunStatus};
use crate::runtime::error::{
    ErrorCategory, ErrorOrigin, ErrorStage, RuntimeError, SideEffectCertainty,
};
use crate::runtime::events::{kinds, lifecycle_kind_for_status};
use crate::runtime::mode::{mode_allows_tool_effect, Mode};
use crate::runtime::tools::{MockToolRunner, ToolOutcomeKind, ToolProposal, ToolResult};
use crate::security::approval::{ApprovalDecision, GrantScope, SECURITY_POLICY_VERSION};
use crate::security::audit::{self, ApprovalAudit};
use crate::security::gate::{
    apply_grants, classify_proposal_path, evaluate_gate, GrantTable,
};
use crate::security::path::PathClass;
use crate::reliability::limits::{
    default_limits, evaluate_run_limits, ResourceLimits, RunUsageSnapshot,
};
use crate::reliability::retry::{
    plan_provider_retry, RetryDecision, RetryPlan, ToolTimeoutClass,
};
use crate::security::{redact_json_value, redact_tool_result_for_event, ActionClass};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Policy version stamped on runs (security matrix identity).
pub const RUNTIME_POLICY_VERSION: &str = SECURITY_POLICY_VERSION;

/// Outcome of [`AgentRuntime::begin_tool`]: finished under the lock, or ready for unlocked I/O.
pub enum BeginTool {
    Finished(ToolResult),
    Ready(LongToolSpec),
}

/// Spec for executing a gated tool after releasing host locks.
#[derive(Debug, Clone)]
pub struct LongToolSpec {
    pub proposal: ToolProposal,
    pub root: Option<PathBuf>,
    pub chat_id: String,
    pub project_id: String,
    pub mode: Mode,
    pub timeout_class: ToolTimeoutClass,
}

/// Tool call paused for user approval, plus any same-batch calls not yet started.
#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub proposal: ToolProposal,
    pub action_class: ActionClass,
    pub chat_id: String,
    pub project_id: String,
    pub mode: Mode,
    pub root: Option<PathBuf>,
    pub timeout_class: ToolTimeoutClass,
    /// Later tool_calls from the same assistant turn that never reached begin_tool.
    pub remaining: Vec<ToolProposal>,
}

/// Core-owned runtime driving a single chat's active run against SQLite + mock tools.
pub struct AgentRuntime {
    tools: MockToolRunner,
    /// Call IDs executed after a cancel was requested (should stay empty).
    post_cancel_executions: Vec<String>,
    cancel_requested: HashSet<String>,
    /// When cancel hits during mock tool work with uncertain side effects.
    uncertain_side_effects: HashSet<String>,
    /// Approval grants scoped per chat / once.
    grants: GrantTable,
    /// Optional project root override for path checks (tests).
    project_root_override: Option<PathBuf>,
    /// Optional checkpoint store root (outside project). When set, Build mutations checkpoint.
    checkpoint_store_override: Option<PathBuf>,
    /// Paused tool proposal per run while status is approval_waiting.
    pending_approvals: HashMap<String, PendingApproval>,
    /// Wall-clock start of each active run (for soft time limits).
    run_started: HashMap<String, Instant>,
    /// Accumulated estimated cost micros per run.
    run_cost_micros: HashMap<String, i64>,
    /// Auto-retry attempts already consumed for provider failures (per run).
    provider_retry_attempts: HashMap<String, u32>,
    /// Soft resource limits (never weaken security).
    resource_limits: ResourceLimits,
}

impl Default for AgentRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentRuntime {
    pub fn new() -> Self {
        Self {
            tools: MockToolRunner::with_default_tools(),
            post_cancel_executions: Vec::new(),
            cancel_requested: HashSet::new(),
            uncertain_side_effects: HashSet::new(),
            grants: GrantTable::default(),
            project_root_override: None,
            checkpoint_store_override: None,
            pending_approvals: HashMap::new(),
            run_started: HashMap::new(),
            run_cost_micros: HashMap::new(),
            provider_retry_attempts: HashMap::new(),
            resource_limits: default_limits(),
        }
    }

    pub fn set_project_root_override(&mut self, root: Option<PathBuf>) {
        self.project_root_override = root;
    }

    pub fn set_checkpoint_store_override(&mut self, store: Option<PathBuf>) {
        self.checkpoint_store_override = store;
    }

    pub fn set_resource_limits(&mut self, limits: ResourceLimits) {
        self.resource_limits = limits;
    }

    pub fn add_run_cost_micros(&mut self, run_id: &str, cost_micros: i64) {
        *self.run_cost_micros.entry(run_id.to_string()).or_insert(0) += cost_micros;
    }

    fn usage_snapshot(&self, run_id: &str) -> RunUsageSnapshot {
        let elapsed_ms = self
            .run_started
            .get(run_id)
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let cost_micros = self.run_cost_micros.get(run_id).copied().unwrap_or(0);
        RunUsageSnapshot {
            elapsed_ms,
            cost_micros,
        }
    }

    /// Enforce soft limits before more work. Returns Err after failing the run.
    fn enforce_limits_or_fail(&mut self, db: &Database, run_id: &str) -> Result<(), String> {
        let usage = self.usage_snapshot(run_id);
        let decision = evaluate_run_limits(&self.resource_limits, &usage);
        if decision.should_warn && !decision.should_stop {
            let run = db.get_run(run_id)?.ok_or_else(|| "run missing".to_string())?;
            let _ = self.emit(
                db,
                &run.chat_id,
                Some(run_id),
                "run.limit_warning",
                json!({
                    "kind": decision.kind,
                    "message": decision.message,
                    "securityPolicyIntact": decision.security_policy_intact,
                    "usage": usage,
                }),
            );
        }
        if decision.should_stop {
            let err = RuntimeError::new(
                ErrorCategory::Runtime,
                ErrorOrigin::Runtime,
                ErrorStage::Terminal,
                SideEffectCertainty::Completed,
                decision.message.clone(),
            )
            .with_detail(format!(
                "limit={:?}; security_policy_intact={}",
                decision.kind, decision.security_policy_intact
            ));
            let _ = self.fail(db, run_id, err)?;
            return Err(decision.message);
        }
        Ok(())
    }

    /// Record a user approval grant for a chat (deny / once / this-chat).
    pub fn grant_approval(
        &mut self,
        db: &Database,
        chat_id: &str,
        run_id: Option<&str>,
        project_id: Option<&str>,
        action_class: ActionClass,
        scope: GrantScope,
    ) -> Result<(), String> {
        self.grants.grant(chat_id, action_class, scope);
        let _ = audit::audit_approval(
            db,
            ApprovalAudit {
                chat_id,
                run_id: run_id.unwrap_or(""),
                project_id,
                action_class: action_class.as_str(),
                decision: "granted",
                grant: Some(match scope {
                    GrantScope::Deny => "deny",
                    GrantScope::AllowOnce => "allow_once",
                    GrantScope::AllowMatchingForChat => "allow_matching_for_chat",
                }),
                tool_name: "",
            },
        )?;
        Ok(())
    }

    /// Pending gated tool for a run waiting on approval.
    pub fn pending_approval(&self, run_id: &str) -> Option<&PendingApproval> {
        self.pending_approvals.get(run_id)
    }

    /// Attach later same-batch proposals that never started because approval halted the loop.
    pub fn set_pending_remaining(
        &mut self,
        run_id: &str,
        remaining: Vec<ToolProposal>,
    ) -> Result<(), String> {
        let pending = self
            .pending_approvals
            .get_mut(run_id)
            .ok_or_else(|| format!("no pending approval for run {run_id}"))?;
        pending.remaining = remaining;
        Ok(())
    }

    pub fn tools(&self) -> &MockToolRunner {
        &self.tools
    }

    pub fn tools_mut(&mut self) -> &mut MockToolRunner {
        &mut self.tools
    }

    pub fn post_cancel_executions(&self) -> &[String] {
        &self.post_cancel_executions
    }

    /// Start a new run in `mode` for `chat_id`. Suspends any prior active run.
    ///
    /// Provider adapter is `"mock"` when none is supplied (unit tests). Production
    /// entry points should use [`start_run_with_provider`].
    pub fn start_run(
        &mut self,
        db: &Database,
        chat_id: &str,
        mode: Mode,
        model: Option<&str>,
    ) -> Result<Run, String> {
        self.start_run_with_provider(db, chat_id, mode, model, "mock")
    }

    /// Start a run stamped with the connected first-party (or custom) adapter id.
    pub fn start_run_with_provider(
        &mut self,
        db: &Database,
        chat_id: &str,
        mode: Mode,
        model: Option<&str>,
        provider_id: &str,
    ) -> Result<Run, String> {
        // Fresh mock runner so a prior cancel does not block the new run.
        self.tools = MockToolRunner::with_default_tools();
        self.cancel_requested.remove(""); // no-op hygiene
        let run = db.create_run(
            chat_id,
            mode.as_str(),
            model,
            Some(provider_id),
            Some(RUNTIME_POLICY_VERSION),
        )?;
        // create_run inserts as queued; emit queued lifecycle then advance.
        let _ = self.emit(
            db,
            chat_id,
            Some(&run.id),
            kinds::RUN_QUEUED,
            json!({
                "runId": run.id,
                "status": "queued",
                "mode": mode.as_str(),
                "providerId": provider_id,
            }),
        )?;
        self.transition(db, &run.id, RunStatus::Preparing)?;
        self.transition(db, &run.id, RunStatus::ModelActive)?;
        self.run_started.insert(run.id.clone(), Instant::now());
        self.run_cost_micros.insert(run.id.clone(), 0);
        self.provider_retry_attempts.insert(run.id.clone(), 0);
        // Clear any stale cancel flag for this new id (ids are unique).
        self.cancel_requested.remove(&run.id);
        db.get_run(&run.id)?
            .ok_or_else(|| "run missing after start".into())
    }

    /// Whether cancel was requested for this run (host agent loop cooperates).
    pub fn is_cancel_requested(&self, run_id: &str) -> bool {
        self.cancel_requested.contains(run_id)
    }

    /// Public soft-limit check used by the agent loop before model rounds.
    pub fn enforce_limits_public(&mut self, db: &Database, run_id: &str) -> Result<(), String> {
        self.enforce_limits_or_fail(db, run_id)
    }

    /// Register an external cancel flag observer (transport) for this run.
    pub fn mark_cancel_requested(&mut self, run_id: &str) {
        self.cancel_requested.insert(run_id.to_string());
    }

    /// Transition run status and emit a lifecycle semantic event.
    pub fn transition(
        &mut self,
        db: &Database,
        run_id: &str,
        to: RunStatus,
    ) -> Result<Run, String> {
        let run = db
            .get_run(run_id)?
            .ok_or_else(|| format!("run not found: {run_id}"))?;
        // Terminal statuses are closed: only idempotent re-set of the same status.
        // Never rewrite completed/failed/cancelled into suspended (or any other status).
        if run.status.is_terminal() {
            if run.status == to {
                return Ok(run);
            }
            return Err(format!(
                "cannot transition terminal run {} from {} to {}",
                run_id,
                run.status.as_str(),
                to.as_str()
            ));
        }

        let updated = db.set_run_status(run_id, to)?;
        if let Some(kind) = lifecycle_kind_for_status(to.as_str()) {
            let payload = json!({
                "runId": run_id,
                "status": to.as_str(),
                "from": run.status.as_str(),
                "mode": updated.mode,
            });
            self.emit(db, &updated.chat_id, Some(run_id), kind, payload)?;
        }
        Ok(updated)
    }

    /// Emit a semantic event into the durable stream.
    pub fn emit(
        &self,
        db: &Database,
        chat_id: &str,
        run_id: Option<&str>,
        kind: &str,
        payload: Value,
    ) -> Result<Event, String> {
        let payload = redact_json_value(&payload);
        db.append_event(chat_id, run_id, kind, &payload.to_string())
    }

    /// Handle a tool proposal under the run's mode; records proposal + result events.
    pub fn propose_tool(
        &mut self,
        db: &Database,
        run_id: &str,
        proposal: ToolProposal,
    ) -> Result<ToolResult, String> {
        let run = db
            .get_run(run_id)?
            .ok_or_else(|| format!("run not found: {run_id}"))?;
        if run.status.is_terminal() || run.status == RunStatus::Suspended {
            return Err(format!(
                "cannot propose tools on run in status {}",
                run.status.as_str()
            ));
        }
        // Soft cost/time ceilings enforced on the real run path (never skip security).
        self.enforce_limits_or_fail(db, run_id)?;
        // Stamp timeout class for diagnostics (tool path still applies its own timer).
        let _timeout_class = ToolTimeoutClass::for_tool(&proposal.tool_name);
        if self.cancel_requested.contains(run_id) || run.status == RunStatus::Cancelling {
            // Do not schedule work after cancel; never execute.
            let result = ToolResult {
                call_id: proposal.call_id.clone(),
                tool_name: proposal.tool_name.clone(),
                kind: crate::runtime::tools::ToolOutcomeKind::Rejected,
                executed: false,
                content: json!({"status": "not_scheduled", "reason": "run_cancelled"}),
                error_message: Some("run cancelled; tool not scheduled".into()),
            };
            if result.executed {
                self.post_cancel_executions.push(proposal.call_id.clone());
            }
            let _ = self.emit(
                db,
                &run.chat_id,
                Some(run_id),
                kinds::TOOL_REJECTED,
                json!({
                    "callId": proposal.call_id,
                    "toolName": proposal.tool_name,
                    "reason": "run_cancelled",
                }),
            )?;
            return Ok(result);
        }

        let mode = Mode::parse(&run.mode)
            .ok_or_else(|| format!("invalid mode on run: {}", run.mode))?;

        let effect = self
            .tools
            .get(&proposal.tool_name)
            .map(|d| d.effect)
            .unwrap_or(crate::runtime::mode::ToolEffect::Mutating);

        // Resolve project root for path boundary
        let chat = db
            .get_chat(&run.chat_id)?
            .ok_or_else(|| format!("chat not found: {}", run.chat_id))?;
        let project = db.get_project(&chat.project_id)?;
        let root_owned = self.project_root_override.clone().or_else(|| {
            project
                .as_ref()
                .map(|p| PathBuf::from(&p.resolved_path))
        });
        let root_ref: Option<&Path> = root_owned.as_deref();
        let path_class = classify_proposal_path(root_ref, &proposal);

        let gate = evaluate_gate(mode, effect, &proposal.tool_name, path_class.as_ref());
        let decision = apply_grants(&gate, &mut self.grants, &run.chat_id);

        self.transition(db, run_id, RunStatus::ToolActive)?;

        let _ = self.emit(
            db,
            &run.chat_id,
            Some(run_id),
            kinds::TOOL_PROPOSAL,
            json!({
                "callId": proposal.call_id,
                "toolName": proposal.tool_name,
                "arguments": proposal.arguments,
                "mode": mode.as_str(),
                "actionClass": gate.action_class.as_str(),
                "policyVersion": gate.policy_version,
                "pathClass": path_class.as_ref().map(|p| format!("{:?}", p.class)),
            }),
        )?;

        // Mode policy still enforced by handle_proposal; security gate first.
        let result = if decision != ApprovalDecision::Allow {
            let _ = audit::audit_approval(
                db,
                ApprovalAudit {
                    chat_id: &run.chat_id,
                    run_id,
                    project_id: Some(&chat.project_id),
                    action_class: gate.action_class.as_str(),
                    decision: decision.as_str(),
                    grant: None,
                    tool_name: &proposal.tool_name,
                },
            )?;
            if decision == ApprovalDecision::RequireApproval {
                self.transition(db, run_id, RunStatus::ApprovalWaiting)?;
            }
            let content = json!({
                "status": decision.as_str(),
                "actionClass": gate.action_class.as_str(),
                "policyVersion": gate.policy_version,
                "reason": gate.reason,
                "path": path_class.as_ref().map(|p| &p.reason),
            });
            if decision == ApprovalDecision::RequireApproval {
                let timeout_class =
                    crate::reliability::retry::ToolTimeoutClass::for_tool(&proposal.tool_name);
                self.pending_approvals.insert(
                    run_id.to_string(),
                    PendingApproval {
                        proposal: proposal.clone(),
                        action_class: gate.action_class,
                        chat_id: run.chat_id.clone(),
                        project_id: chat.project_id.clone(),
                        mode,
                        root: root_owned.clone(),
                        timeout_class,
                        remaining: Vec::new(),
                    },
                );
                ToolResult {
                    call_id: proposal.call_id.clone(),
                    tool_name: proposal.tool_name.clone(),
                    kind: ToolOutcomeKind::PendingApproval,
                    executed: false,
                    content,
                    // Not a failure — UI reads content.status; model history skips these.
                    error_message: None,
                }
            } else {
                ToolResult {
                    call_id: proposal.call_id.clone(),
                    tool_name: proposal.tool_name.clone(),
                    kind: ToolOutcomeKind::Rejected,
                    executed: false,
                    content,
                    error_message: Some(gate.reason.clone()),
                }
            }
        } else if !mode_allows_tool_effect(mode, effect) {
            // Belt-and-suspenders: mode layer
            self.tools.handle_proposal(mode, &proposal)
        } else if path_class
            .as_ref()
            .is_some_and(|p| p.class == PathClass::Rejected)
        {
            ToolResult {
                call_id: proposal.call_id.clone(),
                tool_name: proposal.tool_name.clone(),
                kind: ToolOutcomeKind::Rejected,
                executed: false,
                content: json!({"status": "path_rejected", "reason": path_class.as_ref().unwrap().reason}),
                error_message: Some(path_class.as_ref().unwrap().reason.clone()),
            }
        } else {
            let _ = audit::audit_approval(
                db,
                ApprovalAudit {
                    chat_id: &run.chat_id,
                    run_id,
                    project_id: Some(&chat.project_id),
                    action_class: gate.action_class.as_str(),
                    decision: "allow",
                    grant: None,
                    tool_name: &proposal.tool_name,
                },
            )?;
            self.pending_approvals.remove(run_id);
            // Prefer real built-in tools when project root is known.
            if crate::tools::is_builtin_implemented(&proposal.tool_name) {
                crate::tools::execute_builtin(
                    root_ref,
                    Some(db),
                    Some(&chat.project_id),
                    Some(&run.chat_id),
                    Some(run_id),
                    mode,
                    &proposal,
                )
            } else {
                self.tools.handle_proposal(mode, &proposal)
            }
        };

        let kind = if result.executed {
            kinds::TOOL_RESULT
        } else {
            kinds::TOOL_REJECTED
        };
        let event_content = redact_tool_result_for_event(
            &result.content,
            root_ref,
            &proposal.arguments,
        );
        let _ = self.emit(
            db,
            &run.chat_id,
            Some(run_id),
            kind,
            json!({
                "callId": result.call_id,
                "toolName": result.tool_name,
                "executed": result.executed,
                "outcome": result.kind,
                "content": event_content,
                "error": result.error_message,
            }),
        )?;

        // Build mutating workspace tools create a content-addressed checkpoint.
        if result.executed
            && matches!(mode, Mode::Build)
            && matches!(
                proposal.tool_name.as_str(),
                "write" | "edit" | "patch"
            )
        {
            if let (Some(root), Some(proj)) = (root_owned.as_ref(), project.as_ref()) {
                if let Some(store) = self.checkpoint_store_override.clone().or_else(|| {
                    crate::db::store::default_data_dir()
                        .ok()
                        .map(|d| d.join("checkpoints").join(&proj.id))
                }) {
                    let cursor = db
                        .list_events(&run.chat_id, None)
                        .ok()
                        .and_then(|evs| evs.last().map(|e| e.seq));
                    let _ = crate::surfaces::create_checkpoint(
                        root,
                        &store,
                        &proj.id,
                        Some(&run.chat_id),
                        Some(run_id),
                        cursor,
                    );
                }
            }
        }

        // Tool timeout / hang: fail the run (never leave path open to "completed").
        let timed_out = result.content.get("timedOut").and_then(|v| v.as_bool()) == Some(true)
            || result.content.get("status").and_then(|s| s.as_str()) == Some("timeout");
        if timed_out {
            let err = RuntimeError::new(
                ErrorCategory::ToolTimeout,
                ErrorOrigin::Tool,
                ErrorStage::ToolExecution,
                SideEffectCertainty::Unknown,
                format!(
                    "tool {} timed out (class={:?})",
                    proposal.tool_name, _timeout_class
                ),
            )
            .with_detail(crate::reliability::retry::timeout_run_status_label());
            let _ = self.fail(db, run_id, err)?;
            return Ok(result);
        }

        // Return to model-active so the agent can continue — unless paused for approval.
        if !self.cancel_requested.contains(run_id) {
            let current = db.get_run(run_id)?.unwrap();
            if current.status == RunStatus::ApprovalWaiting {
                // Stay in approval_waiting until grant executes the pending tool.
            } else if !current.status.is_terminal() && current.status != RunStatus::Suspended {
                self.transition(db, run_id, RunStatus::ModelActive)?;
            }
        }

        Ok(result)
    }

    /// Gate + emit proposal; if the tool can run without the live DB, return
    /// [`BeginTool::Ready`] so the caller can unlock during I/O.
    pub fn begin_tool(
        &mut self,
        db: &Database,
        run_id: &str,
        proposal: ToolProposal,
    ) -> Result<BeginTool, String> {
        if !crate::tools::tool_exec_releases_host_locks(&proposal.tool_name)
            || !crate::tools::is_builtin_implemented(&proposal.tool_name)
        {
            return Ok(BeginTool::Finished(self.propose_tool(db, run_id, proposal)?));
        }

        let run = db
            .get_run(run_id)?
            .ok_or_else(|| format!("run not found: {run_id}"))?;
        if run.status.is_terminal() || run.status == RunStatus::Suspended {
            return Err(format!(
                "cannot propose tools on run in status {}",
                run.status.as_str()
            ));
        }
        self.enforce_limits_or_fail(db, run_id)?;
        let timeout_class = ToolTimeoutClass::for_tool(&proposal.tool_name);
        if self.cancel_requested.contains(run_id) || run.status == RunStatus::Cancelling {
            let result = ToolResult {
                call_id: proposal.call_id.clone(),
                tool_name: proposal.tool_name.clone(),
                kind: ToolOutcomeKind::Rejected,
                executed: false,
                content: json!({"status": "not_scheduled", "reason": "run_cancelled"}),
                error_message: Some("run cancelled; tool not scheduled".into()),
            };
            let _ = self.emit(
                db,
                &run.chat_id,
                Some(run_id),
                kinds::TOOL_REJECTED,
                json!({
                    "callId": proposal.call_id,
                    "toolName": proposal.tool_name,
                    "reason": "run_cancelled",
                }),
            )?;
            return Ok(BeginTool::Finished(result));
        }

        let mode = Mode::parse(&run.mode)
            .ok_or_else(|| format!("invalid mode on run: {}", run.mode))?;
        let effect = self
            .tools
            .get(&proposal.tool_name)
            .map(|d| d.effect)
            .unwrap_or(crate::runtime::mode::ToolEffect::Mutating);

        let chat = db
            .get_chat(&run.chat_id)?
            .ok_or_else(|| format!("chat not found: {}", run.chat_id))?;
        let project = db.get_project(&chat.project_id)?;
        let root_owned = self.project_root_override.clone().or_else(|| {
            project
                .as_ref()
                .map(|p| PathBuf::from(&p.resolved_path))
        });
        let root_ref: Option<&Path> = root_owned.as_deref();
        let path_class = classify_proposal_path(root_ref, &proposal);
        let gate = evaluate_gate(mode, effect, &proposal.tool_name, path_class.as_ref());
        let decision = apply_grants(&gate, &mut self.grants, &run.chat_id);

        self.transition(db, run_id, RunStatus::ToolActive)?;
        let _ = self.emit(
            db,
            &run.chat_id,
            Some(run_id),
            kinds::TOOL_PROPOSAL,
            json!({
                "callId": proposal.call_id,
                "toolName": proposal.tool_name,
                "arguments": proposal.arguments,
                "mode": mode.as_str(),
                "actionClass": gate.action_class.as_str(),
                "policyVersion": gate.policy_version,
                "pathClass": path_class.as_ref().map(|p| format!("{:?}", p.class)),
            }),
        )?;

        if decision != ApprovalDecision::Allow {
            let _ = audit::audit_approval(
                db,
                ApprovalAudit {
                    chat_id: &run.chat_id,
                    run_id,
                    project_id: Some(&chat.project_id),
                    action_class: gate.action_class.as_str(),
                    decision: decision.as_str(),
                    grant: None,
                    tool_name: &proposal.tool_name,
                },
            )?;
            if decision == ApprovalDecision::RequireApproval {
                self.transition(db, run_id, RunStatus::ApprovalWaiting)?;
                self.pending_approvals.insert(
                    run_id.to_string(),
                    PendingApproval {
                        proposal: proposal.clone(),
                        action_class: gate.action_class,
                        chat_id: run.chat_id.clone(),
                        project_id: chat.project_id.clone(),
                        mode,
                        root: root_owned.clone(),
                        timeout_class,
                        remaining: Vec::new(),
                    },
                );
            }
            let result = ToolResult {
                call_id: proposal.call_id.clone(),
                tool_name: proposal.tool_name.clone(),
                kind: if decision == ApprovalDecision::RequireApproval {
                    ToolOutcomeKind::PendingApproval
                } else {
                    ToolOutcomeKind::Rejected
                },
                executed: false,
                content: json!({
                    "status": decision.as_str(),
                    "actionClass": gate.action_class.as_str(),
                    "policyVersion": gate.policy_version,
                    "reason": gate.reason,
                }),
                error_message: if decision == ApprovalDecision::RequireApproval {
                    None
                } else {
                    Some(gate.reason.clone())
                },
            };
            let _ = self.emit(
                db,
                &run.chat_id,
                Some(run_id),
                kinds::TOOL_REJECTED,
                json!({
                    "callId": result.call_id,
                    "toolName": result.tool_name,
                    "executed": false,
                    "outcome": result.kind,
                    "content": result.content,
                    "error": result.error_message,
                }),
            )?;
            return Ok(BeginTool::Finished(result));
        }

        if !mode_allows_tool_effect(mode, effect) {
            let result = self.tools.handle_proposal(mode, &proposal);
            return Ok(BeginTool::Finished(self.finalize_tool_result(
                db,
                run_id,
                &proposal,
                mode,
                root_owned,
                project.as_ref().map(|p| p.id.as_str()),
                result,
                timeout_class,
            )?));
        }

        if path_class
            .as_ref()
            .is_some_and(|p| p.class == PathClass::Rejected)
        {
            let result = ToolResult {
                call_id: proposal.call_id.clone(),
                tool_name: proposal.tool_name.clone(),
                kind: ToolOutcomeKind::Rejected,
                executed: false,
                content: json!({"status": "path_rejected", "reason": path_class.as_ref().unwrap().reason}),
                error_message: Some(path_class.as_ref().unwrap().reason.clone()),
            };
            return Ok(BeginTool::Finished(self.finalize_tool_result(
                db,
                run_id,
                &proposal,
                mode,
                root_owned,
                project.as_ref().map(|p| p.id.as_str()),
                result,
                timeout_class,
            )?));
        }

        let _ = audit::audit_approval(
            db,
            ApprovalAudit {
                chat_id: &run.chat_id,
                run_id,
                project_id: Some(&chat.project_id),
                action_class: gate.action_class.as_str(),
                decision: "allow",
                grant: None,
                tool_name: &proposal.tool_name,
            },
        )?;
        self.pending_approvals.remove(run_id);

        Ok(BeginTool::Ready(LongToolSpec {
            proposal,
            root: root_owned,
            chat_id: run.chat_id,
            project_id: chat.project_id,
            mode,
            timeout_class,
        }))
    }

    /// Emit tool result / checkpoint / timeout handling after unlocked execution.
    pub fn complete_long_tool(
        &mut self,
        db: &Database,
        run_id: &str,
        spec: &LongToolSpec,
        result: ToolResult,
    ) -> Result<ToolResult, String> {
        self.finalize_tool_result(
            db,
            run_id,
            &spec.proposal,
            spec.mode,
            spec.root.clone(),
            Some(spec.project_id.as_str()),
            result,
            spec.timeout_class,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn finalize_tool_result(
        &mut self,
        db: &Database,
        run_id: &str,
        proposal: &ToolProposal,
        mode: Mode,
        root_owned: Option<PathBuf>,
        project_id: Option<&str>,
        result: ToolResult,
        timeout_class: ToolTimeoutClass,
    ) -> Result<ToolResult, String> {
        let run = db
            .get_run(run_id)?
            .ok_or_else(|| format!("run not found: {run_id}"))?;

        let kind = if result.executed {
            kinds::TOOL_RESULT
        } else {
            kinds::TOOL_REJECTED
        };
        let event_content = redact_tool_result_for_event(
            &result.content,
            root_owned.as_deref(),
            &proposal.arguments,
        );
        let _ = self.emit(
            db,
            &run.chat_id,
            Some(run_id),
            kind,
            json!({
                "callId": result.call_id,
                "toolName": result.tool_name,
                "executed": result.executed,
                "outcome": result.kind,
                "content": event_content,
                "error": result.error_message,
            }),
        )?;

        if result.executed
            && matches!(mode, Mode::Build)
            && matches!(
                proposal.tool_name.as_str(),
                "write" | "edit" | "patch"
            )
        {
            if let (Some(root), Some(pid)) = (root_owned.as_ref(), project_id) {
                if let Some(store) = self.checkpoint_store_override.clone().or_else(|| {
                    crate::db::store::default_data_dir()
                        .ok()
                        .map(|d| d.join("checkpoints").join(pid))
                }) {
                    let cursor = db
                        .list_events(&run.chat_id, None)
                        .ok()
                        .and_then(|evs| evs.last().map(|e| e.seq));
                    let _ = crate::surfaces::create_checkpoint(
                        root,
                        &store,
                        pid,
                        Some(&run.chat_id),
                        Some(run_id),
                        cursor,
                    );
                }
            }
        }

        let timed_out = result.content.get("timedOut").and_then(|v| v.as_bool()) == Some(true)
            || result.content.get("status").and_then(|s| s.as_str()) == Some("timeout");
        if timed_out {
            let err = RuntimeError::new(
                ErrorCategory::ToolTimeout,
                ErrorOrigin::Tool,
                ErrorStage::ToolExecution,
                SideEffectCertainty::Unknown,
                format!(
                    "tool {} timed out (class={:?})",
                    proposal.tool_name, timeout_class
                ),
            )
            .with_detail(crate::reliability::retry::timeout_run_status_label());
            let _ = self.fail(db, run_id, err)?;
            return Ok(result);
        }

        if !self.cancel_requested.contains(run_id) {
            let current = db.get_run(run_id)?.unwrap();
            if !current.status.is_terminal() && current.status != RunStatus::Suspended {
                self.transition(db, run_id, RunStatus::ModelActive)?;
            }
        }

        Ok(result)
    }

    /// After the user grants approval: execute the paused tool (consuming the grant)
    /// and return any same-batch proposals that still need to run.
    pub fn execute_granted_pending(
        &mut self,
        db: &Database,
        run_id: &str,
    ) -> Result<(ToolResult, Vec<ToolProposal>), String> {
        let pending = self
            .pending_approvals
            .remove(run_id)
            .ok_or_else(|| format!("no pending approval for run {run_id}"))?;

        let run = db
            .get_run(run_id)?
            .ok_or_else(|| format!("run not found: {run_id}"))?;
        if run.status.is_terminal() || run.status == RunStatus::Suspended {
            self.pending_approvals
                .insert(run_id.to_string(), pending);
            return Err(format!(
                "cannot execute pending tool on run in status {}",
                run.status.as_str()
            ));
        }

        self.transition(db, run_id, RunStatus::ToolActive)?;

        let root_ref = pending.root.as_deref();
        let path_class = classify_proposal_path(root_ref, &pending.proposal);
        let effect = self
            .tools
            .get(&pending.proposal.tool_name)
            .map(|d| d.effect)
            .unwrap_or(crate::runtime::mode::ToolEffect::Mutating);
        let gate = evaluate_gate(
            pending.mode,
            effect,
            &pending.proposal.tool_name,
            path_class.as_ref(),
        );
        let decision = apply_grants(&gate, &mut self.grants, &pending.chat_id);
        if decision != ApprovalDecision::Allow {
            self.pending_approvals
                .insert(run_id.to_string(), pending.clone());
            self.transition(db, run_id, RunStatus::ApprovalWaiting)?;
            return Err(format!(
                "grant did not allow pending tool (decision={})",
                decision.as_str()
            ));
        }

        let _ = audit::audit_approval(
            db,
            ApprovalAudit {
                chat_id: &pending.chat_id,
                run_id,
                project_id: Some(&pending.project_id),
                action_class: pending.action_class.as_str(),
                decision: "allow",
                grant: Some("consumed_pending"),
                tool_name: &pending.proposal.tool_name,
            },
        )?;

        let executed = if crate::tools::is_builtin_implemented(&pending.proposal.tool_name) {
            crate::tools::execute_builtin(
                root_ref,
                Some(db),
                Some(&pending.project_id),
                Some(&pending.chat_id),
                Some(run_id),
                pending.mode,
                &pending.proposal,
            )
        } else {
            self.tools
                .handle_proposal(pending.mode, &pending.proposal)
        };

        let result = self.finalize_tool_result(
            db,
            run_id,
            &pending.proposal,
            pending.mode,
            pending.root.clone(),
            Some(pending.project_id.as_str()),
            executed,
            pending.timeout_class,
        )?;

        Ok((result, pending.remaining))
    }

    /// Complete a successful run (assistant finished with no pending tools).
    /// Refuses if the run is already terminal failed/cancelled/suspended.
    pub fn complete(&mut self, db: &Database, run_id: &str) -> Result<Run, String> {
        let run = db
            .get_run(run_id)?
            .ok_or_else(|| format!("run not found: {run_id}"))?;
        if run.status == RunStatus::Failed {
            return Err(format!(
                "cannot complete failed run {run_id} (status remains failed, not completed)"
            ));
        }
        if run.status.is_terminal() {
            return Ok(run);
        }
        self.transition(db, run_id, RunStatus::Completed)
    }

    /// Fail a run with structured error taxonomy + retry plan projection for UI.
    pub fn fail(
        &mut self,
        db: &Database,
        run_id: &str,
        error: RuntimeError,
    ) -> Result<Run, String> {
        let run = db
            .get_run(run_id)?
            .ok_or_else(|| format!("run not found: {run_id}"))?;
        let attempts = self
            .provider_retry_attempts
            .get(run_id)
            .copied()
            .unwrap_or(0);
        let plan = plan_provider_retry(&error, attempts, None);
        let mut payload = serde_json::to_value(&error).map_err(|e| e.to_string())?;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "retryPlan".into(),
                serde_json::to_value(&plan).map_err(|e| e.to_string())?,
            );
            obj.insert(
                "runStatusLabel".into(),
                json!(crate::reliability::retry::timeout_run_status_label()),
            );
        }
        let _ = self.emit(db, &run.chat_id, Some(run_id), kinds::ERROR, payload)?;
        self.transition(db, run_id, RunStatus::Failed)
    }

    /// Record a provider/model failure on the live run path.
    /// Auto-retries transport/rate-limit up to cap; otherwise fails the run.
    pub fn record_provider_failure(
        &mut self,
        db: &Database,
        run_id: &str,
        error: RuntimeError,
        retry_after_secs: Option<u64>,
    ) -> Result<(RetryPlan, Run), String> {
        let run = db
            .get_run(run_id)?
            .ok_or_else(|| format!("run not found: {run_id}"))?;
        if run.status.is_terminal() {
            let attempts = self
                .provider_retry_attempts
                .get(run_id)
                .copied()
                .unwrap_or(0);
            let plan = plan_provider_retry(&error, attempts, retry_after_secs);
            return Ok((plan, run));
        }

        let attempts = self
            .provider_retry_attempts
            .entry(run_id.to_string())
            .or_insert(0);
        let plan = plan_provider_retry(&error, *attempts, retry_after_secs);

        match plan.decision {
            RetryDecision::AutoRetry {
                attempt,
                backoff_ms,
            } => {
                *attempts = attempt;
                let _ = self.emit(
                    db,
                    &run.chat_id,
                    Some(run_id),
                    "run.retry_planned",
                    json!({
                        "category": plan.category,
                        "retryability": plan.retryability,
                        "nextAction": plan.next_action,
                        "message": plan.message,
                        "attempt": attempt,
                        "backoffMs": backoff_ms,
                        "retryPlan": plan,
                    }),
                )?;
                // Stay non-terminal (model_active) so the host can retry.
                if run.status != RunStatus::ModelActive {
                    let _ = self.transition(db, run_id, RunStatus::ModelActive);
                }
                let updated = db.get_run(run_id)?.unwrap();
                Ok((plan, updated))
            }
            RetryDecision::Exhausted | RetryDecision::FailImmediately => {
                let failed = self.fail(db, run_id, error)?;
                Ok((plan, failed))
            }
        }
    }

    /// Request cancellation of an active run.
    ///
    /// Ends in **cancelled** when side effects are known stopped, or **suspended**
    /// when side-effect certainty is unknown (set via [`mark_uncertain_side_effect`]).
    pub fn request_cancel(&mut self, db: &Database, run_id: &str) -> Result<Run, String> {
        let run = db
            .get_run(run_id)?
            .ok_or_else(|| format!("run not found: {run_id}"))?;
        if run.status.is_terminal() {
            return Ok(run);
        }
        if run.status == RunStatus::Suspended {
            return Ok(run);
        }

        // Cancel is scoped to this run_id only — never poison the shared mock tool runner
        // (other chats may still have active runs).
        self.cancel_requested.insert(run_id.to_string());

        let _ = self.emit(
            db,
            &run.chat_id,
            Some(run_id),
            kinds::CANCEL_REQUESTED,
            json!({"runId": run_id}),
        )?;
        self.transition(db, run_id, RunStatus::Cancelling)?;

        let certainty = if self.uncertain_side_effects.contains(run_id) {
            SideEffectCertainty::Unknown
        } else {
            SideEffectCertainty::None
        };

        let err = RuntimeError::cancellation(
            certainty,
            match certainty {
                SideEffectCertainty::Unknown => {
                    "cancel requested; side-effect certainty unknown — suspended for reconciliation"
                }
                _ => "cancel requested; work stopped",
            },
        );
        let _ = self.emit(
            db,
            &run.chat_id,
            Some(run_id),
            kinds::ERROR,
            serde_json::to_value(&err).map_err(|e| e.to_string())?,
        )?;

        match certainty {
            SideEffectCertainty::Unknown => self.transition(db, run_id, RunStatus::Suspended),
            _ => self.transition(db, run_id, RunStatus::Cancelled),
        }
    }

    /// Mark that a run has uncertain in-flight side effects (cancel → suspended).
    pub fn mark_uncertain_side_effect(&mut self, run_id: &str) {
        self.uncertain_side_effects.insert(run_id.to_string());
    }

    /// Apply crash/restart reconciliation used by the app entry path.
    /// Non-terminal runs become **suspended** (never completed/cancelled by guess).
    pub fn reconcile_after_restart(db: &Database) -> Result<usize, String> {
        let n = db.suspend_all_inflight_runs()?;
        // Emit suspend events for any run now suspended that might need visibility
        // (best-effort: list is not filtered; callers inspect status via get_run)
        let _ = n;
        Ok(n)
    }

    /// After reopen, emit suspend lifecycle events for runs already marked suspended
    /// that still lack a suspend event (optional bookkeeping for fixtures).
    pub fn emit_suspend_markers_for_chat(
        &self,
        db: &Database,
        chat_id: &str,
    ) -> Result<usize, String> {
        let runs = db.list_runs(chat_id)?;
        let mut n = 0;
        for run in runs {
            if run.status == RunStatus::Suspended {
                let _ = self.emit(
                    db,
                    chat_id,
                    Some(&run.id),
                    kinds::RUN_SUSPENDED,
                    json!({
                        "runId": run.id,
                        "reason": "restart_reconciliation",
                        "status": "suspended",
                    }),
                )?;
                n += 1;
            }
        }
        Ok(n)
    }

    /// Internal helper for tests: force a non-terminal host phase without tools.
    pub fn enter_phase(
        &mut self,
        db: &Database,
        run_id: &str,
        status: RunStatus,
    ) -> Result<Run, String> {
        if status.is_terminal() {
            return Err("use complete/fail/cancel for terminal states".into());
        }
        self.transition(db, run_id, status)
    }
}

/// Build a policy-denial RuntimeError for a rejected tool (for taxonomy tests).
pub fn error_for_mode_rejection(message: &str) -> RuntimeError {
    RuntimeError::policy_denial(ErrorStage::ToolValidation, message)
}

/// Map failure categories used when a run fails internally.
pub fn internal_runtime_error(message: impl Into<String>) -> RuntimeError {
    RuntimeError::new(
        ErrorCategory::Runtime,
        ErrorOrigin::Runtime,
        ErrorStage::Terminal,
        SideEffectCertainty::Unknown,
        message,
    )
}
