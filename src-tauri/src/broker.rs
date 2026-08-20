use crate::diagnostics::{Diagnostics, rpc_fields, safe_stderr};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::Path,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Instant,
};
use tauri::{AppHandle, Manager, State, ipc::Channel};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, Command},
    sync::{Mutex, mpsc, watch},
    time::{Duration, timeout},
};

const HARNESS_STOP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default)]
pub struct Broker {
    next_id: AtomicU64,
    frontend_generation: AtomicU64,
    shutting_down: AtomicBool,
    lifecycle: Mutex<()>,
    harnesses: Mutex<HashMap<String, HarnessHandle>>,
}

struct HarnessHandle {
    stdin: Arc<Mutex<ChildStdin>>,
    stop: mpsc::Sender<()>,
    stopped: watch::Receiver<bool>,
    pending_requests: Arc<Mutex<HashMap<String, PendingRequest>>>,
    profile_id: Option<String>,
    thread_id: Option<String>,
}

#[derive(Clone)]
struct HarnessStop {
    stop: mpsc::Sender<()>,
    stopped: watch::Receiver<bool>,
    profile_id: Option<String>,
    thread_id: Option<String>,
}

impl HarnessHandle {
    fn stop_control(&self) -> HarnessStop {
        HarnessStop {
            stop: self.stop.clone(),
            stopped: self.stopped.clone(),
            profile_id: self.profile_id.clone(),
            thread_id: self.thread_id.clone(),
        }
    }
}

struct PendingRequest {
    method: String,
    started_at: Instant,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    command: String,
    args: Vec<String>,
    cwd: String,
    profile_id: Option<String>,
    thread_id: Option<String>,
    frontend_generation: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    harness_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum BrokerEvent {
    Rpc { message: Value },
    Stderr { line: String },
    Exited { code: Option<i32>, success: bool },
    Error { message: String },
}

#[tauri::command]
pub async fn launch_harness(
    app: AppHandle,
    broker: State<'_, Broker>,
    request: LaunchRequest,
    on_event: Channel<BrokerEvent>,
) -> Result<LaunchResult, String> {
    let command_name = request.command.trim();
    if command_name.is_empty() {
        return Err("Enter a harness executable".into());
    }

    let cwd = Path::new(request.cwd.trim());
    if !cwd.is_absolute() || !cwd.is_dir() {
        return Err("The working folder must be an existing absolute path".into());
    }

    let _lifecycle = broker.lifecycle.lock().await;
    if broker.shutting_down.load(Ordering::Acquire) {
        return Err("LoopCode is shutting down".into());
    }
    if request.frontend_generation != broker.frontend_generation.load(Ordering::Acquire) {
        return Err("The frontend was replaced before the harness could start".into());
    }
    stop_matching_harnesses(
        &broker,
        &app.state::<Diagnostics>(),
        request.thread_id.as_deref(),
        request.profile_id.as_deref(),
    )
    .await?;

    let mut command = Command::new(command_name);
    command
        .args(&request.args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("NO_COLOR", "1");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.as_std_mut().creation_flags(0x0800_0000);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start {command_name}: {error}"))?;
    let stdin = Arc::new(Mutex::new(
        child.stdin.take().ok_or("The harness did not open stdin")?,
    ));
    let stdout = child
        .stdout
        .take()
        .ok_or("The harness did not open stdout")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("The harness did not open stderr")?;
    let harness_id = format!(
        "harness-{}",
        broker.next_id.fetch_add(1, Ordering::Relaxed) + 1
    );
    let (stop, mut stop_rx) = mpsc::channel(1);
    let (stopped, stopped_rx) = watch::channel(false);
    let pending_requests = Arc::new(Mutex::new(HashMap::new()));
    let diagnostics = app.state::<Diagnostics>().inner().clone();
    diagnostics.record(
        "info",
        "acp.harness.started",
        json!({
            "harnessId": harness_id,
            "profileId": request.profile_id,
            "threadId": request.thread_id,
            "command": command_name,
            "pid": child.id(),
        }),
    );

    broker.harnesses.lock().await.insert(
        harness_id.clone(),
        HarnessHandle {
            stdin: Arc::clone(&stdin),
            stop,
            stopped: stopped_rx,
            pending_requests: Arc::clone(&pending_requests),
            profile_id: request.profile_id.clone(),
            thread_id: request.thread_id.clone(),
        },
    );

    let stdout_events = on_event.clone();
    let stdout_diagnostics = diagnostics.clone();
    let stdout_harness_id = harness_id.clone();
    let stdout_profile_id = request.profile_id.clone();
    let stdout_thread_id = request.thread_id.clone();
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) if line.trim().is_empty() => {}
                Ok(Some(line)) => match serde_json::from_str::<Value>(&line) {
                    Ok(message) => {
                        let mut fields = rpc_fields(
                            &stdout_harness_id,
                            stdout_profile_id.as_deref(),
                            stdout_thread_id.as_deref(),
                            "agent_to_client",
                            &message,
                        );
                        if message.get("method").is_none()
                            && let Some(key) = request_key(&message)
                            && let Some(pending) = pending_requests.lock().await.remove(&key)
                            && let Value::Object(values) = &mut fields
                        {
                            values.insert("method".into(), json!(pending.method));
                            values.insert(
                                "durationMs".into(),
                                json!(pending.started_at.elapsed().as_millis()),
                            );
                        }
                        stdout_diagnostics.record(
                            if message.get("error").is_some() {
                                "error"
                            } else {
                                "debug"
                            },
                            "acp.rpc.received",
                            fields,
                        );
                        let _ = stdout_events.send(BrokerEvent::Rpc { message });
                    }
                    Err(error) => {
                        let message = format!("The harness returned invalid ACP JSON: {error}");
                        stdout_diagnostics.record(
                            "error",
                            "acp.stdout.invalid_json",
                            json!({
                                "harnessId": stdout_harness_id,
                                "profileId": stdout_profile_id,
                                "threadId": stdout_thread_id,
                                "lineLength": line.len(),
                                "message": message,
                            }),
                        );
                        let _ = stdout_events.send(BrokerEvent::Error { message });
                    }
                },
                Ok(None) => break,
                Err(error) => {
                    let message = format!("Could not read harness output: {error}");
                    stdout_diagnostics.record(
                        "error",
                        "acp.stdout.read_failed",
                        json!({
                            "harnessId": stdout_harness_id,
                            "profileId": stdout_profile_id,
                            "threadId": stdout_thread_id,
                            "message": message,
                        }),
                    );
                    let _ = stdout_events.send(BrokerEvent::Error { message });
                    break;
                }
            }
        }
    });

    let stderr_events = on_event.clone();
    let stderr_diagnostics = diagnostics.clone();
    let stderr_harness_id = harness_id.clone();
    let stderr_profile_id = request.profile_id.clone();
    let stderr_thread_id = request.thread_id.clone();
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if !line.trim().is_empty() {
                stderr_diagnostics.record(
                    "warn",
                    "acp.harness.stderr",
                    json!({
                        "harnessId": stderr_harness_id,
                        "profileId": stderr_profile_id,
                        "threadId": stderr_thread_id,
                        "message": safe_stderr(&line),
                    }),
                );
                let _ = stderr_events.send(BrokerEvent::Stderr { line });
            }
        }
    });

    let supervisor_id = harness_id.clone();
    tauri::async_runtime::spawn(async move {
        let status = tokio::select! {
            result = child.wait() => result,
            _ = stop_rx.recv() => {
                let _ = child.kill().await;
                child.wait().await
            }
        };

        app.state::<Broker>()
            .harnesses
            .lock()
            .await
            .remove(&supervisor_id);

        match status {
            Ok(status) => {
                diagnostics.record(
                    if status.success() { "info" } else { "error" },
                    "acp.harness.exited",
                    json!({
                        "harnessId": supervisor_id,
                        "profileId": request.profile_id,
                        "threadId": request.thread_id,
                        "code": status.code(),
                        "success": status.success(),
                    }),
                );
                let _ = on_event.send(BrokerEvent::Exited {
                    code: status.code(),
                    success: status.success(),
                });
            }
            Err(error) => {
                let message = format!("Could not observe the harness process: {error}");
                diagnostics.record(
                    "error",
                    "acp.harness.wait_failed",
                    json!({
                        "harnessId": supervisor_id,
                        "profileId": request.profile_id,
                        "threadId": request.thread_id,
                        "message": message,
                    }),
                );
                let _ = on_event.send(BrokerEvent::Error { message });
            }
        }
        let _ = stopped.send(true);
    });

    Ok(LaunchResult { harness_id })
}

#[tauri::command]
pub async fn send_rpc(
    broker: State<'_, Broker>,
    diagnostics: State<'_, Diagnostics>,
    harness_id: String,
    message: Value,
) -> Result<(), String> {
    if !message.is_object() {
        return Err("ACP messages must be JSON objects".into());
    }

    let (stdin, pending_requests, profile_id, thread_id) = broker
        .harnesses
        .lock()
        .await
        .get(&harness_id)
        .map(|handle| {
            (
                Arc::clone(&handle.stdin),
                Arc::clone(&handle.pending_requests),
                handle.profile_id.clone(),
                handle.thread_id.clone(),
            )
        })
        .ok_or("The harness is no longer running")?;
    if let (Some(method), Some(key)) = (
        message.get("method").and_then(Value::as_str),
        request_key(&message),
    ) {
        pending_requests.lock().await.insert(
            key,
            PendingRequest {
                method: method.to_owned(),
                started_at: Instant::now(),
            },
        );
    }
    diagnostics.record(
        "debug",
        "acp.rpc.sent",
        rpc_fields(
            &harness_id,
            profile_id.as_deref(),
            thread_id.as_deref(),
            "client_to_agent",
            &message,
        ),
    );
    let mut bytes = serde_json::to_vec(&message).map_err(|error| error.to_string())?;
    bytes.push(b'\n');

    let mut stdin = stdin.lock().await;
    if let Err(error) = stdin.write_all(&bytes).await {
        let message = format!("Could not write to the harness: {error}");
        diagnostics.record(
            "error",
            "acp.stdin.write_failed",
            json!({ "harnessId": harness_id, "message": message }),
        );
        return Err(message);
    }
    if let Err(error) = stdin.flush().await {
        let message = format!("Could not flush harness input: {error}");
        diagnostics.record(
            "error",
            "acp.stdin.flush_failed",
            json!({ "harnessId": harness_id, "message": message }),
        );
        return Err(message);
    }
    Ok(())
}

fn request_key(message: &Value) -> Option<String> {
    message.get("id").map(Value::to_string)
}

#[tauri::command]
pub async fn register_frontend(
    broker: State<'_, Broker>,
    terminals: State<'_, crate::terminal::TerminalManager>,
    diagnostics: State<'_, Diagnostics>,
) -> Result<u64, String> {
    let generation = broker.register_frontend(&diagnostics).await?;
    terminals
        .register_frontend(generation, &diagnostics)
        .await?;
    Ok(generation)
}

#[tauri::command]
pub async fn stop_harness(
    broker: State<'_, Broker>,
    diagnostics: State<'_, Diagnostics>,
    harness_id: String,
) -> Result<(), String> {
    let _lifecycle = broker.lifecycle.lock().await;
    let control = broker
        .harnesses
        .lock()
        .await
        .get(&harness_id)
        .map(HarnessHandle::stop_control);
    if let Some(control) = control {
        stop_harness_control(&diagnostics, &harness_id, control, "client_request").await?;
        broker.harnesses.lock().await.remove(&harness_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn stop_all_harnesses(
    broker: State<'_, Broker>,
    diagnostics: State<'_, Diagnostics>,
) -> Result<(), String> {
    broker.stop_all(&diagnostics, "client_request").await
}

impl Broker {
    async fn register_frontend(&self, diagnostics: &Diagnostics) -> Result<u64, String> {
        let _lifecycle = self.lifecycle.lock().await;
        if self.shutting_down.load(Ordering::Acquire) {
            return Err("LoopCode is shutting down".into());
        }
        let generation = self.frontend_generation.fetch_add(1, Ordering::AcqRel) + 1;
        self.stop_all_locked(diagnostics, "frontend_registered")
            .await?;
        diagnostics.record(
            "info",
            "acp.frontend.registered",
            json!({ "generation": generation }),
        );
        Ok(generation)
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
            .harnesses
            .lock()
            .await
            .iter()
            .map(|(harness_id, handle)| (harness_id.clone(), handle.stop_control()))
            .collect::<Vec<_>>();
        diagnostics.record(
            "info",
            "acp.harness.stop_all_requested",
            json!({ "count": controls.len(), "reason": reason }),
        );

        let mut first_error = None;
        for (harness_id, control) in controls {
            match stop_harness_control(diagnostics, &harness_id, control, reason).await {
                Ok(()) => {
                    self.harnesses.lock().await.remove(&harness_id);
                }
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

async fn stop_matching_harnesses(
    broker: &Broker,
    diagnostics: &Diagnostics,
    thread_id: Option<&str>,
    profile_id: Option<&str>,
) -> Result<(), String> {
    let (Some(thread_id), Some(profile_id)) = (thread_id, profile_id) else {
        return Ok(());
    };
    let matching = broker
        .harnesses
        .lock()
        .await
        .iter()
        .filter(|(_, handle)| {
            handle.thread_id.as_deref() == Some(thread_id)
                && handle.profile_id.as_deref() == Some(profile_id)
        })
        .map(|(harness_id, handle)| (harness_id.clone(), handle.stop_control()))
        .collect::<Vec<_>>();

    for (harness_id, control) in matching {
        diagnostics.record(
            "info",
            "acp.harness.replacing",
            json!({
                "harnessId": harness_id,
                "profileId": profile_id,
                "threadId": thread_id,
            }),
        );
        stop_harness_control(diagnostics, &harness_id, control, "replacement").await?;
        broker.harnesses.lock().await.remove(&harness_id);
    }
    Ok(())
}

async fn stop_harness_control(
    diagnostics: &Diagnostics,
    harness_id: &str,
    control: HarnessStop,
    reason: &str,
) -> Result<(), String> {
    diagnostics.record(
        "info",
        "acp.harness.stop_requested",
        json!({
            "harnessId": harness_id,
            "profileId": control.profile_id,
            "threadId": control.thread_id,
            "reason": reason,
        }),
    );
    wait_for_harness_stop(control.stop, control.stopped, harness_id).await?;
    diagnostics.record(
        "info",
        "acp.harness.stop_confirmed",
        json!({ "harnessId": harness_id, "reason": reason }),
    );
    Ok(())
}

async fn wait_for_harness_stop(
    stop: mpsc::Sender<()>,
    mut stopped: watch::Receiver<bool>,
    harness_id: &str,
) -> Result<(), String> {
    let _ = stop.try_send(());
    timeout(HARNESS_STOP_TIMEOUT, stopped.wait_for(|value| *value))
        .await
        .map_err(|_| format!("Timed out waiting for {harness_id} to stop"))?
        .map_err(|_| format!("Could not confirm that {harness_id} stopped"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::wait_for_harness_stop;
    use tokio::{
        sync::{mpsc, watch},
        time::{Duration, timeout},
    };

    #[tokio::test]
    async fn waits_for_process_exit_after_requesting_stop() {
        let (stop, mut stop_rx) = mpsc::channel(1);
        let (stopped, stopped_rx) = watch::channel(false);
        let mut waiting =
            tokio::spawn(
                async move { wait_for_harness_stop(stop, stopped_rx, "harness-test").await },
            );

        stop_rx.recv().await.expect("stop should be requested");
        assert!(
            timeout(Duration::from_millis(10), &mut waiting)
                .await
                .is_err()
        );

        stopped.send(true).expect("waiter should remain subscribed");
        waiting
            .await
            .expect("wait task should finish")
            .expect("stop should be confirmed");
    }
}
