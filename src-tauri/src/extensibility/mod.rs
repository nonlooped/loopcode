//! Extensibility: skills invoke/script gate, MCP dual-gate.

pub mod mcp;

pub use mcp::{
    config_fingerprint, grant_mcp_server_trust, invoke_mcp_http_with_transport,
    invoke_mcp_transport, is_mcp_server_trusted, list_mcp_catalog_tools, list_mcp_servers,
    mcp_call, register_mcp_server, remove_mcp_server, reconfigure_mcp_server,
    set_mcp_server_enabled, McpCallResult, McpServerConfig, McpTransport,
};
