use crate::{broker::FrontendGeneration, diagnostics::Diagnostics};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tauri::{AppHandle, Manager, State, ipc::Channel};
use tokio::{
    sync::{Mutex, watch},
    time::{Duration, timeout},
};

const TERMINAL_STOP_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TERMINAL_INPUT_BYTES: usize = 1024 * 1024;

#[derive(Default)]
pub struct TerminalManager {
    next_id: AtomicU64,
    shutting_down: AtomicBool,
    lifecycle: Mutex<()>,
    sessions: Mutex<HashMap<String, TerminalHandle>>,
}

struct TerminalHandle {
    thread_id: String,
    master: Arc<StdMutex<Box<dyn MasterPty + Send>>>,
    writer: Arc<StdMutex<Box<dyn Write + Send>>>,
    killer: Arc<StdMutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>>,
    stopped: watch::Receiver<bool>,
}

#[derive(Clone)]
struct TerminalStop {
    thread_id: String,
    killer: Arc<StdMutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>>,
    stopped: watch::Receiver<bool>,
}

impl TerminalHandle {
    fn stop_control(&self) -> TerminalStop {
        TerminalStop {
            thread_id: self.thread_id.clone(),
            killer: Arc::clone(&self.killer),
            stopped: self.stopped.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTerminalRequest {
    thread_id: String,
    cwd: String,
    cols: u16,
    rows: u16,
    frontend_generation: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTerminalResult {
    terminal_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum TerminalEvent {
    Output { bytes: Vec<u8> },
    Exited { code: u32, success: bool },
    Error { message: String },
}

#[tauri::command]
pub async fn start_terminal(
    app: AppHandle,
    manager: State<'_, TerminalManager>,
    frontend_generation: State<'_, FrontendGeneration>,
    diagnostics: State<'_, Diagnostics>,
    request: StartTerminalRequest,
    on_event: Channel<TerminalEvent>,
) -> Result<StartTerminalResult, String> {
    let thread_id = valid_thread_id(&request.thread_id)?.to_owned();
    let size = terminal_size(request.cols, request.rows)?;
    let cwd = PathBuf::from(request.cwd.trim());
    if !cwd.is_absolute() {
        return Err("The terminal working folder must be an absolute path".into());
    }
    let cwd = tauri::async_runtime::spawn_blocking(move || {
        let cwd = cwd
            .canonicalize()
            .map_err(|error| format!("Could not resolve the terminal working folder: {error}"))?;
        if !cwd.is_dir() {
            return Err("The terminal working folder must be an existing folder".into());
        }
        Ok::<_, String>(cwd)
    })
    .await
    .map_err(|error| format!("Could not join the terminal folder task: {error}"))??;

    let _lifecycle = manager.lifecycle.lock().await;
    if manager.shutting_down.load(Ordering::Acquire) {
        return Err("LoopCode is shutting down".into());
    }
    if request.frontend_generation != frontend_generation.0.load(Ordering::Acquire) {
        return Err("The frontend was replaced before the terminal could start".into());
    }
    stop_matching_terminal(&manager, &diagnostics, &thread_id).await?;

    let terminal_id = format!(
        "terminal-{}",
        manager.next_id.fetch_add(1, Ordering::Relaxed) + 1
    );
    let mut command = default_shell_command();
    command.cwd(&cwd);
    let startup = tauri::async_runtime::spawn_blocking(move || {
        let pair = native_pty_system()
            .openpty(size)
            .map_err(|error| format!("Could not open a terminal: {error}"))?;
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| format!("Could not start the default shell: {error}"))?;
        drop(pair.slave);
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| format!("Could not read terminal output: {error}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| format!("Could not open terminal input: {error}"))?;
        let killer = child.clone_killer();
        Ok::<_, String>((pair.master, reader, writer, child, killer))
    })
    .await
    .map_err(|error| format!("Could not join the terminal startup task: {error}"))??;
    let (master, mut reader, writer, mut child, killer) = startup;
    let master = Arc::new(StdMutex::new(master));
    let writer = Arc::new(StdMutex::new(writer));
    let killer = Arc::new(StdMutex::new(killer));
    let (stopped, stopped_rx) = watch::channel(false);

    manager.sessions.lock().await.insert(
        terminal_id.clone(),
        TerminalHandle {
            thread_id: thread_id.clone(),
            master,
            writer,
            killer,
            stopped: stopped_rx,
        },
    );
    diagnostics.record(
        "info",
        "terminal.started",
        json!({ "terminalId": terminal_id, "threadId": thread_id }),
    );

    let output_events = on_event.clone();
    let output_terminal_id = terminal_id.clone();
    let output_diagnostics = diagnostics.inner().clone();
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(length) => {
                    let _ = output_events.send(TerminalEvent::Output {
                        bytes: buffer[..length].to_vec(),
                    });
                }
                Err(error) => {
                    let message = format!("Could not read terminal output: {error}");
                    output_diagnostics.record(
                        "error",
                        "terminal.output_failed",
                        json!({ "terminalId": output_terminal_id, "message": message }),
                    );
                    let _ = output_events.send(TerminalEvent::Error { message });
                    break;
                }
            }
        }
    });

    let supervisor_id = terminal_id.clone();
    let supervisor_thread_id = thread_id.clone();
    let supervisor_diagnostics = diagnostics.inner().clone();
    tauri::async_runtime::spawn(async move {
        let status = tauri::async_runtime::spawn_blocking(move || child.wait()).await;
        app.state::<TerminalManager>()
            .sessions
            .lock()
            .await
            .remove(&supervisor_id);

        match status {
            Ok(Ok(status)) => {
                supervisor_diagnostics.record(
                    if status.success() { "info" } else { "error" },
                    "terminal.exited",
                    json!({
                        "terminalId": supervisor_id,
                        "threadId": supervisor_thread_id,
                        "code": status.exit_code(),
                        "success": status.success(),
                    }),
                );
                let _ = on_event.send(TerminalEvent::Exited {
                    code: status.exit_code(),
                    success: status.success(),
                });
            }
            Ok(Err(error)) => {
                let message = format!("Could not observe the terminal process: {error}");
                supervisor_diagnostics.record(
                    "error",
                    "terminal.wait_failed",
                    json!({ "terminalId": supervisor_id, "message": message }),
                );
                let _ = on_event.send(TerminalEvent::Error { message });
            }
            Err(error) => {
                let message = format!("Could not join the terminal process task: {error}");
                supervisor_diagnostics.record(
                    "error",
                    "terminal.wait_join_failed",
                    json!({ "terminalId": supervisor_id, "message": message }),
                );
                let _ = on_event.send(TerminalEvent::Error { message });
            }
        }
        let _ = stopped.send(true);
    });

    Ok(StartTerminalResult { terminal_id })
}

#[tauri::command]
pub async fn write_terminal(
    manager: State<'_, TerminalManager>,
    terminal_id: String,
    data: String,
) -> Result<(), String> {
    if data.len() > MAX_TERMINAL_INPUT_BYTES {
        return Err("Terminal input is too large".into());
    }
    valid_terminal_id(&terminal_id)?;
    let writer = manager
        .sessions
        .lock()
        .await
        .get(&terminal_id)
        .map(|handle| Arc::clone(&handle.writer))
        .ok_or("The terminal is no longer running")?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut writer = writer
            .lock()
            .map_err(|_| "Could not lock terminal input".to_owned())?;
        writer
            .write_all(data.as_bytes())
            .and_then(|_| writer.flush())
            .map_err(|error| format!("Could not write to the terminal: {error}"))
    })
    .await
    .map_err(|error| format!("Could not join the terminal input task: {error}"))?
}

#[tauri::command]
pub async fn resize_terminal(
    manager: State<'_, TerminalManager>,
    terminal_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let size = terminal_size(cols, rows)?;
    valid_terminal_id(&terminal_id)?;
    let master = manager
        .sessions
        .lock()
        .await
        .get(&terminal_id)
        .map(|handle| Arc::clone(&handle.master))
        .ok_or("The terminal is no longer running")?;
    tauri::async_runtime::spawn_blocking(move || {
        master
            .lock()
            .map_err(|_| "Could not lock the terminal".to_owned())?
            .resize(size)
            .map_err(|error| format!("Could not resize the terminal: {error}"))
    })
    .await
    .map_err(|error| format!("Could not join the terminal resize task: {error}"))?
}

#[tauri::command]
pub async fn stop_terminal(
    manager: State<'_, TerminalManager>,
    diagnostics: State<'_, Diagnostics>,
    terminal_id: String,
) -> Result<(), String> {
    valid_terminal_id(&terminal_id)?;
    let _lifecycle = manager.lifecycle.lock().await;
    let control = manager
        .sessions
        .lock()
        .await
        .get(&terminal_id)
        .map(TerminalHandle::stop_control);
    if let Some(control) = control {
        stop_terminal_control(&diagnostics, &terminal_id, control, "client_request").await?;
        manager.sessions.lock().await.remove(&terminal_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn stop_terminal_for_thread(
    manager: State<'_, TerminalManager>,
    diagnostics: State<'_, Diagnostics>,
    thread_id: String,
) -> Result<(), String> {
    let thread_id = valid_thread_id(&thread_id)?;
    let _lifecycle = manager.lifecycle.lock().await;
    stop_matching_terminal(&manager, &diagnostics, thread_id).await
}

#[tauri::command]
pub async fn stop_all_terminals(
    manager: State<'_, TerminalManager>,
    diagnostics: State<'_, Diagnostics>,
) -> Result<(), String> {
    manager.stop_all(&diagnostics, "client_request").await
}

impl TerminalManager {
    pub async fn register_frontend(&self, diagnostics: &Diagnostics) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        if self.shutting_down.load(Ordering::Acquire) {
            return Err("LoopCode is shutting down".into());
        }
        self.stop_all_locked(diagnostics, "frontend_registered")
            .await
    }

    pub async fn stop_all(&self, diagnostics: &Diagnostics, reason: &str) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        self.stop_all_locked(diagnostics, reason).await
    }

    pub async fn shutdown(&self, diagnostics: &Diagnostics) -> Result<(), String> {
        let _lifecycle = self.lifecycle.lock().await;
        self.shutting_down.store(true, Ordering::Release);
        self.stop_all_locked(diagnostics, "app_exit").await
    }

    async fn stop_all_locked(&self, diagnostics: &Diagnostics, reason: &str) -> Result<(), String> {
        let controls = self
            .sessions
            .lock()
            .await
            .iter()
            .map(|(id, handle)| (id.clone(), handle.stop_control()))
            .collect::<Vec<_>>();
        let mut first_error = None;
        for (terminal_id, control) in controls {
            match stop_terminal_control(diagnostics, &terminal_id, control, reason).await {
                Ok(()) => {
                    self.sessions.lock().await.remove(&terminal_id);
                }
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

async fn stop_matching_terminal(
    manager: &TerminalManager,
    diagnostics: &Diagnostics,
    thread_id: &str,
) -> Result<(), String> {
    let matching = manager
        .sessions
        .lock()
        .await
        .iter()
        .filter(|(_, handle)| handle.thread_id == thread_id)
        .map(|(id, handle)| (id.clone(), handle.stop_control()))
        .collect::<Vec<_>>();
    for (terminal_id, control) in matching {
        stop_terminal_control(diagnostics, &terminal_id, control, "replacement").await?;
        manager.sessions.lock().await.remove(&terminal_id);
    }
    Ok(())
}

async fn stop_terminal_control(
    diagnostics: &Diagnostics,
    terminal_id: &str,
    control: TerminalStop,
    reason: &str,
) -> Result<(), String> {
    diagnostics.record(
        "info",
        "terminal.stop_requested",
        json!({
            "terminalId": terminal_id,
            "threadId": control.thread_id,
            "reason": reason,
        }),
    );
    let killer = Arc::clone(&control.killer);
    let kill_result = tauri::async_runtime::spawn_blocking(move || {
        killer
            .lock()
            .map_err(|_| "Could not lock the terminal process".to_owned())?
            .kill()
            .map_err(|error| format!("Could not stop the terminal: {error}"))
    })
    .await
    .map_err(|error| format!("Could not join the terminal stop task: {error}"))?;
    let mut stopped = control.stopped;
    let wait_result = timeout(TERMINAL_STOP_TIMEOUT, stopped.wait_for(|value| *value)).await;
    match wait_result {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(_)) => Err("Could not confirm that the terminal stopped".into()),
        Err(_) => kill_result.and(Err(format!("Timed out waiting for {terminal_id} to stop"))),
    }
}

fn valid_thread_id(thread_id: &str) -> Result<&str, String> {
    let thread_id = thread_id.trim();
    if thread_id.is_empty() || thread_id.len() > 512 {
        return Err("The terminal thread id is invalid".into());
    }
    Ok(thread_id)
}

fn valid_terminal_id(terminal_id: &str) -> Result<(), String> {
    if terminal_id.is_empty() || terminal_id.len() > 128 {
        return Err("The terminal id is invalid".into());
    }
    Ok(())
}

fn default_shell_command() -> CommandBuilder {
    #[cfg(windows)]
    {
        CommandBuilder::new("powershell.exe")
    }
    #[cfg(not(windows))]
    {
        CommandBuilder::new_default_prog()
    }
}

fn terminal_size(cols: u16, rows: u16) -> Result<PtySize, String> {
    if !(2..=1000).contains(&cols) || !(1..=1000).contains(&rows) {
        return Err("The terminal size is invalid".into());
    }
    Ok(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::terminal_size;

    #[test]
    fn validates_terminal_dimensions() {
        assert!(terminal_size(80, 24).is_ok());
        assert!(terminal_size(1, 24).is_err());
        assert!(terminal_size(80, 0).is_err());
        assert!(terminal_size(1001, 24).is_err());
    }
}
