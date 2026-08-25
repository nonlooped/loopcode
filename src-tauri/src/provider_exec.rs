//! Allowlist for provider/harness process spawns.
//!
//! Frontend-supplied `command` + `args` are intentional for ACP providers, but
//! unrestricted spawn turns any webview compromise into local RCE. Only known
//! provider executables (by basename) may run; `npx` is further limited to
//! known ACP packages.

use std::path::{Component, Path};

const MAX_COMMAND_LEN: usize = 4096;
const MAX_ARG_LEN: usize = 4096;
const MAX_ARGS: usize = 16;

const ALLOWED_EXECUTABLES: &[&str] = &[
    "agent",
    "agent.cmd",
    "claude",
    "claude.cmd",
    "codex",
    "codex.cmd",
    "fx",
    "grok",
    "grok.exe",
    "npx",
    "npx.cmd",
    "opencode",
    "pi",
    "pi.cmd",
];

const ALLOWED_NPX_PACKAGES: &[&str] = &[
    "@agentclientprotocol/claude-agent-acp",
    "@agentclientprotocol/codex-acp",
    "@victor-software-house/pi-acp",
];

/// Validate a provider probe or harness spawn requested by the frontend.
pub fn validate_provider_spawn(command: &str, args: &[String]) -> Result<(), String> {
    let command = command.trim();
    if command.is_empty() || command.len() > MAX_COMMAND_LEN {
        return Err("Enter a valid provider executable".into());
    }
    if args.len() > MAX_ARGS || args.iter().any(|arg| arg.len() > MAX_ARG_LEN) {
        return Err("Provider command arguments are too long".into());
    }

    let path = Path::new(command);
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("Provider executable paths must not contain '..'".into());
    }

    let has_separator = command.contains(['/', '\\']);
    if has_separator && !path.is_absolute() {
        return Err("Provider executable overrides must be an absolute path".into());
    }

    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return Err("Enter a valid provider executable".into());
    };
    if !is_allowed_executable(file_name) {
        return Err(format!(
            "Provider executable '{file_name}' is not an allowed LoopCode provider"
        ));
    }

    if is_npx(file_name) {
        validate_npx_args(args)?;
    }

    Ok(())
}

fn is_allowed_executable(file_name: &str) -> bool {
    ALLOWED_EXECUTABLES
        .iter()
        .any(|allowed| file_name.eq_ignore_ascii_case(allowed))
}

fn is_npx(file_name: &str) -> bool {
    file_name.eq_ignore_ascii_case("npx") || file_name.eq_ignore_ascii_case("npx.cmd")
}

fn validate_npx_args(args: &[String]) -> Result<(), String> {
    match args {
        [flag, package] if flag == "--yes" && is_allowed_npx_package(package) => Ok(()),
        _ => Err(
            "npx may only run known LoopCode ACP packages with '--yes <package>@<version>'".into(),
        ),
    }
}

fn is_allowed_npx_package(spec: &str) -> bool {
    let (name, version) = match spec.rsplit_once('@') {
        Some((name, version)) if name.starts_with('@') => (name, version),
        // Scoped packages look like @scope/name@version — rsplit keeps the scope '@'.
        _ => return false,
    };
    if version.is_empty()
        || !version
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '+'))
    {
        return false;
    }
    ALLOWED_NPX_PACKAGES
        .iter()
        .any(|allowed| name.eq_ignore_ascii_case(allowed))
}

#[cfg(test)]
mod tests {
    use super::validate_provider_spawn;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn allows_known_bare_executables() {
        assert!(validate_provider_spawn("claude", &args(&["--version"])).is_ok());
        assert!(validate_provider_spawn("opencode", &args(&["acp"])).is_ok());
        assert!(validate_provider_spawn("grok.exe", &args(&["version"])).is_ok());
    }

    #[test]
    fn allows_absolute_overrides_with_allowed_basename() {
        assert!(validate_provider_spawn("/usr/local/bin/claude", &args(&["auth", "status"])).is_ok());
        #[cfg(windows)]
        assert!(
            validate_provider_spawn(r"C:\Tools\claude.cmd", &args(&["--version"])).is_ok()
        );
    }

    #[test]
    fn rejects_shells_and_unknown_binaries() {
        assert!(validate_provider_spawn("sh", &args(&["-c", "id"])).is_err());
        assert!(validate_provider_spawn("bash", &args(&["-c", "id"])).is_err());
        assert!(validate_provider_spawn("cmd.exe", &args(&["/c", "dir"])).is_err());
        assert!(validate_provider_spawn("powershell", &args(&["-Command", "id"])).is_err());
        assert!(validate_provider_spawn("python", &args(&["-c", "print(1)"])).is_err());
        assert!(validate_provider_spawn("/usr/bin/python3", &args(&["-c", "print(1)"])).is_err());
    }

    #[test]
    fn rejects_relative_paths_and_parent_dirs() {
        assert!(validate_provider_spawn("./claude", &[]).is_err());
        assert!(validate_provider_spawn("../claude", &[]).is_err());
        assert!(validate_provider_spawn("/tmp/../usr/bin/claude", &[]).is_err());
    }

    #[test]
    fn allows_known_npx_packages_only() {
        assert!(
            validate_provider_spawn(
                "npx",
                &args(&["--yes", "@agentclientprotocol/codex-acp@1.4.0"]),
            )
            .is_ok()
        );
        assert!(
            validate_provider_spawn(
                "npx.cmd",
                &args(&["--yes", "@agentclientprotocol/claude-agent-acp@0.69.0"]),
            )
            .is_ok()
        );
        assert!(
            validate_provider_spawn("npx", &args(&["--yes", "evil-package@1.0.0"])).is_err()
        );
        assert!(validate_provider_spawn("npx", &args(&["evil-package"])).is_err());
        assert!(
            validate_provider_spawn(
                "npx",
                &args(&["--yes", "@agentclientprotocol/codex-acp@1.0.0;rm -rf /"]),
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_oversized_commands_and_args() {
        assert!(validate_provider_spawn("", &[]).is_err());
        assert!(validate_provider_spawn(&"x".repeat(4097), &[]).is_err());
        assert!(validate_provider_spawn("claude", &vec!["x".into(); 17]).is_err());
        assert!(validate_provider_spawn("claude", &args(&[&"x".repeat(4097)])).is_err());
    }
}
