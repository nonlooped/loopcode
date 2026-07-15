//! Tauri IPC commands — Core owns SQLite; WebView only receives DTOs.

use crate::cockpit::{default_keyboard_map, project_timeline, KeyBinding, TimelineProjection};
use crate::db::Database;
use crate::domain::{
    derive_chat_title, Chat, HealthStatus, Project, Run, RunStatus, DEFAULT_CHAT_TITLE,
};
use crate::health_status;
use crate::providers::catalog::CatalogProvider;
use crate::providers::{
    connect_first_party, create_custom_profile, get_ready_provider, list_catalog_cards,
    list_hero_cards, set_ready_provider_selection, ConnectResult, HeroCard,
};
use crate::runtime::{AgentRuntime, Mode};
use crate::security::approval::GrantScope;
use crate::security::{redact_payload_json, ActionClass, SharedSecretStore};
use crate::surfaces::checkpoint::{
    chat_event_cursor, create_checkpoint, default_checkpoint_store, load_manifest_for_project,
    CheckpointManifest,
};
use crate::surfaces::diff::{project_diffs, BufferSnapshot, DiffModel, DiffScope};
use crate::surfaces::explorer::{list_project_tree, open_path_external, open_url_external, TreeEntry};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub db: Mutex<Database>,
    pub runtime: Mutex<AgentRuntime>,
    pub secrets: SharedSecretStore,
    /// Per-run cooperative cancel flags shared with in-flight HTTP transports.
    pub run_cancel_flags:
        Mutex<std::collections::HashMap<String, std::sync::Arc<std::sync::atomic::AtomicBool>>>,
}

#[tauri::command]
pub fn health(state: State<'_, AppState>) -> Result<HealthStatus, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let mut status = health_status();
    status.schema_version = Some(db.schema_version()?);
    status.db_path = Some(db.path().display().to_string());
    status.journal_mode = Some(db.journal_mode()?);
    Ok(status)
}

#[tauri::command]
pub fn open_project(state: State<'_, AppState>, path: String) -> Result<Project, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db.open_project(path)
}

#[tauri::command]
pub fn rebind_project(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<Project, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db.rebind_project(&project_id, path)
}

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db.refresh_orphan_flags()?;
    db.list_projects()
}

#[tauri::command]
pub fn delete_project_record(state: State<'_, AppState>, project_id: String) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db.delete_project_record(&project_id)
}

#[tauri::command]
pub fn create_chat(
    state: State<'_, AppState>,
    project_id: String,
    title: Option<String>,
) -> Result<Chat, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db.create_chat(&project_id, title.as_deref())
}

#[tauri::command]
pub fn list_chats(
    state: State<'_, AppState>,
    project_id: String,
    include_archived: Option<bool>,
) -> Result<Vec<Chat>, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db.list_chats(&project_id, include_archived.unwrap_or(false))
}

#[tauri::command]
pub fn get_chat(state: State<'_, AppState>, chat_id: String) -> Result<Option<Chat>, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db.get_chat(&chat_id)
}

#[tauri::command]
pub fn update_chat(
    state: State<'_, AppState>,
    chat_id: String,
    title: Option<String>,
    archived: Option<bool>,
) -> Result<Chat, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db.update_chat(&chat_id, title.as_deref(), archived)
}

#[tauri::command]
pub fn delete_chat(state: State<'_, AppState>, chat_id: String) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db.delete_chat(&chat_id)
}

#[tauri::command]
pub fn runtime_cancel_run(state: State<'_, AppState>, run_id: String) -> Result<Run, String> {
    use crate::runtime::agent_loop::product_request_cancel;
    // Cooperative flag first (works even while agent holds transport I/O without locks),
    // then short-lock request_cancel — concurrent agent loop releases locks around HTTP.
    product_request_cancel(&state.db, &state.runtime, &state.run_cancel_flags, &run_id)
}

// --- Providers / onboarding ---

#[tauri::command]
pub fn onboarding_list_heroes(state: State<'_, AppState>) -> Result<Vec<HeroCard>, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    list_hero_cards(Some(&state.secrets), Some(&db))
}

#[tauri::command]
pub fn onboarding_list_catalog() -> Result<Vec<CatalogProvider>, String> {
    list_catalog_cards()
}

/// Connect a provider with a live Core-side probe. Fixture probes are test-only.
#[tauri::command]
pub fn onboarding_connect_provider(
    state: State<'_, AppState>,
    provider_id: String,
    api_key: String,
) -> Result<ConnectResult, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    connect_first_party(
        &db,
        &state.secrets,
        &provider_id,
        &api_key,
        None,
    )
}

#[tauri::command]
pub fn onboarding_create_custom_profile(
    state: State<'_, AppState>,
    display_name: String,
    protocol: String,
    base_url: String,
    model: Option<String>,
) -> Result<crate::providers::connection::ValueDto, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    create_custom_profile(&db, &display_name, &protocol, &base_url, model.as_deref())
}

#[tauri::command]
pub fn onboarding_get_ready_provider(state: State<'_, AppState>) -> Result<Option<Value>, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    get_ready_provider(&db)
}

/// Persist cockpit provider/model selection across restarts (updates `provider.ready`).
#[tauri::command]
pub fn cockpit_set_provider_selection(
    state: State<'_, AppState>,
    provider_id: String,
    model: String,
) -> Result<Value, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    set_ready_provider_selection(&db, &state.secrets, &provider_id, &model)
}

// --- Cockpit ---

/// Project Core events into timeline UI DTOs.
#[tauri::command]
pub fn cockpit_timeline(
    state: State<'_, AppState>,
    chat_id: Option<String>,
) -> Result<TimelineProjection, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    match chat_id {
        None => Ok(project_timeline(&[], &[], false, false, None)),
        Some(cid) if cid.is_empty() => Ok(project_timeline(&[], &[], false, false, None)),
        Some(cid) => {
            let events = db.list_events(&cid, None)?;
            let runs = db.list_runs(&cid)?;
            Ok(project_timeline(&events, &runs, true, false, None))
        }
    }
}

/// Grant approval from the composer approval notice (deny / once / this_chat).
/// If the run is waiting for approval, this also transitions it and resumes
/// the agent loop so the granted tool can execute.
#[tauri::command]
pub fn cockpit_grant_approval(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    chat_id: String,
    run_id: Option<String>,
    action_class: String,
    scope: String,
) -> Result<(), String> {
    use crate::providers::transport::LiveHttpTransport;
    use crate::runtime::agent_loop::{
        execute_agent_run_after_grant_concurrent, project_root_for_chat, provider_store_key,
        AgentLoopOwned,
    };
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use tauri::Manager;

    let class = ActionClass::parse(&action_class)
        .ok_or_else(|| format!("unknown action_class: {action_class}"))?;
    let scope = match scope.as_str() {
        "deny" => GrantScope::Deny,
        "once" | "allow_once" => GrantScope::AllowOnce,
        "this_chat" | "allow_matching_for_chat" => GrantScope::AllowMatchingForChat,
        other => return Err(format!("unknown grant scope: {other}")),
    };

    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let chat = db
        .get_chat(&chat_id)?
        .ok_or_else(|| format!("chat not found: {chat_id}"))?;
    let mut rt = state
        .runtime
        .lock()
        .map_err(|_| "runtime lock poisoned".to_string())?;
    rt.grant_approval(
        &db,
        &chat_id,
        run_id.as_deref(),
        Some(&chat.project_id),
        class,
        scope,
    )?;

    // Resume only when a specific run is stalled waiting for this grant.
    let Some(run_id) = run_id else {
        return Ok(());
    };
    let run = db
        .get_run(&run_id)?
        .ok_or_else(|| format!("run not found: {run_id}"))?;
    if run.status != RunStatus::ApprovalWaiting {
        return Ok(());
    }

    if scope == GrantScope::Deny {
        let err = crate::runtime::RuntimeError::new(
            crate::runtime::ErrorCategory::PolicyDenial,
            crate::runtime::ErrorOrigin::User,
            crate::runtime::ErrorStage::Approval,
            crate::runtime::SideEffectCertainty::None,
            "approval denied",
        );
        let _ = rt.fail(&db, &run_id, err);
        return Ok(());
    }

    // Allow grants: execute the paused tool under the grant, then continue the
    // agent loop with same-run history (no amnesiac re-propose).
    let user_event = db
        .list_events(&chat_id, None)?
        .into_iter()
        .find(|e| e.run_id.as_deref() == Some(&run_id) && e.kind == "user_message")
        .and_then(|e| serde_json::from_str::<serde_json::Value>(&e.payload_json).ok());
    let user_text = user_event
        .as_ref()
        .and_then(|v| v.get("text")?.as_str().map(String::from))
        .unwrap_or_default();
    // Restore the composer's reasoning selection for the resumed loop.
    let reasoning = user_event
        .as_ref()
        .and_then(|v| v.get("reasoning"))
        .and_then(|r| {
            serde_json::from_value::<crate::providers::chat::ReasoningConfig>(r.clone()).ok()
        });

    let provider_id = run.adapter.clone().unwrap_or_default();
    let model = run.model.clone().unwrap_or_default();
    if provider_id.is_empty() || model.is_empty() {
        return Err("run is missing provider adapter or model".into());
    }
    let api_key = state
        .secrets
        .get(&provider_store_key(&provider_id))?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| {
            format!("no API key stored for provider {provider_id}; reconnect in onboarding")
        })?;
    let project_root = project_root_for_chat(&db, &chat_id)?;
    let cancel_flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut map) = state.run_cancel_flags.lock() {
        map.insert(run_id.clone(), Arc::clone(&cancel_flag));
    }

    let owned = AgentLoopOwned {
        run_id: run_id.clone(),
        user_text,
        provider_id,
        model,
        api_key,
        project_root,
        cancel_flag: Arc::clone(&cancel_flag),
        base_url_override: None,
        reasoning,
    };
    let run_id_for_cleanup = run_id.clone();

    // Release host locks before spawning the background turn.
    drop(db);
    drop(rt);

    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let clear_flag = |s: &AppState, id: &str| {
            if let Ok(mut map) = s.run_cancel_flags.lock() {
                map.remove(id);
            }
        };
        let transport = match LiveHttpTransport::with_cancel(Arc::clone(&owned.cancel_flag)) {
            Ok(t) => t,
            Err(e) => {
                if let (Ok(db), Ok(mut rt)) = (state.db.lock(), state.runtime.lock()) {
                    let err = crate::runtime::RuntimeError::new(
                        crate::runtime::ErrorCategory::Network,
                        crate::runtime::ErrorOrigin::Provider,
                        crate::runtime::ErrorStage::Model,
                        crate::runtime::SideEffectCertainty::None,
                        e,
                    );
                    let _ = rt.fail(&db, &owned.run_id, err);
                }
                clear_flag(&state, &run_id_for_cleanup);
                return;
            }
        };
        let _ = execute_agent_run_after_grant_concurrent(
            &state.db,
            &state.runtime,
            owned,
            &transport,
        );
        clear_flag(&state, &run_id_for_cleanup);
    });

    Ok(())
}

/// Shipped keyboard map for the cockpit.
#[tauri::command]
pub fn cockpit_keyboard_map() -> Vec<KeyBinding> {
    default_keyboard_map()
}

/// Composer send: start run + append user message, return **immediately** with
/// `model_active` so the UI can set `activeRunId` and offer Stop. The agent loop
/// continues on a background thread with short DB/runtime lock scopes around I/O.
#[tauri::command]
pub fn cockpit_send(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    chat_id: String,
    text: String,
    mode: String,
    model: Option<String>,
    reasoning: Option<crate::providers::chat::ReasoningConfig>,
) -> Result<Run, String> {
    use crate::providers::transport::LiveHttpTransport;
    use crate::runtime::agent_loop::{
        execute_agent_run_concurrent, project_root_for_chat, provider_store_key,
        resolve_run_provider, AgentLoopOwned,
    };
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use tauri::Manager;

    let mode = Mode::parse(&mode).ok_or_else(|| format!("invalid mode: {mode}"))?;
    let (provider_id, resolved_model) = {
        let db = state
            .db
            .lock()
            .map_err(|_| "database lock poisoned".to_string())?;
        resolve_run_provider(&db, model.as_deref())?
    };

    let api_key = state
        .secrets
        .get(&provider_store_key(&provider_id))?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| {
            format!("no API key stored for provider {provider_id}; reconnect in onboarding")
        })?;

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let project_root = {
        let db = state
            .db
            .lock()
            .map_err(|_| "database lock poisoned".to_string())?;
        project_root_for_chat(&db, &chat_id)?
    };

    let run = {
        let db = state
            .db
            .lock()
            .map_err(|_| "database lock poisoned".to_string())?;
        let mut rt = state
            .runtime
            .lock()
            .map_err(|_| "runtime lock poisoned".to_string())?;
        let run =
            rt.start_run_with_provider(&db, &chat_id, mode, Some(&resolved_model), &provider_id)?;
        // Reasoning rides in the durable payload so approval-resume can restore it.
        let payload = serde_json::json!({
            "text": text,
            "mode": mode.as_str(),
            "providerId": provider_id,
            "model": resolved_model,
            "reasoning": reasoning,
        });
        db.append_event(
            &chat_id,
            Some(&run.id),
            "user_message",
            &redact_payload_json(&payload.to_string()),
        )?;
        // Auto-title the chat from the first user message when it still has the
        // default placeholder. Skips chats the user (or a template) already named.
        if let Some(chat) = db.get_chat(&chat_id)? {
            if chat.title == DEFAULT_CHAT_TITLE {
                let derived = derive_chat_title(&text);
                db.update_chat(&chat_id, Some(&derived), None)?;
            }
        }
        if let Ok(mut map) = state.run_cancel_flags.lock() {
            map.insert(run.id.clone(), Arc::clone(&cancel_flag));
        }
        run
    };

    let owned = AgentLoopOwned {
        run_id: run.id.clone(),
        user_text: text,
        provider_id,
        model: resolved_model,
        api_key,
        project_root,
        cancel_flag: Arc::clone(&cancel_flag),
        base_url_override: None,
        reasoning,
    };
    let run_id_for_cleanup = run.id.clone();

    // Background turn: do not hold the Tauri command or AppState locks for HTTPS.
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let clear_flag = |s: &AppState, id: &str| {
            if let Ok(mut map) = s.run_cancel_flags.lock() {
                map.remove(id);
            }
        };
        let transport = match LiveHttpTransport::with_cancel(Arc::clone(&owned.cancel_flag)) {
            Ok(t) => t,
            Err(e) => {
                // Fail the run so it is not stuck model_active forever.
                if let (Ok(db), Ok(mut rt)) = (state.db.lock(), state.runtime.lock()) {
                    let err = crate::runtime::RuntimeError::new(
                        crate::runtime::ErrorCategory::Network,
                        crate::runtime::ErrorOrigin::Provider,
                        crate::runtime::ErrorStage::Model,
                        crate::runtime::SideEffectCertainty::None,
                        e,
                    );
                    let _ = rt.fail(&db, &owned.run_id, err);
                }
                clear_flag(&state, &run_id_for_cleanup);
                return;
            }
        };
        let _ = execute_agent_run_concurrent(&state.db, &state.runtime, owned, &transport);
        clear_flag(&state, &run_id_for_cleanup);
    });

    // Return immediately while run is model_active so the composer can Stop.
    Ok(run)
}

// --- Files / editor / diffs / checkpoints / terminal ---

fn project_root_for(db: &Database, project_id: &str) -> Result<PathBuf, String> {
    let p = db
        .get_project(project_id)?
        .ok_or_else(|| format!("project not found: {project_id}"))?;
    Ok(PathBuf::from(p.resolved_path))
}

#[tauri::command]
pub fn surfaces_list_tree(
    state: State<'_, AppState>,
    project_id: String,
    show_excluded: Option<bool>,
) -> Result<Vec<TreeEntry>, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let root = project_root_for(&db, &project_id)?;
    list_project_tree(&root, show_excluded.unwrap_or(false), None)
}

/// Open a project file in the OS default application (user's text editor).
#[tauri::command]
pub fn surfaces_open_external(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let root = project_root_for(&db, &project_id)?;
    open_path_external(&root, &path)
}

/// Open an http(s) URL in the OS default browser (no WebView opener permission).
#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    open_url_external(&url)
}

#[tauri::command]
pub fn surfaces_project_diffs(
    state: State<'_, AppState>,
    chat_id: String,
    scope: String,
    run_id: Option<String>,
    dirty_buffers: Option<Vec<BufferSnapshotDto>>,
    checkpoint_id: Option<String>,
    project_id: Option<String>,
) -> Result<DiffModel, String> {
    let scope = match scope.as_str() {
        "this_run" | "turn" => DiffScope::ThisRun,
        "since_chat_start" | "cumulative" => DiffScope::SinceChatStart,
        "buffer_vs_disk" | "unsaved" => DiffScope::BufferVsDisk,
        "vs_checkpoint" => DiffScope::VsCheckpoint,
        other => return Err(format!("unknown diff scope: {other}")),
    };
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let events = db.list_events(&chat_id, None)?;
    // Honor conversation soft-hide cursor
    let events = if let Some(cur) = chat_event_cursor(&db, &chat_id)? {
        events.into_iter().filter(|e| e.seq <= cur).collect()
    } else {
        events
    };
    let root = match project_id.as_deref() {
        Some(pid) => Some(project_root_for(&db, pid)?),
        None => None,
    };
    let buffers: Vec<BufferSnapshot> = dirty_buffers
        .unwrap_or_default()
        .into_iter()
        .map(|b| BufferSnapshot {
            path: b.path,
            content: b.content,
            dirty: b.dirty,
        })
        .collect();
    let checkpoint = match (project_id.as_deref(), checkpoint_id) {
        (Some(pid), Some(path)) => {
            let data = crate::db::store::default_data_dir()?;
            Some(load_manifest_for_project(&data, pid, &PathBuf::from(path))?.0)
        }
        _ => None,
    };
    Ok(project_diffs(
        scope,
        &events,
        root.as_deref(),
        run_id.as_deref(),
        &buffers,
        checkpoint.as_ref(),
    ))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BufferSnapshotDto {
    pub path: String,
    pub content: String,
    pub dirty: bool,
}

#[tauri::command]
pub fn surfaces_create_checkpoint(
    state: State<'_, AppState>,
    project_id: String,
    chat_id: Option<String>,
    run_id: Option<String>,
) -> Result<CheckpointManifest, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let root = project_root_for(&db, &project_id)?;
    let data = crate::db::store::default_data_dir()?;
    let store = default_checkpoint_store(&data, &project_id);
    let cursor = match &chat_id {
        Some(cid) => db.list_events(cid, None)?.last().map(|e| e.seq),
        None => None,
    };
    create_checkpoint(
        &root,
        &store,
        &project_id,
        chat_id.as_deref(),
        run_id.as_deref(),
        cursor,
    )
}

// --- Skills / MCP ---

#[tauri::command]
pub fn ext_list_skills(state: State<'_, AppState>, project_id: String) -> Result<Value, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let root = project_root_for(&db, &project_id)?;
    crate::tools::skills::skills_list(&root, &serde_json::json!({}))
}

#[tauri::command]
pub fn ext_load_skill(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<Value, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let root = project_root_for(&db, &project_id)?;
    crate::tools::skills::skills_load(&root, &serde_json::json!({"path": path}))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRegisterDto {
    pub name: String,
    pub transport_type: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env_keys: Option<Vec<String>>,
    pub url: Option<String>,
    pub header_keys: Option<Vec<String>>,
    pub project_id: Option<String>,
    pub id: Option<String>,
}

#[tauri::command]
pub fn ext_mcp_register(
    state: State<'_, AppState>,
    config: McpRegisterDto,
) -> Result<crate::extensibility::McpServerConfig, String> {
    let transport = match config.transport_type.as_str() {
        "stdio" => crate::extensibility::McpTransport::Stdio {
            command: config.command.ok_or("command required for stdio")?,
            args: config.args.unwrap_or_default(),
            env_keys: config.env_keys.unwrap_or_default(),
        },
        "http" | "sse" | "remote" => crate::extensibility::McpTransport::Http {
            url: config.url.ok_or("url required for http")?,
            header_keys: config.header_keys.unwrap_or_default(),
        },
        other => return Err(format!("unknown transport: {other}")),
    };
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    crate::extensibility::register_mcp_server(
        &db,
        &config.name,
        transport,
        config.project_id.as_deref(),
        config.id.as_deref(),
    )
}

#[tauri::command]
pub fn ext_mcp_list(
    state: State<'_, AppState>,
) -> Result<Vec<crate::extensibility::McpServerConfig>, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    crate::extensibility::list_mcp_servers(&db)
}

#[tauri::command]
pub fn ext_mcp_set_enabled(
    state: State<'_, AppState>,
    server_id: String,
    enabled: bool,
) -> Result<crate::extensibility::McpServerConfig, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    crate::extensibility::set_mcp_server_enabled(&db, &server_id, enabled)
}

#[tauri::command]
pub fn ext_mcp_remove(state: State<'_, AppState>, server_id: String) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    crate::extensibility::remove_mcp_server(&db, &server_id)
}

#[tauri::command]
pub fn ext_mcp_grant_trust(
    state: State<'_, AppState>,
    server_id: String,
    project_id: Option<String>,
) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    crate::extensibility::grant_mcp_server_trust(&db, &server_id, project_id.as_deref())
}

// --- Reliability / updates ---

#[tauri::command]
pub fn reliability_integrity_check(
    state: State<'_, AppState>,
) -> Result<crate::reliability::IntegrityReport, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db.integrity_check()
}

#[tauri::command]
pub fn reliability_backup_db(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let data = crate::db::store::default_data_dir()?;
    let dir = crate::reliability::backup::default_backups_dir(&data);
    let dest = dir.join(format!(
        "backup-{}.sqlite",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    ));
    let path = db.backup_to(&dest)?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub fn reliability_update_check(state: State<'_, AppState>, enabled: bool) -> Result<String, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db.set_setting("user", "updates.enabled", if enabled { "true" } else { "false" })?;
    if !enabled {
        return Err("update check disabled — no network request was made".into());
    }
    // Do not claim a fixture is a release result. A real signed update channel
    // must be configured before this product surface performs network I/O.
    Err("update channel is not configured; no network request was made".into())
}

#[tauri::command]
pub fn reliability_settings(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let update_enabled = db
        .get_setting("user", "updates.enabled")?
        .map(|setting| setting.value_json == "true")
        .unwrap_or(false);
    Ok(serde_json::json!({
        "updateCheckDefault": update_enabled,
        "labels": ["suspended", "rate_limited", "timeout", "retryable", "cost_limit"],
        "toolTimeoutClasses": ["short_read", "medium_edit", "long_shell"],
    }))
}

// --- Accessibility / desktop integration ---

#[tauri::command]
pub fn a11y_focus_order() -> Vec<String> {
    crate::a11y::focus_order_regions()
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[tauri::command]
pub fn a11y_apply_theme(theme: String) -> Result<crate::a11y::theme::ThemeApplyResult, String> {
    let t = crate::a11y::parse_theme(&theme).ok_or_else(|| format!("unknown theme: {theme}"))?;
    Ok(crate::a11y::apply_theme_preference(t))
}

#[tauri::command]
pub fn a11y_single_instance_decision(
    lock_held_by_other: bool,
) -> crate::a11y::SingleInstanceDecision {
    crate::a11y::single_instance::decide_instance(lock_held_by_other)
}

#[tauri::command]
pub fn a11y_build_approval_notification(
    action_class: String,
    tool_name: Option<String>,
    chat_id: Option<String>,
    run_id: Option<String>,
    detail: Option<Value>,
) -> crate::a11y::ApprovalNotificationPayload {
    crate::a11y::build_approval_notification(
        &action_class,
        tool_name.as_deref(),
        chat_id.as_deref(),
        run_id.as_deref(),
        detail.as_ref(),
    )
}

#[tauri::command]
pub fn a11y_menu_actions() -> Vec<serde_json::Value> {
    // Native-style menu chrome → existing command handlers
    vec![
        json!({"menu": "File", "label": "New chat", "command": "new_chat", "chord": "Ctrl+N"}),
        json!({"menu": "File", "label": "Open folder…", "command": "open_folder", "chord": null}),
        json!({"menu": "Edit", "label": "Focus composer", "command": "focus_composer", "chord": "Ctrl+L"}),
        json!({"menu": "View", "label": "Toggle left", "command": "toggle_left", "chord": "Ctrl+B"}),
        json!({"menu": "View", "label": "Toggle right", "command": "toggle_right", "chord": "Ctrl+Alt+B"}),
        json!({"menu": "View", "label": "High contrast theme", "command": "theme_high_contrast", "chord": null}),
        json!({"menu": "View", "label": "Default theme", "command": "theme_default", "chord": null}),
        json!({"menu": "Help", "label": "Install docs", "command": "open_install_docs", "chord": null}),
        json!({"menu": "Help", "label": "Advanced settings", "command": "open_advanced", "chord": null}),
    ]
}

/// Clipboard access is Core-owned so the WebView does not receive direct OS
/// clipboard authority.
#[tauri::command]
pub fn clipboard_write_text(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("open clipboard: {e}"))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("write clipboard: {e}"))
}

#[tauri::command]
pub fn clipboard_read_text() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("open clipboard: {e}"))?;
    clipboard
        .get_text()
        .map_err(|e| format!("read clipboard: {e}"))
}
