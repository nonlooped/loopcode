//! `exec` — Core-owned process with output caps (always approval-gated).
//!
//! On Windows the host prefers `pwsh`, then Windows PowerShell, then `cmd`.
//! Timeouts kill the process tree (Job Object on Windows; process group on Unix).

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const MAX_OUTPUT_BYTES: usize = 64 * 1024;

/// Resolved shell host for `exec` (also surfaced in tool results / prompts).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellHost {
    pub program: String,
    pub kind: &'static str,
}

/// Prefer pwsh → powershell → cmd on Windows; `sh -c` elsewhere.
pub fn resolve_shell_host() -> ShellHost {
    #[cfg(windows)]
    {
        if command_on_path("pwsh") {
            return ShellHost {
                program: "pwsh".into(),
                kind: "pwsh",
            };
        }
        if command_on_path("powershell") {
            return ShellHost {
                program: "powershell".into(),
                kind: "powershell",
            };
        }
        ShellHost {
            program: "cmd".into(),
            kind: "cmd",
        }
    }
    #[cfg(not(windows))]
    {
        ShellHost {
            program: "sh".into(),
            kind: "sh",
        }
    }
}

fn command_on_path(name: &str) -> bool {
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(name)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        Command::new("which")
            .arg(name)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

fn build_command(host: &ShellHost, script: &str, cwd: &Path) -> Command {
    let mut command = Command::new(&host.program);
    match host.kind {
        "pwsh" | "powershell" => {
            command
                .arg("-NoLogo")
                .arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-Command")
                .arg(script);
        }
        "cmd" => {
            command.arg("/C").arg(script);
        }
        _ => {
            command.arg("-c").arg(script);
        }
    }
    command
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Own process group so timeouts can signal the whole tree.
        command.process_group(0);
    }

    command
}

/// Run a command with cwd defaulting to project root.
pub fn shell_exec(root: &Path, args: &Value) -> Result<Value, String> {
    let cmd = args
        .get("command")
        .or_else(|| args.get("cmd"))
        .and_then(|v| v.as_str())
        .ok_or("command required")?;
    if cmd.trim().is_empty() {
        return Err("empty command".into());
    }

    let cwd = resolve_cwd(root, args)?;
    let class = crate::reliability::retry::ToolTimeoutClass::for_tool("exec");
    let timeout = Duration::from_secs(
        args.get("timeoutSecs")
            .and_then(|v| v.as_u64())
            .unwrap_or(class.timeout_secs())
            .min(class.timeout_secs()),
    );

    let host = resolve_shell_host();
    let mut command = build_command(&host, cmd, &cwd);

    let mut child = command.spawn().map_err(|e| format!("spawn: {e}"))?;
    let job = attach_job_object(&mut child);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = take_pipe_bytes(child.stdout.take());
                let stderr = take_pipe_bytes(child.stderr.take());
                drop(job);
                return Ok(json!({
                    "status": "ok",
                    "exitCode": status.code(),
                    "success": status.success(),
                    "stdout": String::from_utf8_lossy(&stdout),
                    "stderr": String::from_utf8_lossy(&stderr),
                    "timedOut": false,
                    "elapsedMs": start.elapsed().as_millis() as u64,
                    "shell": host.kind,
                }));
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    terminate_tree(&mut child, job.as_ref());
                    let _ = child.wait();
                    return Ok(json!({
                        "status": "timeout",
                        "exitCode": null,
                        "success": false,
                        "stdout": "",
                        "stderr": "command timed out",
                        "timedOut": true,
                        "elapsedMs": start.elapsed().as_millis() as u64,
                        "shell": host.kind,
                    }));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => {
                terminate_tree(&mut child, job.as_ref());
                return Err(format!("wait: {e}"));
            }
        }
    }
}

fn resolve_cwd(root: &Path, args: &Value) -> Result<PathBuf, String> {
    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(|c| root.join(c))
        .unwrap_or_else(|| root.to_path_buf());
    if !cwd.starts_with(root) && cwd != root {
        let canon_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let canon_cwd = cwd.canonicalize().unwrap_or(cwd.clone());
        if !canon_cwd.starts_with(&canon_root) {
            return Err("cwd outside project root".into());
        }
    }
    Ok(cwd)
}

fn take_pipe_bytes(pipe: Option<impl std::io::Read>) -> Vec<u8> {
    pipe.map(|mut s| {
        let mut buf = Vec::new();
        let _ = std::io::Read::read_to_end(&mut s, &mut buf);
        truncate_bytes(&buf)
    })
    .unwrap_or_default()
}

fn truncate_bytes(buf: &[u8]) -> Vec<u8> {
    if buf.len() <= MAX_OUTPUT_BYTES {
        buf.to_vec()
    } else {
        let mut v = buf[..MAX_OUTPUT_BYTES].to_vec();
        v.extend_from_slice(b"\n...[truncated]");
        v
    }
}

fn terminate_tree(child: &mut Child, job: Option<&JobGuard>) {
    if let Some(j) = job {
        j.terminate();
    }
    #[cfg(unix)]
    {
        let pid = child.id();
        // Negative PID = process group (set via process_group(0) at spawn).
        let _ = Command::new("kill")
            .args(["-TERM", &format!("-{pid}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
}

struct JobGuard {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl Drop for JobGuard {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.handle);
            }
        }
    }
}

impl JobGuard {
    fn terminate(&self) {
        #[cfg(windows)]
        unsafe {
            let _ = windows_sys::Win32::System::JobObjects::TerminateJobObject(self.handle, 1);
        }
    }
}

fn attach_job_object(child: &mut Child) -> Option<JobGuard> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        unsafe {
            let job: HANDLE = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return None;
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let ok = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &mut info as *mut _ as *mut _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                CloseHandle(job);
                return None;
            }

            let process = child.as_raw_handle() as HANDLE;
            if AssignProcessToJobObject(job, process) == 0 {
                CloseHandle(job);
                return None;
            }
            Some(JobGuard { handle: job })
        }
    }
    #[cfg(not(windows))]
    {
        let _ = child;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_shell_host_is_non_empty() {
        let h = resolve_shell_host();
        assert!(!h.program.is_empty());
        assert!(!h.kind.is_empty());
    }

    #[test]
    fn shell_exec_echo_succeeds() {
        let root = std::env::temp_dir().join(format!(
            "lc-shell-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let host = resolve_shell_host();
        let command = match host.kind {
            "pwsh" | "powershell" => "Write-Output hello-loop",
            "cmd" => "echo hello-loop",
            _ => "echo hello-loop",
        };
        let out = shell_exec(&root, &json!({"command": command, "timeoutSecs": 30})).unwrap();
        assert_eq!(out["timedOut"], false);
        assert!(
            out["stdout"].as_str().unwrap_or("").contains("hello-loop")
                || out["success"].as_bool() == Some(true),
            "{out}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
