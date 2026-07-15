//! MCP dual-gate: server trust + per-tool invocation approval.
//! Stdio and remote HTTP config models with real call-time transport.
//! Set `LOOPCODE_MCP_FIXTURE=1` to force echo fixtures (CI without process spawn).

use crate::db::store::Database;
use crate::security::approval::{approval_decision, ActionClass, ApprovalDecision};
use crate::security::trust::{self, content_hash, TrustKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;
use uuid::Uuid;

const SETTINGS_SCOPE: &str = "mcp";
const SETTINGS_KEY: &str = "servers";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum McpTransport {
    Stdio {
        command: String,
        args: Vec<String>,
        /// Env **keys** only (never secret values in config fingerprint surface).
        env_keys: Vec<String>,
    },
    Http {
        url: String,
        header_keys: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub transport: McpTransport,
    pub project_id: Option<String>,
    /// Whether the user has enabled this server for progressive disclosure (not trust).
    pub enabled: bool,
    pub fingerprint: String,
}

/// Stable fingerprint of command/args/URL/env keys — change resets trust.
pub fn config_fingerprint(transport: &McpTransport) -> String {
    let canonical = match transport {
        McpTransport::Stdio {
            command,
            args,
            env_keys,
        } => {
            let mut keys = env_keys.clone();
            keys.sort();
            format!("stdio|{command}|{}|{}", args.join("\u{1f}"), keys.join(","))
        }
        McpTransport::Http { url, header_keys } => {
            let mut keys = header_keys.clone();
            keys.sort();
            format!("http|{url}|{}", keys.join(","))
        }
    };
    content_hash(canonical.as_bytes())
}

fn load_all(db: &Database) -> Result<Vec<McpServerConfig>, String> {
    match db.get_setting(SETTINGS_SCOPE, SETTINGS_KEY)? {
        None => Ok(vec![]),
        Some(s) => serde_json::from_str(&s.value_json).map_err(|e| e.to_string()),
    }
}

fn save_all(db: &Database, servers: &[McpServerConfig]) -> Result<(), String> {
    let json = serde_json::to_string(servers).map_err(|e| e.to_string())?;
    db.set_setting(SETTINGS_SCOPE, SETTINGS_KEY, &json)?;
    Ok(())
}

/// Register or update a server. Fingerprint change invalidates prior server trust.
pub fn register_mcp_server(
    db: &Database,
    name: &str,
    transport: McpTransport,
    project_id: Option<&str>,
    id: Option<&str>,
) -> Result<McpServerConfig, String> {
    let fingerprint = config_fingerprint(&transport);
    let mut servers = load_all(db)?;
    let id = id.map(str::to_string).unwrap_or_else(|| Uuid::new_v4().to_string());

    if let Some(existing) = servers.iter().find(|s| s.id == id) {
        if existing.fingerprint != fingerprint {
            trust::invalidate_trust(
                db,
                TrustKind::McpServer,
                &id,
                project_id.or(existing.project_id.as_deref()),
            )?;
        }
    }

    let cfg = McpServerConfig {
        id: id.clone(),
        name: name.into(),
        transport,
        project_id: project_id.map(str::to_string),
        enabled: false, // not auto-started
        fingerprint: fingerprint.clone(),
    };

    if let Some(slot) = servers.iter_mut().find(|s| s.id == id) {
        *slot = cfg.clone();
    } else {
        servers.push(cfg.clone());
    }
    save_all(db, &servers)?;
    Ok(cfg)
}

pub fn list_mcp_servers(db: &Database) -> Result<Vec<McpServerConfig>, String> {
    load_all(db)
}

/// Toggle progressive-disclosure enabled flag (does not grant or revoke trust).
pub fn set_mcp_server_enabled(
    db: &Database,
    server_id: &str,
    enabled: bool,
) -> Result<McpServerConfig, String> {
    let mut servers = load_all(db)?;
    let cfg = servers
        .iter_mut()
        .find(|s| s.id == server_id)
        .ok_or_else(|| format!("unknown MCP server: {server_id}"))?;
    cfg.enabled = enabled;
    let out = cfg.clone();
    save_all(db, &servers)?;
    Ok(out)
}

/// Remove a server registration. Prior trust rows remain but become inert without a config.
pub fn remove_mcp_server(db: &Database, server_id: &str) -> Result<(), String> {
    let mut servers = load_all(db)?;
    let before = servers.len();
    servers.retain(|s| s.id != server_id);
    if servers.len() == before {
        return Err(format!("unknown MCP server: {server_id}"));
    }
    save_all(db, &servers)?;
    Ok(())
}

/// Gate 1: explicit server trust at the current fingerprint.
pub fn grant_mcp_server_trust(
    db: &Database,
    server_id: &str,
    project_id: Option<&str>,
) -> Result<(), String> {
    let servers = load_all(db)?;
    let cfg = servers
        .into_iter()
        .find(|s| s.id == server_id)
        .ok_or_else(|| format!("unknown MCP server: {server_id}"))?;
    let fp = config_fingerprint(&cfg.transport);
    if fp != cfg.fingerprint {
        return Err("server config fingerprint mismatch — re-register".into());
    }
    trust::grant_trust(
        db,
        TrustKind::McpServer,
        server_id,
        &fp,
        project_id.or(cfg.project_id.as_deref()),
    )?;
    // Mark enabled for progressive disclosure (still no auto-start of tools).
    let mut all = load_all(db)?;
    if let Some(s) = all.iter_mut().find(|s| s.id == server_id) {
        s.enabled = true;
        s.fingerprint = fp;
    }
    save_all(db, &all)?;
    Ok(())
}

pub fn is_mcp_server_trusted(
    db: &Database,
    server_id: &str,
    project_id: Option<&str>,
) -> Result<bool, String> {
    let servers = load_all(db)?;
    let Some(cfg) = servers.into_iter().find(|s| s.id == server_id) else {
        return Ok(false);
    };
    let fp = config_fingerprint(&cfg.transport);
    if fp != cfg.fingerprint {
        // Stale stored fingerprint vs live transport — treat untrusted
        return Ok(false);
    }
    trust::is_trusted(
        db,
        TrustKind::McpServer,
        server_id,
        &fp,
        project_id.or(cfg.project_id.as_deref()),
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCallResult {
    pub status: String,
    pub server_id: String,
    pub tool_name: String,
    pub server_trusted: bool,
    pub invocation_decision: String,
    pub action_class: String,
    pub content: Value,
    pub executed: bool,
}

/// Dual-gate MCP tool invocation.
/// Gate 1: server trusted at current fingerprint.
/// Gate 2: fixed matrix for ActionClass::Mcp (always RequireApproval unless caller grants once).
///
/// `invocation_granted` is true when the host already applied AllowOnce/ThisChat for MCP.
pub fn mcp_call(
    db: &Database,
    server_id: &str,
    tool_name: &str,
    arguments: &Value,
    project_id: Option<&str>,
    invocation_granted: bool,
) -> Result<McpCallResult, String> {
    let servers = load_all(db)?;
    let cfg = servers
        .into_iter()
        .find(|s| s.id == server_id)
        .ok_or_else(|| format!("unknown MCP server: {server_id}"))?;

    let fp = config_fingerprint(&cfg.transport);
    // Config change after trust: fingerprint mismatch → untrusted
    let server_trusted = if fp != cfg.fingerprint {
        false
    } else {
        trust::is_trusted(
            db,
            TrustKind::McpServer,
            server_id,
            &fp,
            project_id.or(cfg.project_id.as_deref()),
        )?
    };

    if !server_trusted {
        return Ok(McpCallResult {
            status: "require_server_trust".into(),
            server_id: server_id.into(),
            tool_name: tool_name.into(),
            server_trusted: false,
            invocation_decision: "blocked".into(),
            action_class: ActionClass::Mcp.as_str().into(),
            content: json!({
                "reason": "MCP server not trusted at current config fingerprint",
                "fingerprint": fp,
            }),
            executed: false,
        });
    }

    // Gate 2: per-tool invocation — connection does not blanket-allow.
    let decision = if invocation_granted {
        ApprovalDecision::Allow
    } else {
        approval_decision(ActionClass::Mcp)
    };

    if decision != ApprovalDecision::Allow {
        return Ok(McpCallResult {
            status: "require_approval".into(),
            server_id: server_id.into(),
            tool_name: tool_name.into(),
            server_trusted: true,
            invocation_decision: decision.as_str().into(),
            action_class: ActionClass::Mcp.as_str().into(),
            content: json!({
                "reason": "MCP tool invocation requires approval (not covered by server trust)",
                "tool": tool_name,
                "arguments": arguments,
            }),
            executed: false,
        });
    }

    // Optional CI fixture (explicit) — production always attempts real transport.
    if mcp_fixture_mode() {
        return Ok(McpCallResult {
            status: "ok".into(),
            server_id: server_id.into(),
            tool_name: tool_name.into(),
            server_trusted: true,
            invocation_decision: "allow".into(),
            action_class: ActionClass::Mcp.as_str().into(),
            content: json!({
                "fixture": true,
                "tool": tool_name,
                "echo": arguments,
                "transport": match &cfg.transport {
                    McpTransport::Stdio { command, .. } => format!("stdio:{command}"),
                    McpTransport::Http { url, .. } => format!("http:{url}"),
                },
                "note": "LOOPCODE_MCP_FIXTURE=1 — echo only",
            }),
            executed: true,
        });
    }

    let content = match invoke_mcp_transport(&cfg.transport, tool_name, arguments) {
        Ok(v) => v,
        Err(e) => {
            return Ok(McpCallResult {
                status: "transport_error".into(),
                server_id: server_id.into(),
                tool_name: tool_name.into(),
                server_trusted: true,
                invocation_decision: "allow".into(),
                action_class: ActionClass::Mcp.as_str().into(),
                content: json!({
                    "fixture": false,
                    "error": e,
                    "tool": tool_name,
                }),
                executed: false,
            });
        }
    };

    Ok(McpCallResult {
        status: "ok".into(),
        server_id: server_id.into(),
        tool_name: tool_name.into(),
        server_trusted: true,
        invocation_decision: "allow".into(),
        action_class: ActionClass::Mcp.as_str().into(),
        content,
        executed: true,
    })
}

fn mcp_fixture_mode() -> bool {
    matches!(
        std::env::var("LOOPCODE_MCP_FIXTURE").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// Invoke MCP tools/call over stdio or HTTP (call-time; servers are not auto-started at launch).
pub fn invoke_mcp_transport(
    transport: &McpTransport,
    tool_name: &str,
    arguments: &Value,
) -> Result<Value, String> {
    match transport {
        McpTransport::Stdio {
            command,
            args,
            env_keys: _,
        } => invoke_mcp_stdio(command, args, tool_name, arguments),
        McpTransport::Http {
            url,
            header_keys: _,
        } => invoke_mcp_http(url, tool_name, arguments),
    }
}

fn jsonrpc_tools_call(tool_name: &str, arguments: &Value, id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments,
        }
    })
}

fn jsonrpc_tools_list(id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/list",
        "params": {}
    })
}

/// Sanitize a fragment for OpenAI-safe tool names (`[a-zA-Z0-9_-]`, ≤64 total).
pub fn sanitize_mcp_name_fragment(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("tool");
    }
    out
}

/// Catalog name for a merged MCP tool: `mcp_{server}_{tool}` (truncated to 64).
pub fn mcp_catalog_tool_name(server_id: &str, tool_name: &str) -> String {
    let server = sanitize_mcp_name_fragment(server_id);
    let tool = sanitize_mcp_name_fragment(tool_name);
    let mut name = format!("mcp_{server}_{tool}");
    if name.len() > 64 {
        name.truncate(64);
    }
    name
}

/// List MCP tools from enabled servers and merge them into the agent catalog.
///
/// Discovery is best-effort: a failing server is skipped. Fixture mode returns a
/// single echo tool per enabled server so CI can exercise catalog merge.
pub fn list_mcp_catalog_tools(
    db: &Database,
    project_id: Option<&str>,
) -> Result<Vec<crate::runtime::tools::ToolDefinition>, String> {
    use crate::runtime::tools::ToolDefinition;

    let servers = load_all(db)?;
    let mut out = Vec::new();
    for cfg in servers {
        if !cfg.enabled {
            continue;
        }
        // Prefer project-scoped servers; also include global (no project) servers.
        if let Some(pid) = project_id {
            if let Some(ref sp) = cfg.project_id {
                if sp != pid {
                    continue;
                }
            }
        }

        let tools = if mcp_fixture_mode() {
            vec![json!({
                "name": "echo",
                "description": format!("Fixture echo tool for MCP server {}", cfg.name),
                "inputSchema": {
                    "type": "object",
                    "properties": { "input": { "type": "object" } }
                }
            })]
        } else {
            match list_mcp_tools_transport(&cfg.transport) {
                Ok(t) => t,
                Err(_) => continue,
            }
        };

        for t in tools {
            let tool_name = t
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("tool")
                .to_string();
            let description = t
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("MCP tool")
                .to_string();
            let schema = t
                .get("inputSchema")
                .or_else(|| t.get("input_schema"))
                .cloned();
            let catalog_name = mcp_catalog_tool_name(&cfg.id, &tool_name);
            out.push(ToolDefinition::mcp(
                catalog_name,
                cfg.id.clone(),
                tool_name,
                format!("[{}] {description}", cfg.name),
                schema,
            ));
        }
    }
    Ok(out)
}

fn list_mcp_tools_transport(transport: &McpTransport) -> Result<Vec<Value>, String> {
    match transport {
        McpTransport::Stdio {
            command,
            args,
            env_keys: _,
        } => list_mcp_tools_stdio(command, args),
        McpTransport::Http {
            url,
            header_keys: _,
        } => list_mcp_tools_http(url),
    }
}

fn list_mcp_tools_stdio(command: &str, args: &[String]) -> Result<Vec<Value>, String> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn MCP stdio `{command}`: {e}"))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "MCP stdio: no stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "MCP stdio: no stdout".to_string())?;
    let mut reader = BufReader::new(stdout);

    write_jsonrpc_line(&mut stdin, &jsonrpc_initialize(1))?;
    let _init = read_jsonrpc_line(&mut reader, Duration::from_secs(15))?;
    let notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    write_jsonrpc_line(&mut stdin, &notif)?;
    write_jsonrpc_line(&mut stdin, &jsonrpc_tools_list(2))?;
    let resp = read_jsonrpc_line(&mut reader, Duration::from_secs(30))?;

    let _ = child.kill();
    let _ = child.wait();

    if let Some(err) = resp.get("error") {
        return Err(format!("MCP tools/list error: {err}"));
    }
    let tools = resp
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(tools)
}

fn list_mcp_tools_http(url: &str) -> Result<Vec<Value>, String> {
    use crate::providers::adapter::HttpRequestSpec;
    use crate::providers::transport::{HttpTransport, LiveHttpTransport};
    use std::collections::HashMap;

    let transport = LiveHttpTransport::new()?;
    let body = jsonrpc_tools_list(1);
    let mut headers = HashMap::new();
    headers.insert("Content-Type".into(), "application/json".into());
    headers.insert("Accept".into(), "application/json".into());
    let req = HttpRequestSpec {
        method: "POST".into(),
        url: url.to_string(),
        headers,
        body,
    };
    let (status, raw) = transport.execute(&req)?;
    if !(200..300).contains(&status) {
        return Err(format!("MCP HTTP {status}: {}", raw.chars().take(200).collect::<String>()));
    }
    let resp: Value =
        serde_json::from_str(&raw).map_err(|e| format!("MCP HTTP JSON: {e}"))?;
    if let Some(err) = resp.get("error") {
        return Err(format!("MCP tools/list error: {err}"));
    }
    Ok(resp
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default())
}

fn jsonrpc_initialize(id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "loopcode", "version": env!("CARGO_PKG_VERSION") }
        }
    })
}

fn invoke_mcp_stdio(
    command: &str,
    args: &[String],
    tool_name: &str,
    arguments: &Value,
) -> Result<Value, String> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn MCP stdio `{command}`: {e}"))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "MCP stdio: no stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "MCP stdio: no stdout".to_string())?;
    let mut reader = BufReader::new(stdout);

    // Minimal handshake + tools/call (newline-delimited JSON-RPC).
    write_jsonrpc_line(&mut stdin, &jsonrpc_initialize(1))?;
    let _init = read_jsonrpc_line(&mut reader, Duration::from_secs(15))?;
    // notifications/initialized (no id)
    let notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    write_jsonrpc_line(&mut stdin, &notif)?;
    write_jsonrpc_line(&mut stdin, &jsonrpc_tools_call(tool_name, arguments, 2))?;
    let resp = read_jsonrpc_line(&mut reader, Duration::from_secs(30))?;

    // Best-effort terminate
    let _ = child.kill();
    let _ = child.wait();

    if let Some(err) = resp.get("error") {
        return Err(format!("MCP JSON-RPC error: {err}"));
    }
    let result = resp
        .get("result")
        .cloned()
        .unwrap_or_else(|| json!({"raw": resp}));
    Ok(json!({
        "fixture": false,
        "transport": format!("stdio:{command}"),
        "tool": tool_name,
        "result": result,
    }))
}

fn write_jsonrpc_line(w: &mut impl Write, msg: &Value) -> Result<(), String> {
    let line = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    writeln!(w, "{line}").map_err(|e| format!("MCP write: {e}"))?;
    w.flush().map_err(|e| format!("MCP flush: {e}"))
}

fn read_jsonrpc_line(
    reader: &mut BufReader<impl std::io::Read>,
    _timeout: Duration,
) -> Result<Value, String> {
    // Blocking read; callers use short-lived processes. (Full async timeouts are
    // a later hardening step; spawn failures and EOF are fail-closed.)
    let mut line = String::new();
    let n = reader
        .read_line(&mut line)
        .map_err(|e| format!("MCP read: {e}"))?;
    if n == 0 {
        return Err("MCP stdio EOF before response".into());
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err("MCP stdio empty line".into());
    }
    serde_json::from_str(trimmed).map_err(|e| format!("MCP JSON parse: {e} — {trimmed}"))
}

fn invoke_mcp_http(url: &str, tool_name: &str, arguments: &Value) -> Result<Value, String> {
    use crate::providers::adapter::HttpRequestSpec;
    use crate::providers::transport::{HttpTransport, LiveHttpTransport};
    use std::collections::HashMap;

    let transport = LiveHttpTransport::new()?;
    let body = jsonrpc_tools_call(tool_name, arguments, 1);
    let mut headers = HashMap::new();
    headers.insert("Content-Type".into(), "application/json".into());
    headers.insert("Accept".into(), "application/json".into());
    let req = HttpRequestSpec {
        method: "POST".into(),
        url: url.to_string(),
        headers,
        body,
    };
    let (status, raw) = transport.execute(&req)?;
    if !(200..300).contains(&status) {
        return Err(format!("MCP HTTP {status}: {}", raw.chars().take(200).collect::<String>()));
    }
    let resp: Value =
        serde_json::from_str(&raw).map_err(|e| format!("MCP HTTP JSON: {e}"))?;
    if let Some(err) = resp.get("error") {
        return Err(format!("MCP JSON-RPC error: {err}"));
    }
    let result = resp
        .get("result")
        .cloned()
        .unwrap_or_else(|| json!({"raw": resp}));
    Ok(json!({
        "fixture": false,
        "transport": format!("http:{url}"),
        "tool": tool_name,
        "result": result,
        "httpStatus": status,
    }))
}

/// Test/helper: invoke MCP with an injected HTTP transport (same code path body shape).
pub fn invoke_mcp_http_with_transport(
    url: &str,
    tool_name: &str,
    arguments: &Value,
    transport: &dyn crate::providers::transport::HttpTransport,
) -> Result<Value, String> {
    use crate::providers::adapter::HttpRequestSpec;
    use std::collections::HashMap;

    let body = jsonrpc_tools_call(tool_name, arguments, 1);
    let mut headers = HashMap::new();
    headers.insert("Content-Type".into(), "application/json".into());
    let req = HttpRequestSpec {
        method: "POST".into(),
        url: url.to_string(),
        headers,
        body,
    };
    let (status, raw) = transport.execute(&req)?;
    if !(200..300).contains(&status) {
        return Err(format!("MCP HTTP {status}"));
    }
    let resp: Value =
        serde_json::from_str(&raw).map_err(|e| format!("MCP HTTP JSON: {e}"))?;
    if let Some(err) = resp.get("error") {
        return Err(format!("MCP JSON-RPC error: {err}"));
    }
    Ok(json!({
        "fixture": false,
        "transport": format!("http:{url}"),
        "tool": tool_name,
        "result": resp.get("result").cloned().unwrap_or(resp),
        "httpStatus": status,
    }))
}

/// When config is re-registered with different fingerprint, prior trust must fail.
pub fn reconfigure_mcp_server(
    db: &Database,
    server_id: &str,
    new_transport: McpTransport,
) -> Result<McpServerConfig, String> {
    let servers = load_all(db)?;
    let existing = servers
        .iter()
        .find(|s| s.id == server_id)
        .ok_or_else(|| format!("unknown MCP server: {server_id}"))?
        .clone();
    register_mcp_server(
        db,
        &existing.name,
        new_transport,
        existing.project_id.as_deref(),
        Some(server_id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::store::Database;

    #[test]
    fn dual_gate_and_fingerprint_reset() {
        let db = Database::open_in_memory().unwrap();
        let cfg = register_mcp_server(
            &db,
            "demo",
            McpTransport::Stdio {
                command: "npx".into(),
                args: vec!["-y".into(), "demo-mcp".into()],
                env_keys: vec!["API_KEY".into()],
            },
            Some("proj1"),
            None,
        )
        .unwrap();
        let fp_a = cfg.fingerprint.clone();

        // No trust → blocked
        let r = mcp_call(&db, &cfg.id, "search", &json!({}), Some("proj1"), false).unwrap();
        assert_eq!(r.status, "require_server_trust");
        assert!(!r.executed);

        grant_mcp_server_trust(&db, &cfg.id, Some("proj1")).unwrap();
        assert!(is_mcp_server_trusted(&db, &cfg.id, Some("proj1")).unwrap());

        // Trusted but no invocation grant → still require_approval
        let r = mcp_call(&db, &cfg.id, "search", &json!({}), Some("proj1"), false).unwrap();
        assert_eq!(r.status, "require_approval");
        assert!(r.server_trusted);
        assert!(!r.executed);

        // Both gates (fixture mode: dual-gate contract only; real transport tested elsewhere)
        std::env::set_var("LOOPCODE_MCP_FIXTURE", "1");
        let r = mcp_call(&db, &cfg.id, "search", &json!({"q": "x"}), Some("proj1"), true).unwrap();
        std::env::remove_var("LOOPCODE_MCP_FIXTURE");
        assert_eq!(r.status, "ok");
        assert!(r.executed);

        // Config change → fingerprint B, trust invalid
        let cfg_b = reconfigure_mcp_server(
            &db,
            &cfg.id,
            McpTransport::Stdio {
                command: "npx".into(),
                args: vec!["-y".into(), "demo-mcp@2".into()],
                env_keys: vec!["API_KEY".into()],
            },
        )
        .unwrap();
        assert_ne!(cfg_b.fingerprint, fp_a);
        assert!(!is_mcp_server_trusted(&db, &cfg.id, Some("proj1")).unwrap());
        let r = mcp_call(&db, &cfg.id, "search", &json!({}), Some("proj1"), true).unwrap();
        assert_eq!(r.status, "require_server_trust");
    }

    #[test]
    fn http_transport_fingerprint() {
        let t = McpTransport::Http {
            url: "https://mcp.example/sse".into(),
            header_keys: vec!["Authorization".into()],
        };
        let a = config_fingerprint(&t);
        let b = config_fingerprint(&McpTransport::Http {
            url: "https://mcp.example/sse".into(),
            header_keys: vec!["Authorization".into()],
        });
        assert_eq!(a, b);
        let c = config_fingerprint(&McpTransport::Http {
            url: "https://mcp.example/v2".into(),
            header_keys: vec!["Authorization".into()],
        });
        assert_ne!(a, c);
    }
}
