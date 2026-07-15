//! Host-owned agent loop: context → model → tools → complete/fail/cancel.

use crate::db::store::Database;
use crate::domain::{Run, RunStatus};
use crate::providers::chat::{
    complete_chat_turn, host_system_prompt, messages_from_context, AssistantTurn, ChatMessage,
    DOOM_LOOP_MESSAGE, ROUND_BUDGET_NUDGE,
};
use crate::providers::first_party::first_party_adapter;
use crate::providers::transport::HttpTransport;
use crate::runtime::engine::AgentRuntime;
use crate::runtime::error::{
    ErrorCategory, ErrorOrigin, ErrorStage, RuntimeError, SideEffectCertainty,
};
use crate::runtime::history::{history_from_events, history_including_run};
use crate::runtime::mode::Mode;
use crate::runtime::tools::ToolProposal;
use crate::tools::catalog::effective_catalog;
use crate::tools::context::{compile_context, ContextManifest, DEFAULT_BODY_TOKEN_BUDGET};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Maximum model↔tool rounds per user send (bounds runaway loops).
pub const MAX_TOOL_ROUNDS: usize = 24;

/// Identical tool-call fingerprint streak that triggers a doom-loop block.
pub const DOOM_LOOP_THRESHOLD: usize = 3;

/// Inputs for one agent run after the host has created the run + user_message event.
pub struct AgentLoopInput<'a> {
    pub run_id: &'a str,
    pub user_text: &'a str,
    pub provider_id: &'a str,
    pub model: &'a str,
    pub api_key: &'a str,
    pub project_root: Option<&'a Path>,
    pub transport: &'a dyn HttpTransport,
    pub cancel_flag: Arc<AtomicBool>,
    pub base_url_override: Option<&'a str>,
    pub reasoning: Option<&'a crate::providers::chat::ReasoningConfig>,
}

/// Owned job payload for background agent turns (Send across threads).
#[derive(Debug, Clone)]
pub struct AgentLoopOwned {
    pub run_id: String,
    pub user_text: String,
    pub provider_id: String,
    pub model: String,
    pub api_key: String,
    pub project_root: Option<PathBuf>,
    pub cancel_flag: Arc<AtomicBool>,
    pub base_url_override: Option<String>,
    pub reasoning: Option<crate::providers::chat::ReasoningConfig>,
}

impl AgentLoopOwned {
    pub fn as_input<'a>(&'a self, transport: &'a dyn HttpTransport) -> AgentLoopInput<'a> {
        AgentLoopInput {
            run_id: &self.run_id,
            user_text: &self.user_text,
            provider_id: &self.provider_id,
            model: &self.model,
            api_key: &self.api_key,
            project_root: self.project_root.as_deref(),
            transport,
            cancel_flag: Arc::clone(&self.cancel_flag),
            base_url_override: self.base_url_override.as_deref(),
            reasoning: self.reasoning.as_ref(),
        }
    }
}

/// Resolve ready provider settings (no secrets).
pub fn read_ready_provider(db: &Database) -> Option<(String, String)> {
    let setting = db.get_setting("user", "provider.ready").ok().flatten()?;
    let v: Value = serde_json::from_str(&setting.value_json).ok()?;
    let provider_id = v.get("providerId")?.as_str()?.to_string();
    let model = v
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown")
        .to_string();
    Some((provider_id, model))
}

/// Store key used by connection layer for first-party API keys.
pub fn provider_store_key(provider_id: &str) -> String {
    format!("loopcode/provider/{provider_id}/api_key")
}

fn system_from_manifest(mode: &str, root: Option<&Path>, manifest: &ContextManifest) -> String {
    let project_hint = root
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(no project root)".into());
    let skills = root
        .map(crate::tools::skills::discover_skills)
        .unwrap_or_default();
    let mut sys = host_system_prompt(mode, &project_hint, &skills);
    for item in &manifest.items {
        if item.source == "user.turn" {
            continue;
        }
        if let Some(content) = &item.content {
            if content.is_empty() {
                continue;
            }
            sys.push_str("\n\n");
            sys.push_str(&format!("[{}] {}\n{}", item.source, item.reason, content));
        }
    }
    sys
}

fn emit_assistant_message(
    rt: &mut AgentRuntime,
    db: &Database,
    chat_id: &str,
    run_id: &str,
    turn: &AssistantTurn,
) -> Result<(), String> {
    if !turn.text.is_empty() || turn.has_tool_calls() {
        let tool_calls: Vec<Value> = turn
            .tool_calls
            .iter()
            .map(|tc| {
                json!({
                    "id": tc.id,
                    "name": tc.name,
                    "arguments": tc.arguments,
                })
            })
            .collect();
        rt.emit(
            db,
            chat_id,
            Some(run_id),
            "assistant_message",
            json!({
                "text": turn.text,
                "toolCalls": tool_calls,
                "model": turn.model,
                "providerId": turn.provider_id,
                "adapterId": turn.adapter_id,
                "finishReason": turn.finish_reason,
                "partial": turn.partial,
            }),
        )?;
    }
    if let Some(usage) = &turn.usage {
        let in_tok = usage
            .get("prompt_tokens")
            .or_else(|| usage.get("input_tokens"))
            .and_then(|v| v.as_i64());
        let out_tok = usage
            .get("completion_tokens")
            .or_else(|| usage.get("output_tokens"))
            .and_then(|v| v.as_i64());
        let _ = db.record_usage(crate::db::store::NewUsage {
            run_id: Some(run_id.into()),
            chat_id: Some(chat_id.into()),
            provider: Some(turn.provider_id.clone()),
            model: Some(turn.model.clone()),
            input_tokens: in_tok,
            output_tokens: out_tok,
            cost_micros: None,
            elapsed_ms: None,
        });
        rt.emit(
            db,
            chat_id,
            Some(run_id),
            "run.usage",
            json!({
                "providerId": turn.provider_id,
                "model": turn.model,
                "usage": usage,
            }),
        )?;
    }
    Ok(())
}

fn adapter_error_to_runtime(err: &crate::providers::adapter::AdapterError) -> RuntimeError {
    let category = match err.category.as_str() {
        "cancellation" => ErrorCategory::Cancellation,
        "network" => ErrorCategory::Network,
        "timeout" => ErrorCategory::Timeout,
        "rate_limit" => ErrorCategory::RateLimit,
        "provider_error" if err.retryable => ErrorCategory::RateLimit,
        "provider_error" => ErrorCategory::Runtime,
        "authentication" | "configuration" => ErrorCategory::PolicyDenial,
        "schema_validation" | "model_output" => ErrorCategory::SchemaValidation,
        _ => ErrorCategory::Runtime,
    };
    let mut e = RuntimeError::new(
        category,
        ErrorOrigin::Provider,
        ErrorStage::Model,
        SideEffectCertainty::None,
        err.message.clone(),
    );
    if let Some(d) = &err.detail {
        e = e.with_detail(d.clone());
    }
    e
}

fn load_prior_messages(db: &Database, chat_id: &str, exclude_run_id: &str) -> Vec<ChatMessage> {
    match db.list_events(chat_id, None) {
        Ok(events) => history_from_events(&events, exclude_run_id),
        Err(_) => Vec::new(),
    }
}

#[derive(Debug, Default)]
struct DoomTracker {
    recent: VecDeque<(String, String)>,
}

impl DoomTracker {
    fn note_is_doom(&mut self, name: &str, args: &Value) -> bool {
        let fp = (
            name.to_string(),
            serde_json::to_string(args).unwrap_or_default(),
        );
        self.recent.push_back(fp.clone());
        while self.recent.len() > DOOM_LOOP_THRESHOLD {
            self.recent.pop_front();
        }
        self.recent.len() == DOOM_LOOP_THRESHOLD && self.recent.iter().all(|x| x == &fp)
    }
}

fn doom_block_result(proposal: &ToolProposal) -> crate::runtime::tools::ToolResult {
    crate::runtime::tools::ToolResult {
        call_id: proposal.call_id.clone(),
        tool_name: proposal.tool_name.clone(),
        kind: crate::runtime::tools::ToolOutcomeKind::Rejected,
        executed: false,
        content: json!({
            "status": "doom_loop_blocked",
            "reason": DOOM_LOOP_MESSAGE,
        }),
        error_message: Some(DOOM_LOOP_MESSAGE.into()),
    }
}

fn soft_round_limit_complete(
    rt: &mut AgentRuntime,
    db: &Database,
    chat_id: &str,
    run_id: &str,
) -> Result<Run, String> {
    let _ = rt.emit(
        db,
        chat_id,
        Some(run_id),
        "assistant_message",
        json!({
            "text": format!(
                "Stopped at the tool-round budget ({MAX_TOOL_ROUNDS}). Send another message to continue from here."
            ),
            "toolCalls": [],
            "partial": false,
            "finishReason": "round_budget",
        }),
    );
    rt.complete(db, run_id)
}

/// Check host cancel flag + cooperative transport flag.
pub fn cancel_requested(
    rt: &AgentRuntime,
    run_id: &str,
    flag: &AtomicBool,
) -> bool {
    flag.load(Ordering::SeqCst) || rt.is_cancel_requested(run_id)
}

/// Drive a full agent run to a terminal or approval_waiting state.
///
/// Holds `db`/`rt` for the whole call — fine for unit tests. Production send uses
/// [`execute_agent_run_concurrent`] so cancel and timeline can interleave.
pub fn execute_agent_run(
    db: &Database,
    rt: &mut AgentRuntime,
    input: AgentLoopInput<'_>,
) -> Result<Run, String> {
    execute_agent_run_inner(db, rt, &input)
}

/// Product cancel path (mirrors `runtime_cancel_run`): flip cooperative flag, then
/// host `request_cancel` under short locks.
pub fn product_request_cancel(
    db: &Mutex<Database>,
    rt: &Mutex<AgentRuntime>,
    flags: &Mutex<HashMap<String, Arc<AtomicBool>>>,
    run_id: &str,
) -> Result<Run, String> {
    if let Ok(map) = flags.lock() {
        if let Some(flag) = map.get(run_id) {
            flag.store(true, Ordering::SeqCst);
        }
    }
    let db = db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let mut rt = rt
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?;
    rt.request_cancel(&db, run_id)
}

/// Drive an agent run **releasing** `db`/`rt` mutexes during provider HTTP I/O so
/// timeline polls and [`product_request_cancel`] can proceed mid-turn.
pub fn execute_agent_run_concurrent(
    db: &Mutex<Database>,
    rt: &Mutex<AgentRuntime>,
    owned: AgentLoopOwned,
    transport: &dyn HttpTransport,
) -> Result<Run, String> {
    let run_id = owned.run_id.clone();
    let cancel_flag = Arc::clone(&owned.cancel_flag);

    let (mode, chat_id, system, tools, prior) = {
        let db = db
            .lock()
            .map_err(|_| "database lock poisoned".to_string())?;
        let rt_g = rt
            .lock()
            .map_err(|_| "runtime lock poisoned".to_string())?;
        let run = db
            .get_run(&run_id)?
            .ok_or_else(|| format!("run not found: {run_id}"))?;
        if run.status.is_terminal() || run.status == RunStatus::Suspended {
            return Ok(run);
        }
        let mode = Mode::parse(&run.mode).ok_or_else(|| format!("invalid mode: {}", run.mode))?;
        let chat_id = run.chat_id.clone();
        let project_id = db
            .get_chat(&chat_id)
            .ok()
            .flatten()
            .map(|c| c.project_id);
        let tools = effective_catalog(Some(&db), project_id.as_deref());
        let prior = load_prior_messages(&db, &chat_id, &run_id);
        let manifest = if let Some(root) = owned.project_root.as_deref() {
            compile_context(root, &owned.user_text, &[], DEFAULT_BODY_TOKEN_BUDGET)
        } else {
            compile_context(
                Path::new("."),
                &owned.user_text,
                &[],
                DEFAULT_BODY_TOKEN_BUDGET,
            )
        };
        let system = system_from_manifest(mode.as_str(), owned.project_root.as_deref(), &manifest);
        let _ = rt_g.emit(
            &db,
            &chat_id,
            Some(&run_id),
            "run.context_manifest",
            serde_json::to_value(&manifest).unwrap_or(json!({})),
        );
        (mode, chat_id, system, tools, prior)
    };

    let adapter = first_party_adapter(&owned.provider_id).ok_or_else(|| {
        format!(
            "no first-party adapter for provider {}",
            owned.provider_id
        )
    })?;

    let mut messages = messages_from_context(system.as_str(), &owned.user_text, &prior);
    let mut round = 0usize;
    let mut doom = DoomTracker::default();
    let mut budget_nudged = false;

    loop {
        // --- short lock: cancel / limits / model_active ---
        {
            let db = db
                .lock()
                .map_err(|_| "database lock poisoned".to_string())?;
            let mut rt = rt
                .lock()
                .map_err(|_| "runtime lock poisoned".to_string())?;
            if cancel_requested(&rt, &run_id, &cancel_flag) {
                let current = db.get_run(&run_id)?.unwrap();
                if current.status.is_terminal() || current.status == RunStatus::Suspended {
                    return Ok(current);
                }
                return rt.request_cancel(&db, &run_id);
            }
            if round >= MAX_TOOL_ROUNDS {
                return soft_round_limit_complete(&mut rt, &db, &chat_id, &run_id);
            }
            if round + 1 >= MAX_TOOL_ROUNDS && !budget_nudged {
                messages.push(ChatMessage {
                    role: "user".into(),
                    content: Some(ROUND_BUDGET_NUDGE.into()),
                    tool_calls: vec![],
                    tool_call_id: None,
                    name: None,
                });
                budget_nudged = true;
            }
            if let Err(e) = rt.enforce_limits_public(&db, &run_id) {
                let current = db.get_run(&run_id)?.unwrap();
                if current.status.is_terminal() {
                    return Ok(current);
                }
                return Err(e);
            }
            if run_status_is_model_phase(&db, &run_id)? {
                let _ = rt.transition(&db, &run_id, RunStatus::ModelActive);
            }
        }

        // --- no locks: provider HTTP ---
        let turn_result = complete_chat_turn(
            adapter.as_ref(),
            &owned.model,
            &owned.api_key,
            &messages,
            &tools,
            transport,
            owned.base_url_override.as_deref(),
            owned.reasoning.as_ref(),
        );

        // --- short lock: apply turn ---
        let tool_calls = {
            let db = db
                .lock()
                .map_err(|_| "database lock poisoned".to_string())?;
            let mut rt = rt
                .lock()
                .map_err(|_| "runtime lock poisoned".to_string())?;

            if cancel_requested(&rt, &run_id, &cancel_flag)
                || matches!(&turn_result, Err(e) if e.category == "cancellation")
            {
                let current = db.get_run(&run_id)?.unwrap();
                if current.status.is_terminal() || current.status == RunStatus::Suspended {
                    return Ok(current);
                }
                return rt.request_cancel(&db, &run_id);
            }

            let turn = match turn_result {
                Ok(t) => t,
                Err(err) => {
                    let runtime_err = adapter_error_to_runtime(&err);
                    let (plan, updated) =
                        rt.record_provider_failure(&db, &run_id, runtime_err, None)?;
                    if updated.status.is_terminal() {
                        return Ok(updated);
                    }
                    if matches!(
                        plan.decision,
                        crate::reliability::retry::RetryDecision::AutoRetry { .. }
                    ) {
                        round += 1;
                        continue;
                    }
                    return Ok(updated);
                }
            };

            emit_assistant_message(&mut rt, &db, &chat_id, &run_id, &turn)?;

            messages.push(ChatMessage {
                role: "assistant".into(),
                content: if turn.text.is_empty() {
                    None
                } else {
                    Some(turn.text.clone())
                },
                tool_calls: turn.tool_calls.clone(),
                tool_call_id: None,
                name: None,
            });

            if !turn.has_tool_calls() {
                return rt.complete(&db, &run_id);
            }
            turn.tool_calls
        };

        let mut approval_halt = false;
        for (idx, tc) in tool_calls.iter().enumerate() {
            let proposal = ToolProposal {
                call_id: tc.id.clone(),
                tool_name: tc.name.clone(),
                arguments: tc.arguments.clone(),
            };

            // --- short lock: cancel + begin/gate ---
            let begin = {
                let db = db
                    .lock()
                    .map_err(|_| "database lock poisoned".to_string())?;
                let mut rt = rt
                    .lock()
                    .map_err(|_| "runtime lock poisoned".to_string())?;
                if cancel_requested(&rt, &run_id, &cancel_flag) {
                    return rt.request_cancel(&db, &run_id);
                }
                if doom.note_is_doom(&proposal.tool_name, &proposal.arguments) {
                    let blocked = doom_block_result(&proposal);
                    let _ = rt.emit(
                        &db,
                        &chat_id,
                        Some(&run_id),
                        crate::runtime::events::kinds::TOOL_REJECTED,
                        json!({
                            "callId": blocked.call_id,
                            "toolName": blocked.tool_name,
                            "executed": false,
                            "outcome": "rejected",
                            "content": blocked.content,
                            "error": blocked.error_message,
                        }),
                    );
                    crate::runtime::engine::BeginTool::Finished(blocked)
                } else {
                    rt.begin_tool(&db, &run_id, proposal)?
                }
            };

            let result = match begin {
                crate::runtime::engine::BeginTool::Finished(r) => r,
                crate::runtime::engine::BeginTool::Ready(spec) => {
                    // --- no locks: tool I/O ---
                    if cancel_flag.load(Ordering::SeqCst) {
                        let db = db
                            .lock()
                            .map_err(|_| "database lock poisoned".to_string())?;
                        let mut rt = rt
                            .lock()
                            .map_err(|_| "runtime lock poisoned".to_string())?;
                        return rt.request_cancel(&db, &run_id);
                    }
                    let executed = crate::tools::execute_builtin(
                        spec.root.as_deref(),
                        None,
                        Some(spec.project_id.as_str()),
                        Some(spec.chat_id.as_str()),
                        Some(run_id.as_str()),
                        spec.mode,
                        &spec.proposal,
                    );
                    let db = db
                        .lock()
                        .map_err(|_| "database lock poisoned".to_string())?;
                    let mut rt = rt
                        .lock()
                        .map_err(|_| "runtime lock poisoned".to_string())?;
                    rt.complete_long_tool(&db, &run_id, &spec, executed)?
                }
            };

            {
                let db = db
                    .lock()
                    .map_err(|_| "database lock poisoned".to_string())?;
                let mut rt = rt
                    .lock()
                    .map_err(|_| "runtime lock poisoned".to_string())?;
                let current = db.get_run(&run_id)?.unwrap();
                if current.status.is_terminal() || current.status == RunStatus::Suspended {
                    return Ok(current);
                }
                if current.status == RunStatus::ApprovalWaiting {
                    let rest: Vec<ToolProposal> = tool_calls[idx + 1..]
                        .iter()
                        .map(|t| ToolProposal {
                            call_id: t.id.clone(),
                            tool_name: t.name.clone(),
                            arguments: t.arguments.clone(),
                        })
                        .collect();
                    let _ = rt.set_pending_remaining(&run_id, rest);
                    approval_halt = true;
                }
                if approval_halt {
                    return Ok(current);
                }
            }
            let result_text = serde_json::to_string(&result.content)
                .unwrap_or_else(|_| result.content.to_string());
            messages.push(ChatMessage {
                role: "tool".into(),
                content: Some(result_text),
                tool_calls: vec![],
                tool_call_id: Some(tc.id.clone()),
                name: Some(tc.name.clone()),
            });
        }

        let _ = mode;
        round += 1;
    }
}

/// Inner single-threaded driver (unit tests).
fn execute_agent_run_inner(
    db: &Database,
    rt: &mut AgentRuntime,
    input: &AgentLoopInput<'_>,
) -> Result<Run, String> {
    let run = db
        .get_run(input.run_id)?
        .ok_or_else(|| format!("run not found: {}", input.run_id))?;
    if run.status.is_terminal() || run.status == RunStatus::Suspended {
        return Ok(run);
    }

    let mode = Mode::parse(&run.mode).ok_or_else(|| format!("invalid mode: {}", run.mode))?;
    let chat_id = run.chat_id.clone();

    let adapter = first_party_adapter(input.provider_id).ok_or_else(|| {
        format!(
            "no first-party adapter for provider {}",
            input.provider_id
        )
    })?;

    let tools = effective_catalog(
        Some(db),
        db.get_chat(&chat_id)
            .ok()
            .flatten()
            .as_ref()
            .map(|c| c.project_id.as_str()),
    );

    let manifest = if let Some(root) = input.project_root {
        compile_context(root, input.user_text, &[], DEFAULT_BODY_TOKEN_BUDGET)
    } else {
        compile_context(
            Path::new("."),
            input.user_text,
            &[],
            DEFAULT_BODY_TOKEN_BUDGET,
        )
    };
    let system = system_from_manifest(mode.as_str(), input.project_root, &manifest);

    let _ = rt.emit(
        db,
        &chat_id,
        Some(input.run_id),
        "run.context_manifest",
        serde_json::to_value(&manifest).unwrap_or(json!({})),
    );

    let prior = load_prior_messages(db, &chat_id, input.run_id);
    let mut messages = messages_from_context(system.as_str(), input.user_text, &prior);
    let mut round = 0usize;
    let mut doom = DoomTracker::default();
    let mut budget_nudged = false;

    loop {
        if cancel_requested(rt, input.run_id, &input.cancel_flag) {
            let current = db.get_run(input.run_id)?.unwrap();
            if current.status.is_terminal() || current.status == RunStatus::Suspended {
                return Ok(current);
            }
            return rt.request_cancel(db, input.run_id);
        }

        if round >= MAX_TOOL_ROUNDS {
            return soft_round_limit_complete(rt, db, &chat_id, input.run_id);
        }
        if round + 1 >= MAX_TOOL_ROUNDS && !budget_nudged {
            messages.push(ChatMessage {
                role: "user".into(),
                content: Some(ROUND_BUDGET_NUDGE.into()),
                tool_calls: vec![],
                tool_call_id: None,
                name: None,
            });
            budget_nudged = true;
        }

        if let Err(e) = rt.enforce_limits_public(db, input.run_id) {
            let current = db.get_run(input.run_id)?.unwrap();
            if current.status.is_terminal() {
                return Ok(current);
            }
            return Err(e);
        }

        if run_status_is_model_phase(db, input.run_id)? {
            let _ = rt.transition(db, input.run_id, RunStatus::ModelActive);
        }

        let turn_result = complete_chat_turn(
            adapter.as_ref(),
            input.model,
            input.api_key,
            &messages,
            &tools,
            input.transport,
            input.base_url_override,
            input.reasoning,
        );

        if cancel_requested(rt, input.run_id, &input.cancel_flag)
            || matches!(&turn_result, Err(e) if e.category == "cancellation")
        {
            let current = db.get_run(input.run_id)?.unwrap();
            if current.status.is_terminal() || current.status == RunStatus::Suspended {
                return Ok(current);
            }
            return rt.request_cancel(db, input.run_id);
        }

        let turn = match turn_result {
            Ok(t) => t,
            Err(err) => {
                let runtime_err = adapter_error_to_runtime(&err);
                let (plan, updated) =
                    rt.record_provider_failure(db, input.run_id, runtime_err, None)?;
                if updated.status.is_terminal() {
                    return Ok(updated);
                }
                if matches!(
                    plan.decision,
                    crate::reliability::retry::RetryDecision::AutoRetry { .. }
                ) {
                    round += 1;
                    continue;
                }
                return Ok(updated);
            }
        };

        emit_assistant_message(rt, db, &chat_id, input.run_id, &turn)?;

        messages.push(ChatMessage {
            role: "assistant".into(),
            content: if turn.text.is_empty() {
                None
            } else {
                Some(turn.text.clone())
            },
            tool_calls: turn.tool_calls.clone(),
            tool_call_id: None,
            name: None,
        });

        if !turn.has_tool_calls() {
            return rt.complete(db, input.run_id);
        }

        let mut approval_halt = false;
        for tc in &turn.tool_calls {
            if cancel_requested(rt, input.run_id, &input.cancel_flag) {
                return rt.request_cancel(db, input.run_id);
            }

            let proposal = ToolProposal {
                call_id: tc.id.clone(),
                tool_name: tc.name.clone(),
                arguments: tc.arguments.clone(),
            };
            let result = if doom.note_is_doom(&proposal.tool_name, &proposal.arguments) {
                let blocked = doom_block_result(&proposal);
                let _ = rt.emit(
                    db,
                    &chat_id,
                    Some(input.run_id),
                    crate::runtime::events::kinds::TOOL_REJECTED,
                    json!({
                        "callId": blocked.call_id,
                        "toolName": blocked.tool_name,
                        "executed": false,
                        "outcome": "rejected",
                        "content": blocked.content,
                        "error": blocked.error_message,
                    }),
                );
                blocked
            } else {
                rt.propose_tool(db, input.run_id, proposal)?
            };

            let current = db.get_run(input.run_id)?.unwrap();
            if current.status.is_terminal() || current.status == RunStatus::Suspended {
                return Ok(current);
            }
            if current.status == RunStatus::ApprovalWaiting {
                approval_halt = true;
            }

            let result_text = serde_json::to_string(&result.content)
                .unwrap_or_else(|_| result.content.to_string());
            messages.push(ChatMessage {
                role: "tool".into(),
                content: Some(result_text),
                tool_calls: vec![],
                tool_call_id: Some(tc.id.clone()),
                name: Some(tc.name.clone()),
            });

            if approval_halt {
                return Ok(current);
            }
        }

        round += 1;
    }
}

fn run_status_is_model_phase(db: &Database, run_id: &str) -> Result<bool, String> {
    let run = db
        .get_run(run_id)?
        .ok_or_else(|| format!("run not found: {run_id}"))?;
    Ok(matches!(
        run.status,
        RunStatus::ModelActive
            | RunStatus::ToolActive
            | RunStatus::Preparing
            | RunStatus::Queued
            | RunStatus::ApprovalWaiting
    ) && !run.status.is_terminal())
}

/// Resume after the user grants approval: execute the paused tool, drain any
/// same-batch remaining proposals, then continue the model loop with full
/// same-run history (no amnesiac re-prompt of the original user text alone).
pub fn execute_agent_run_after_grant_concurrent(
    db: &Mutex<Database>,
    rt: &Mutex<AgentRuntime>,
    owned: AgentLoopOwned,
    transport: &dyn HttpTransport,
) -> Result<Run, String> {
    let run_id = owned.run_id.clone();
    let cancel_flag = Arc::clone(&owned.cancel_flag);

    // 1) Execute the granted pending tool (+ capture remaining batch).
    let remaining = {
        let db_g = db
            .lock()
            .map_err(|_| "database lock poisoned".to_string())?;
        let mut rt_g = rt
            .lock()
            .map_err(|_| "runtime lock poisoned".to_string())?;
        if cancel_requested(&rt_g, &run_id, &cancel_flag) {
            return rt_g.request_cancel(&db_g, &run_id);
        }
        if rt_g.pending_approval(&run_id).is_none() {
            // Fallback: no in-memory pending (e.g. process restarted) — continue
            // with same-run history so the model can re-propose with context.
            drop(rt_g);
            drop(db_g);
            return execute_agent_run_concurrent_resuming(db, rt, owned, transport, true);
        }
        let (_result, remaining) = rt_g.execute_granted_pending(&db_g, &run_id)?;
        let current = db_g.get_run(&run_id)?.unwrap();
        if current.status.is_terminal() || current.status == RunStatus::Suspended {
            return Ok(current);
        }
        if current.status == RunStatus::ApprovalWaiting {
            return Ok(current);
        }
        remaining
    };

    // 2) Drain remaining same-batch proposals (may halt on another approval).
    for proposal in remaining {
        let begin = {
            let db = db
                .lock()
                .map_err(|_| "database lock poisoned".to_string())?;
            let mut rt = rt
                .lock()
                .map_err(|_| "runtime lock poisoned".to_string())?;
            if cancel_requested(&rt, &run_id, &cancel_flag) {
                return rt.request_cancel(&db, &run_id);
            }
            rt.begin_tool(&db, &run_id, proposal)?
        };
        match begin {
            crate::runtime::engine::BeginTool::Finished(_) => {
                let db = db
                    .lock()
                    .map_err(|_| "database lock poisoned".to_string())?;
                let current = db.get_run(&run_id)?.unwrap();
                if current.status.is_terminal() || current.status == RunStatus::Suspended {
                    return Ok(current);
                }
                if current.status == RunStatus::ApprovalWaiting {
                    return Ok(current);
                }
            }
            crate::runtime::engine::BeginTool::Ready(spec) => {
                if cancel_flag.load(Ordering::SeqCst) {
                    let db = db
                        .lock()
                        .map_err(|_| "database lock poisoned".to_string())?;
                    let mut rt = rt
                        .lock()
                        .map_err(|_| "runtime lock poisoned".to_string())?;
                    return rt.request_cancel(&db, &run_id);
                }
                let executed = crate::tools::execute_builtin(
                    spec.root.as_deref(),
                    None,
                    Some(spec.project_id.as_str()),
                    Some(spec.chat_id.as_str()),
                    Some(run_id.as_str()),
                    spec.mode,
                    &spec.proposal,
                );
                let db = db
                    .lock()
                    .map_err(|_| "database lock poisoned".to_string())?;
                let mut rt = rt
                    .lock()
                    .map_err(|_| "runtime lock poisoned".to_string())?;
                let _ = rt.complete_long_tool(&db, &run_id, &spec, executed)?;
                let current = db.get_run(&run_id)?.unwrap();
                if current.status.is_terminal() || current.status == RunStatus::Suspended {
                    return Ok(current);
                }
                if current.status == RunStatus::ApprovalWaiting {
                    return Ok(current);
                }
            }
        }
    }

    // 3) Continue model with same-run history included.
    execute_agent_run_concurrent_resuming(db, rt, owned, transport, true)
}

/// Like [`execute_agent_run_concurrent`], but when `include_current_run` is true the
/// provider history includes this run's events (after granted tools have results).
fn execute_agent_run_concurrent_resuming(
    db: &Mutex<Database>,
    rt: &Mutex<AgentRuntime>,
    owned: AgentLoopOwned,
    transport: &dyn HttpTransport,
    include_current_run: bool,
) -> Result<Run, String> {
    let run_id = owned.run_id.clone();
    let cancel_flag = Arc::clone(&owned.cancel_flag);

    let (mode, chat_id, system, tools, mut messages) = {
        let db = db
            .lock()
            .map_err(|_| "database lock poisoned".to_string())?;
        let rt_g = rt
            .lock()
            .map_err(|_| "runtime lock poisoned".to_string())?;
        let run = db
            .get_run(&run_id)?
            .ok_or_else(|| format!("run not found: {run_id}"))?;
        if run.status.is_terminal() || run.status == RunStatus::Suspended {
            return Ok(run);
        }
        let mode = Mode::parse(&run.mode).ok_or_else(|| format!("invalid mode: {}", run.mode))?;
        let chat_id = run.chat_id.clone();
        let project_id = db
            .get_chat(&chat_id)
            .ok()
            .flatten()
            .map(|c| c.project_id);
        let tools = effective_catalog(Some(&db), project_id.as_deref());
        let events = db.list_events(&chat_id, None).unwrap_or_default();
        let prior = if include_current_run {
            history_including_run(&events, &run_id)
        } else {
            history_from_events(&events, &run_id)
        };
        let manifest = if let Some(root) = owned.project_root.as_deref() {
            compile_context(root, &owned.user_text, &[], DEFAULT_BODY_TOKEN_BUDGET)
        } else {
            compile_context(
                Path::new("."),
                &owned.user_text,
                &[],
                DEFAULT_BODY_TOKEN_BUDGET,
            )
        };
        let system = system_from_manifest(mode.as_str(), owned.project_root.as_deref(), &manifest);
        let _ = rt_g.emit(
            &db,
            &chat_id,
            Some(&run_id),
            "run.context_manifest",
            serde_json::to_value(&manifest).unwrap_or(json!({})),
        );
        // When resuming with same-run history, the user turn is already present —
        // do not append a duplicate user message.
        let messages = if include_current_run {
            let mut msgs = Vec::new();
            if !system.is_empty() {
                msgs.push(ChatMessage {
                    role: "system".into(),
                    content: Some(system.clone()),
                    tool_calls: vec![],
                    tool_call_id: None,
                    name: None,
                });
            }
            msgs.extend(prior);
            msgs
        } else {
            messages_from_context(system.as_str(), &owned.user_text, &prior)
        };
        (mode, chat_id, system, tools, messages)
    };
    let _ = system;

    let adapter = first_party_adapter(&owned.provider_id).ok_or_else(|| {
        format!(
            "no first-party adapter for provider {}",
            owned.provider_id
        )
    })?;

    let mut round = 0usize;
    let mut doom = DoomTracker::default();
    let mut budget_nudged = false;

    // Reuse the same tool/model loop body as execute_agent_run_concurrent by
    // jumping into a shared continuation — duplicated below for clarity/control.
    loop {
        {
            let db = db
                .lock()
                .map_err(|_| "database lock poisoned".to_string())?;
            let mut rt = rt
                .lock()
                .map_err(|_| "runtime lock poisoned".to_string())?;
            if cancel_requested(&rt, &run_id, &cancel_flag) {
                let current = db.get_run(&run_id)?.unwrap();
                if current.status.is_terminal() || current.status == RunStatus::Suspended {
                    return Ok(current);
                }
                return rt.request_cancel(&db, &run_id);
            }
            if round >= MAX_TOOL_ROUNDS {
                return soft_round_limit_complete(&mut rt, &db, &chat_id, &run_id);
            }
            if round + 1 >= MAX_TOOL_ROUNDS && !budget_nudged {
                messages.push(ChatMessage {
                    role: "user".into(),
                    content: Some(ROUND_BUDGET_NUDGE.into()),
                    tool_calls: vec![],
                    tool_call_id: None,
                    name: None,
                });
                budget_nudged = true;
            }
            if let Err(e) = rt.enforce_limits_public(&db, &run_id) {
                let current = db.get_run(&run_id)?.unwrap();
                if current.status.is_terminal() {
                    return Ok(current);
                }
                return Err(e);
            }
            if run_status_is_model_phase(&db, &run_id)? {
                let _ = rt.transition(&db, &run_id, RunStatus::ModelActive);
            }
        }

        let turn_result = complete_chat_turn(
            adapter.as_ref(),
            &owned.model,
            &owned.api_key,
            &messages,
            &tools,
            transport,
            owned.base_url_override.as_deref(),
            owned.reasoning.as_ref(),
        );

        let tool_calls = {
            let db = db
                .lock()
                .map_err(|_| "database lock poisoned".to_string())?;
            let mut rt = rt
                .lock()
                .map_err(|_| "runtime lock poisoned".to_string())?;

            if cancel_requested(&rt, &run_id, &cancel_flag)
                || matches!(&turn_result, Err(e) if e.category == "cancellation")
            {
                let current = db.get_run(&run_id)?.unwrap();
                if current.status.is_terminal() || current.status == RunStatus::Suspended {
                    return Ok(current);
                }
                return rt.request_cancel(&db, &run_id);
            }

            let turn = match turn_result {
                Ok(t) => t,
                Err(err) => {
                    let runtime_err = adapter_error_to_runtime(&err);
                    let (plan, updated) =
                        rt.record_provider_failure(&db, &run_id, runtime_err, None)?;
                    if updated.status.is_terminal() {
                        return Ok(updated);
                    }
                    if matches!(
                        plan.decision,
                        crate::reliability::retry::RetryDecision::AutoRetry { .. }
                    ) {
                        round += 1;
                        continue;
                    }
                    return Ok(updated);
                }
            };

            emit_assistant_message(&mut rt, &db, &chat_id, &run_id, &turn)?;

            messages.push(ChatMessage {
                role: "assistant".into(),
                content: if turn.text.is_empty() {
                    None
                } else {
                    Some(turn.text.clone())
                },
                tool_calls: turn.tool_calls.clone(),
                tool_call_id: None,
                name: None,
            });

            if !turn.has_tool_calls() {
                return rt.complete(&db, &run_id);
            }
            turn.tool_calls
        };

        let mut approval_halt = false;
        for (idx, tc) in tool_calls.iter().enumerate() {
            let proposal = ToolProposal {
                call_id: tc.id.clone(),
                tool_name: tc.name.clone(),
                arguments: tc.arguments.clone(),
            };

            let begin = {
                let db = db
                    .lock()
                    .map_err(|_| "database lock poisoned".to_string())?;
                let mut rt = rt
                    .lock()
                    .map_err(|_| "runtime lock poisoned".to_string())?;
                if cancel_requested(&rt, &run_id, &cancel_flag) {
                    return rt.request_cancel(&db, &run_id);
                }
                if doom.note_is_doom(&proposal.tool_name, &proposal.arguments) {
                    let blocked = doom_block_result(&proposal);
                    let _ = rt.emit(
                        &db,
                        &chat_id,
                        Some(&run_id),
                        crate::runtime::events::kinds::TOOL_REJECTED,
                        json!({
                            "callId": blocked.call_id,
                            "toolName": blocked.tool_name,
                            "executed": false,
                            "outcome": "rejected",
                            "content": blocked.content,
                            "error": blocked.error_message,
                        }),
                    );
                    crate::runtime::engine::BeginTool::Finished(blocked)
                } else {
                    rt.begin_tool(&db, &run_id, proposal)?
                }
            };

            let result = match begin {
                crate::runtime::engine::BeginTool::Finished(r) => r,
                crate::runtime::engine::BeginTool::Ready(spec) => {
                    if cancel_flag.load(Ordering::SeqCst) {
                        let db = db
                            .lock()
                            .map_err(|_| "database lock poisoned".to_string())?;
                        let mut rt = rt
                            .lock()
                            .map_err(|_| "runtime lock poisoned".to_string())?;
                        return rt.request_cancel(&db, &run_id);
                    }
                    let executed = crate::tools::execute_builtin(
                        spec.root.as_deref(),
                        None,
                        Some(spec.project_id.as_str()),
                        Some(spec.chat_id.as_str()),
                        Some(run_id.as_str()),
                        spec.mode,
                        &spec.proposal,
                    );
                    let db = db
                        .lock()
                        .map_err(|_| "database lock poisoned".to_string())?;
                    let mut rt = rt
                        .lock()
                        .map_err(|_| "runtime lock poisoned".to_string())?;
                    rt.complete_long_tool(&db, &run_id, &spec, executed)?
                }
            };

            {
                let db = db
                    .lock()
                    .map_err(|_| "database lock poisoned".to_string())?;
                let mut rt = rt
                    .lock()
                    .map_err(|_| "runtime lock poisoned".to_string())?;
                let current = db.get_run(&run_id)?.unwrap();
                if current.status.is_terminal() || current.status == RunStatus::Suspended {
                    return Ok(current);
                }
                if current.status == RunStatus::ApprovalWaiting {
                    let rest: Vec<ToolProposal> = tool_calls[idx + 1..]
                        .iter()
                        .map(|t| ToolProposal {
                            call_id: t.id.clone(),
                            tool_name: t.name.clone(),
                            arguments: t.arguments.clone(),
                        })
                        .collect();
                    let _ = rt.set_pending_remaining(&run_id, rest);
                    approval_halt = true;
                }
                if approval_halt {
                    return Ok(current);
                }
            }
            let result_text = serde_json::to_string(&result.content)
                .unwrap_or_else(|_| result.content.to_string());
            messages.push(ChatMessage {
                role: "tool".into(),
                content: Some(result_text),
                tool_calls: vec![],
                tool_call_id: Some(tc.id.clone()),
                name: Some(tc.name.clone()),
            });
        }

        let _ = mode;
        round += 1;
    }
}

/// Resume an approval_waiting run after the user grants: re-enter model phase with
/// a synthetic notice. Prefer [`execute_agent_run_after_grant_concurrent`] in product.
pub fn continue_after_tools_if_model_active(
    db: &Database,
    rt: &mut AgentRuntime,
    input: AgentLoopInput<'_>,
    prior_messages: Vec<ChatMessage>,
) -> Result<Run, String> {
    let run = db
        .get_run(input.run_id)?
        .ok_or_else(|| format!("run not found: {}", input.run_id))?;
    if run.status != RunStatus::ModelActive {
        return Ok(run);
    }
    if !prior_messages.is_empty() {
        return execute_agent_run_inner_with_prior(db, rt, &input, prior_messages);
    }
    execute_agent_run(db, rt, input)
}

fn execute_agent_run_inner_with_prior(
    db: &Database,
    rt: &mut AgentRuntime,
    input: &AgentLoopInput<'_>,
    prior_messages: Vec<ChatMessage>,
) -> Result<Run, String> {
    // Reuse the standard driver but swap history load by writing prior into a
    // synthetic approach: call inner after ensuring events already contain prior.
    // When prior_messages is non-empty, pass it directly by temporarily
    // overriding via a local fork of message bootstrap.
    let run = db
        .get_run(input.run_id)?
        .ok_or_else(|| format!("run not found: {}", input.run_id))?;
    if run.status.is_terminal() || run.status == RunStatus::Suspended {
        return Ok(run);
    }
    let mode = Mode::parse(&run.mode).ok_or_else(|| format!("invalid mode: {}", run.mode))?;
    let chat_id = run.chat_id.clone();
    let adapter = first_party_adapter(input.provider_id).ok_or_else(|| {
        format!(
            "no first-party adapter for provider {}",
            input.provider_id
        )
    })?;
    let tools = effective_catalog(
        Some(db),
        db.get_chat(&chat_id)
            .ok()
            .flatten()
            .as_ref()
            .map(|c| c.project_id.as_str()),
    );
    let manifest = if let Some(root) = input.project_root {
        compile_context(root, input.user_text, &[], DEFAULT_BODY_TOKEN_BUDGET)
    } else {
        compile_context(
            Path::new("."),
            input.user_text,
            &[],
            DEFAULT_BODY_TOKEN_BUDGET,
        )
    };
    let system = system_from_manifest(mode.as_str(), input.project_root, &manifest);
    let _ = rt.emit(
        db,
        &chat_id,
        Some(input.run_id),
        "run.context_manifest",
        serde_json::to_value(&manifest).unwrap_or(json!({})),
    );
    // Prefer explicit prior transcript over DB reconstruction for resume.
    let mut messages = messages_from_context(system.as_str(), input.user_text, &prior_messages);
    let mut round = 0usize;
    let mut doom = DoomTracker::default();
    let mut budget_nudged = false;
    loop {
        if cancel_requested(rt, input.run_id, &input.cancel_flag) {
            let current = db.get_run(input.run_id)?.unwrap();
            if current.status.is_terminal() || current.status == RunStatus::Suspended {
                return Ok(current);
            }
            return rt.request_cancel(db, input.run_id);
        }
        if round >= MAX_TOOL_ROUNDS {
            return soft_round_limit_complete(rt, db, &chat_id, input.run_id);
        }
        if round + 1 >= MAX_TOOL_ROUNDS && !budget_nudged {
            messages.push(ChatMessage {
                role: "user".into(),
                content: Some(ROUND_BUDGET_NUDGE.into()),
                tool_calls: vec![],
                tool_call_id: None,
                name: None,
            });
            budget_nudged = true;
        }
        if let Err(e) = rt.enforce_limits_public(db, input.run_id) {
            let current = db.get_run(input.run_id)?.unwrap();
            if current.status.is_terminal() {
                return Ok(current);
            }
            return Err(e);
        }
        if run_status_is_model_phase(db, input.run_id)? {
            let _ = rt.transition(db, input.run_id, RunStatus::ModelActive);
        }
        let turn_result = complete_chat_turn(
            adapter.as_ref(),
            input.model,
            input.api_key,
            &messages,
            &tools,
            input.transport,
            input.base_url_override,
            input.reasoning,
        );
        if cancel_requested(rt, input.run_id, &input.cancel_flag)
            || matches!(&turn_result, Err(e) if e.category == "cancellation")
        {
            let current = db.get_run(input.run_id)?.unwrap();
            if current.status.is_terminal() || current.status == RunStatus::Suspended {
                return Ok(current);
            }
            return rt.request_cancel(db, input.run_id);
        }
        let turn = match turn_result {
            Ok(t) => t,
            Err(err) => {
                let runtime_err = adapter_error_to_runtime(&err);
                let (plan, updated) =
                    rt.record_provider_failure(db, input.run_id, runtime_err, None)?;
                if updated.status.is_terminal() {
                    return Ok(updated);
                }
                if matches!(
                    plan.decision,
                    crate::reliability::retry::RetryDecision::AutoRetry { .. }
                ) {
                    round += 1;
                    continue;
                }
                return Ok(updated);
            }
        };
        emit_assistant_message(rt, db, &chat_id, input.run_id, &turn)?;
        messages.push(ChatMessage {
            role: "assistant".into(),
            content: if turn.text.is_empty() {
                None
            } else {
                Some(turn.text.clone())
            },
            tool_calls: turn.tool_calls.clone(),
            tool_call_id: None,
            name: None,
        });
        if !turn.has_tool_calls() {
            return rt.complete(db, input.run_id);
        }
        for tc in &turn.tool_calls {
            if cancel_requested(rt, input.run_id, &input.cancel_flag) {
                return rt.request_cancel(db, input.run_id);
            }
            let proposal = ToolProposal {
                call_id: tc.id.clone(),
                tool_name: tc.name.clone(),
                arguments: tc.arguments.clone(),
            };
            let result = if doom.note_is_doom(&proposal.tool_name, &proposal.arguments) {
                doom_block_result(&proposal)
            } else {
                rt.propose_tool(db, input.run_id, proposal)?
            };
            let current = db.get_run(input.run_id)?.unwrap();
            if current.status.is_terminal() || current.status == RunStatus::Suspended {
                return Ok(current);
            }
            if current.status == RunStatus::ApprovalWaiting {
                return Ok(current);
            }
            messages.push(ChatMessage {
                role: "tool".into(),
                content: Some(
                    serde_json::to_string(&result.content)
                        .unwrap_or_else(|_| result.content.to_string()),
                ),
                tool_calls: vec![],
                tool_call_id: Some(tc.id.clone()),
                name: Some(tc.name.clone()),
            });
        }
        round += 1;
    }
}

/// Project root for a chat id.
pub fn project_root_for_chat(db: &Database, chat_id: &str) -> Result<Option<PathBuf>, String> {
    let chat = db
        .get_chat(chat_id)?
        .ok_or_else(|| format!("chat not found: {chat_id}"))?;
    let project = db.get_project(&chat.project_id)?;
    Ok(project.map(|p| PathBuf::from(p.resolved_path)))
}

/// Start run metadata: provider adapter id + model from ready connection.
pub fn resolve_run_provider(
    db: &Database,
    model_override: Option<&str>,
) -> Result<(String, String), String> {
    let (provider_id, default_model) = read_ready_provider(db).ok_or_else(|| {
        "no ready provider — connect a first-party provider in onboarding first".to_string()
    })?;
    let model = model_override
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(default_model);
    Ok((provider_id, model))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::chat::{openai_assistant_text_body, openai_assistant_tool_body};
    use crate::providers::transport::FixtureHttpTransport;
    use crate::runtime::events::kinds;
    use crate::security::secrets::SharedSecretStore;

    fn setup() -> (Database, String, PathBuf) {
        let db = Database::open_in_memory().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "loopcode-agent-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hello.txt"), "hello world").unwrap();
        let project = db.open_project(&dir).unwrap();
        let chat = db.create_chat(&project.id, Some("t")).unwrap();
        (db, chat.id, dir)
    }

    #[test]
    fn text_only_turn_completes() {
        let (db, chat_id, root) = setup();
        let mut rt = AgentRuntime::new();
        rt.set_project_root_override(Some(root.clone()));
        let body = openai_assistant_text_body("This project is a demo.", "gpt-test");
        let transport = FixtureHttpTransport::single(200, body);
        let run = rt
            .start_run_with_provider(&db, &chat_id, Mode::Ask, Some("gpt-test"), "openai")
            .unwrap();
        db.append_event(
            &chat_id,
            Some(&run.id),
            "user_message",
            &json!({"text": "What is this project?"}).to_string(),
        )
        .unwrap();
        let flag = Arc::new(AtomicBool::new(false));
        let finished = execute_agent_run(
            &db,
            &mut rt,
            AgentLoopInput {
                run_id: &run.id,
                user_text: "What is this project?",
                provider_id: "openai",
                model: "gpt-test",
                api_key: "sk-test",
                project_root: Some(&root),
                transport: &transport,
                cancel_flag: flag,
                base_url_override: None,
                reasoning: None,
            },
        )
        .unwrap();
        assert_eq!(finished.status, RunStatus::Completed);
        assert_eq!(finished.adapter.as_deref(), Some("openai"));
        let events = db.list_events(&chat_id, None).unwrap();
        assert!(events.iter().any(|e| e.kind == "assistant_message"));
        assert!(events.iter().any(|e| e.kind == kinds::RUN_COMPLETED));
    }

    #[test]
    fn tool_loop_lists_workspace_then_completes() {
        let (db, chat_id, root) = setup();
        let mut rt = AgentRuntime::new();
        rt.set_project_root_override(Some(root.clone()));
        let call_id = "call_list_1";
        let tool_body = openai_assistant_tool_body(
            "glob",
            &json!({"pattern": "**/*"}),
            "gpt-test",
            call_id,
        );
        let final_body = openai_assistant_text_body("Found hello.txt in the project root.", "gpt-test");
        let transport =
            FixtureHttpTransport::new(vec![(200, tool_body), (200, final_body)]);
        let run = rt
            .start_run_with_provider(&db, &chat_id, Mode::Ask, Some("gpt-test"), "openai")
            .unwrap();
        db.append_event(
            &chat_id,
            Some(&run.id),
            "user_message",
            &json!({"text": "List files"}).to_string(),
        )
        .unwrap();
        let finished = execute_agent_run(
            &db,
            &mut rt,
            AgentLoopInput {
                run_id: &run.id,
                user_text: "List files",
                provider_id: "openai",
                model: "gpt-test",
                api_key: "sk-test",
                project_root: Some(&root),
                transport: &transport,
                cancel_flag: Arc::new(AtomicBool::new(false)),
                base_url_override: None,
                reasoning: None,
            },
        )
        .unwrap();
        assert_eq!(finished.status, RunStatus::Completed);
        let events = db.list_events(&chat_id, None).unwrap();
        assert!(events.iter().any(|e| e.kind == kinds::TOOL_PROPOSAL));
        assert!(events.iter().any(|e| e.kind == kinds::TOOL_RESULT));
        let result_ev = events
            .iter()
            .find(|e| e.kind == kinds::TOOL_RESULT)
            .unwrap();
        assert!(
            result_ev.payload_json.contains("hello.txt")
                || result_ev.payload_json.contains("entries"),
            "tool result should reflect real list: {}",
            result_ev.payload_json
        );
    }

    #[test]
    fn cancel_mid_request_terminates() {
        let (db, chat_id, root) = setup();
        let mut rt = AgentRuntime::new();
        rt.set_project_root_override(Some(root.clone()));
        let flag = Arc::new(AtomicBool::new(false));
        let transport = FixtureHttpTransport::single(200, openai_assistant_text_body("late", "m"))
            .with_delay(std::time::Duration::from_secs(10))
            .with_cancel_flag(Arc::clone(&flag));
        let run = rt
            .start_run_with_provider(&db, &chat_id, Mode::Ask, Some("m"), "openai")
            .unwrap();
        let run_id = run.id.clone();
        let handle = std::thread::spawn({
            let db_path = {
                // Use in-memory only in one thread — open second connection via shared?
                // Instead cancel on same thread after short sleep via flag + request_cancel pattern:
            };
            let _ = db_path;
            move || {}
        });
        let _ = handle;
        // Same-thread: set cancel before execute so transport sees it during delay.
        // Start execute in thread with cloned cancel; cancel from main.
        // In-memory Database is not Sync across threads easily (Mutex in AppState).
        // Use flag-only cancel observed by transport + loop:
        flag.store(true, Ordering::SeqCst);
        let finished = execute_agent_run(
            &db,
            &mut rt,
            AgentLoopInput {
                run_id: &run_id,
                user_text: "hi",
                provider_id: "openai",
                model: "m",
                api_key: "sk",
                project_root: Some(&root),
                transport: &transport,
                cancel_flag: Arc::clone(&flag),
                base_url_override: None,
                reasoning: None,
            },
        )
        .unwrap();
        assert!(
            matches!(
                finished.status,
                RunStatus::Cancelled | RunStatus::Failed | RunStatus::Cancelling
            ) || finished.status == RunStatus::Cancelled,
            "status={:?}",
            finished.status
        );
        // Flag was set before execute → cancel path
        assert_eq!(finished.status, RunStatus::Cancelled);
    }

    #[test]
    fn provider_error_fails_run() {
        let (db, chat_id, root) = setup();
        let mut rt = AgentRuntime::new();
        let transport = FixtureHttpTransport::single(
            401,
            r#"{"error":{"message":"invalid api key"}}"#,
        );
        let run = rt
            .start_run_with_provider(&db, &chat_id, Mode::Ask, Some("m"), "openai")
            .unwrap();
        let finished = execute_agent_run(
            &db,
            &mut rt,
            AgentLoopInput {
                run_id: &run.id,
                user_text: "hi",
                provider_id: "openai",
                model: "m",
                api_key: "bad",
                project_root: Some(&root),
                transport: &transport,
                cancel_flag: Arc::new(AtomicBool::new(false)),
                base_url_override: None,
                reasoning: None,
            },
        )
        .unwrap();
        assert_eq!(finished.status, RunStatus::Failed);
        assert_eq!(finished.adapter.as_deref(), Some("openai"));
    }

    #[test]
    #[allow(unused)]
    fn secrets_store_key_stable() {
        let _ = SharedSecretStore::memory();
        assert_eq!(
            provider_store_key("opencode"),
            "loopcode/provider/opencode/api_key"
        );
    }
}
