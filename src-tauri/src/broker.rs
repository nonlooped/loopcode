use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    path::Path,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tauri::{AppHandle, Manager, State, ipc::Channel};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{ChildStdin, Command},
    sync::{Mutex, mpsc},
};

#[derive(Default)]
pub struct Broker {
    next_id: AtomicU64,
    harnesses: Mutex<HashMap<String, HarnessHandle>>,
}

struct HarnessHandle {
    stdin: Arc<Mutex<ChildStdin>>,
    stop: mpsc::Sender<()>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    command: String,
    args: Vec<String>,
    cwd: String,
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

    broker.harnesses.lock().await.insert(
        harness_id.clone(),
        HarnessHandle {
            stdin: Arc::clone(&stdin),
            stop,
        },
    );

    let stdout_events = on_event.clone();
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) if line.trim().is_empty() => {}
                Ok(Some(line)) => match serde_json::from_str::<Value>(&line) {
                    Ok(message) => {
                        let _ = stdout_events.send(BrokerEvent::Rpc { message });
                    }
                    Err(error) => {
                        let _ = stdout_events.send(BrokerEvent::Error {
                            message: format!("The harness returned invalid ACP JSON: {error}"),
                        });
                    }
                },
                Ok(None) => break,
                Err(error) => {
                    let _ = stdout_events.send(BrokerEvent::Error {
                        message: format!("Could not read harness output: {error}"),
                    });
                    break;
                }
            }
        }
    });

    let stderr_events = on_event.clone();
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if !line.trim().is_empty() {
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
                let _ = on_event.send(BrokerEvent::Exited {
                    code: status.code(),
                    success: status.success(),
                });
            }
            Err(error) => {
                let _ = on_event.send(BrokerEvent::Error {
                    message: format!("Could not observe the harness process: {error}"),
                });
            }
        }
    });

    Ok(LaunchResult { harness_id })
}

#[tauri::command]
pub async fn send_rpc(
    broker: State<'_, Broker>,
    harness_id: String,
    message: Value,
) -> Result<(), String> {
    if !message.is_object() {
        return Err("ACP messages must be JSON objects".into());
    }

    let stdin = broker
        .harnesses
        .lock()
        .await
        .get(&harness_id)
        .map(|handle| Arc::clone(&handle.stdin))
        .ok_or("The harness is no longer running")?;
    let mut bytes = serde_json::to_vec(&message).map_err(|error| error.to_string())?;
    bytes.push(b'\n');

    let mut stdin = stdin.lock().await;
    stdin
        .write_all(&bytes)
        .await
        .map_err(|error| format!("Could not write to the harness: {error}"))?;
    stdin
        .flush()
        .await
        .map_err(|error| format!("Could not flush harness input: {error}"))
}

#[tauri::command]
pub async fn stop_harness(broker: State<'_, Broker>, harness_id: String) -> Result<(), String> {
    let handle = broker.harnesses.lock().await.remove(&harness_id);
    if let Some(handle) = handle {
        handle
            .stop
            .send(())
            .await
            .map_err(|_| "The harness already stopped")?;
    }
    Ok(())
}

#[tauri::command]
pub async fn stop_all_harnesses(broker: State<'_, Broker>) -> Result<(), String> {
    let handles = broker
        .harnesses
        .lock()
        .await
        .drain()
        .map(|(_, handle)| handle)
        .collect::<Vec<_>>();

    for handle in handles {
        let _ = handle.stop.send(()).await;
    }
    Ok(())
}
