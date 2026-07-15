//! Fixed built-in tool catalog. No first-party Git tools.
//! MCP tools from enabled/trusted servers are merged at catalog-build time.

use crate::db::store::Database;
use crate::runtime::mode::ToolEffect;
use crate::runtime::tools::ToolDefinition;

/// Names that must never appear in the built-in catalog (v1 product boundary).
pub const FORBIDDEN_GIT_TOOLS: &[&str] = &[
    "git.status",
    "git.commit",
    "git.push",
    "git.pull",
    "git.stage",
    "git.unstage",
    "git.diff",
    "git.branch",
    "git.checkout",
    "git.log",
    "git.pr",
    "vcs.commit",
];

/// Built-in catalog (stable names). MCP tools are merged via [`effective_catalog`].
pub fn builtin_catalog() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition::new("glob", ToolEffect::ReadOnly, "Find files by glob pattern"),
        ToolDefinition::new("read", ToolEffect::ReadOnly, "Read a text file"),
        ToolDefinition::new(
            "grep",
            ToolEffect::ReadOnly,
            "Search file contents (bounded)",
        ),
        ToolDefinition::new(
            "edit",
            ToolEffect::Mutating,
            "Exact string replacement in a file",
        ),
        ToolDefinition::new(
            "patch",
            ToolEffect::Mutating,
            "Replace or delete file contents with pre-image hash",
        ),
        ToolDefinition::new("write", ToolEffect::Mutating, "Write whole file"),
        ToolDefinition::new("exec", ToolEffect::Mutating, "Run a shell command"),
        ToolDefinition::new("plan_write", ToolEffect::ReadOnly, "Create plan artifact"),
        ToolDefinition::new("plan_update", ToolEffect::ReadOnly, "Update plan artifact"),
        ToolDefinition::new("webfetch", ToolEffect::ReadOnly, "Fetch a URL"),
        ToolDefinition::new(
            "context.attach",
            ToolEffect::ReadOnly,
            "Attach path to context manifest",
        ),
        ToolDefinition::new(
            "skill",
            ToolEffect::ReadOnly,
            "Load a skill body into context",
        ),
    ]
}

/// Built-ins plus MCP tools from enabled servers (best-effort list).
pub fn effective_catalog(db: Option<&Database>, project_id: Option<&str>) -> Vec<ToolDefinition> {
    let mut tools = builtin_catalog();
    if let Some(db) = db {
        match crate::extensibility::mcp::list_mcp_catalog_tools(db, project_id) {
            Ok(mcp) => tools.extend(mcp),
            Err(_) => {
                // Fail soft — agent still gets builtins if MCP discovery fails.
            }
        }
    }
    tools
}

/// Assert catalog contains no Git tools (shipped invariant).
pub fn catalog_has_no_git_tools(defs: &[ToolDefinition]) -> bool {
    !defs.iter().any(|d| {
        FORBIDDEN_GIT_TOOLS.contains(&d.name.as_str())
            || d.name.starts_with("git.")
            || d.name.starts_with("vcs.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_git_in_builtin_catalog() {
        assert!(catalog_has_no_git_tools(&builtin_catalog()));
    }

    #[test]
    fn builtin_names_are_flat() {
        let names: Vec<_> = builtin_catalog().into_iter().map(|d| d.name).collect();
        assert!(names.contains(&"glob".into()));
        assert!(names.contains(&"edit".into()));
        assert!(names.contains(&"patch".into()));
        assert!(names.contains(&"exec".into()));
        assert!(names.contains(&"skill".into()));
        assert!(!names.iter().any(|n| n == "mcp.call"));
        assert!(!names.iter().any(|n| n == "diagnostics.collect"));
        assert!(!names.iter().any(|n| n.starts_with("workspace.")));
    }
}
