use serde_json::{Map, Value, json};
use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, OpenOptions},
    hash::{Hash, Hasher},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const LOG_FILE_NAME: &str = "acp.jsonl";
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const RETAINED_LOGS: usize = 5;
const MAX_STRING_CHARS: usize = 2_000;
const RECORD_QUEUE_CAPACITY: usize = 2_048;
const FLUSH_INTERVAL: Duration = Duration::from_secs(1);

type CommandResult = Result<(), String>;

enum WriterCommand {
    Record {
        line: String,
        flush: bool,
    },
    Export {
        destination: PathBuf,
        completed: mpsc::Sender<CommandResult>,
    },
    Shutdown {
        completed: mpsc::Sender<CommandResult>,
    },
}

struct DiagnosticsInner {
    sender: SyncSender<WriterCommand>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Clone)]
pub struct Diagnostics {
    inner: Arc<DiagnosticsInner>,
}

impl Diagnostics {
    pub fn new(directory: PathBuf) -> Self {
        let (sender, receiver) = mpsc::sync_channel(RECORD_QUEUE_CAPACITY);
        let worker = thread::Builder::new()
            .name("loopcode-diagnostics".into())
            .spawn(move || writer_loop(directory, receiver))
            .expect("diagnostics writer thread should start");
        Self {
            inner: Arc::new(DiagnosticsInner {
                sender,
                worker: Mutex::new(Some(worker)),
            }),
        }
    }

    pub fn record(&self, level: &str, event: &str, fields: Value) {
        let mut entry = Map::new();
        entry.insert("timestampMs".into(), json!(timestamp_ms()));
        entry.insert("level".into(), json!(level));
        entry.insert("event".into(), json!(event));
        if let Value::Object(fields) = sanitize_value(fields) {
            entry.extend(fields);
        }
        let Ok(line) = serde_json::to_string(&entry) else {
            return;
        };
        let flush = level == "error"
            || matches!(
                event,
                "acp.harness.started"
                    | "acp.harness.exited"
                    | "acp.frontend.registered"
                    | "diagnostics.exported"
            )
            || event.starts_with("acp.harness.stop");
        let command = WriterCommand::Record { line, flush };
        if event.starts_with("acp.rpc.") && level != "error" {
            match self.inner.sender.try_send(command) {
                Ok(()) | Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {}
            }
        } else {
            let _ = self.inner.sender.send(command);
        }
    }

    pub fn export_to(&self, destination: &Path) -> Result<(), String> {
        let (completed, response) = mpsc::channel();
        self.inner
            .sender
            .send(WriterCommand::Export {
                destination: destination.to_owned(),
                completed,
            })
            .map_err(|_| "The diagnostics log is unavailable".to_owned())?;
        response
            .recv()
            .map_err(|_| "The diagnostics export did not complete".to_owned())?
    }

    pub fn shutdown(&self) -> Result<(), String> {
        let mut worker = self
            .inner
            .worker
            .lock()
            .map_err(|_| "The diagnostics writer is unavailable".to_owned())?;
        let Some(handle) = worker.take() else {
            return Ok(());
        };
        let (completed, response) = mpsc::channel();
        self.inner
            .sender
            .send(WriterCommand::Shutdown { completed })
            .map_err(|_| "The diagnostics writer stopped unexpectedly".to_owned())?;
        let result = response
            .recv()
            .map_err(|_| "The diagnostics shutdown did not complete".to_owned())?;
        handle
            .join()
            .map_err(|_| "The diagnostics writer thread panicked".to_owned())?;
        result
    }
}

struct WriterState {
    directory: PathBuf,
    output: Option<BufWriter<fs::File>>,
    bytes_written: u64,
}

impl WriterState {
    fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            output: None,
            bytes_written: 0,
        }
    }

    fn write_line(&mut self, line: &str) -> CommandResult {
        let line_bytes = line.len() as u64 + 1;
        self.ensure_open()?;
        if self.bytes_written > 0 && self.bytes_written + line_bytes > MAX_LOG_BYTES {
            self.flush()?;
            self.output = None;
            rotate(&self.directory);
            self.bytes_written = 0;
            self.ensure_open()?;
        }
        let output = self.output.as_mut().expect("diagnostics output is open");
        writeln!(output, "{line}")
            .map_err(|error| format!("Could not write the diagnostics log: {error}"))?;
        self.bytes_written += line_bytes;
        Ok(())
    }

    fn ensure_open(&mut self) -> CommandResult {
        if self.output.is_some() {
            return Ok(());
        }
        fs::create_dir_all(&self.directory)
            .map_err(|error| format!("Could not create the diagnostics directory: {error}"))?;
        let path = self.directory.join(LOG_FILE_NAME);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("Could not open the diagnostics log: {error}"))?;
        self.bytes_written = file.metadata().map_or(0, |metadata| metadata.len());
        self.output = Some(BufWriter::new(file));
        Ok(())
    }

    fn flush(&mut self) -> CommandResult {
        if let Some(output) = &mut self.output {
            output
                .flush()
                .map_err(|error| format!("Could not flush the diagnostics log: {error}"))?;
        }
        Ok(())
    }

    fn export_to(&mut self, destination: &Path) -> CommandResult {
        self.flush()?;
        let mut output = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(destination)
            .map_err(|error| format!("Could not create the diagnostics export: {error}"))?;

        for index in (1..RETAINED_LOGS).rev() {
            append_file(
                &self.directory.join(format!("{LOG_FILE_NAME}.{index}")),
                &mut output,
            )?;
        }
        append_file(&self.directory.join(LOG_FILE_NAME), &mut output)?;
        output
            .flush()
            .map_err(|error| format!("Could not flush the diagnostics export: {error}"))
    }
}

fn writer_loop(directory: PathBuf, receiver: Receiver<WriterCommand>) {
    let mut state = WriterState::new(directory);
    loop {
        match receiver.recv_timeout(FLUSH_INTERVAL) {
            Ok(WriterCommand::Record { line, flush }) => {
                if state.write_line(&line).is_ok() && flush {
                    let _ = state.flush();
                }
            }
            Ok(WriterCommand::Export {
                destination,
                completed,
            }) => {
                let _ = completed.send(state.export_to(&destination));
            }
            Ok(WriterCommand::Shutdown { completed }) => {
                let _ = completed.send(state.flush());
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                let _ = state.flush();
            }
            Err(RecvTimeoutError::Disconnected) => {
                let _ = state.flush();
                break;
            }
        }
    }
}

pub fn rpc_fields(
    harness_id: &str,
    profile_id: Option<&str>,
    thread_id: Option<&str>,
    direction: &str,
    message: &Value,
) -> Value {
    let mut fields = Map::new();
    fields.insert("harnessId".into(), json!(harness_id));
    fields.insert("profileId".into(), json!(profile_id));
    fields.insert("threadId".into(), json!(thread_id));
    fields.insert("direction".into(), json!(direction));
    if let Some(method) = message.get("method").and_then(Value::as_str) {
        fields.insert("method".into(), json!(method));
        fields.insert(
            "kind".into(),
            json!(if message.get("id").is_some() {
                "request"
            } else {
                "notification"
            }),
        );
    } else {
        fields.insert("kind".into(), json!("response"));
    }
    if let Some(id) = message.get("id") {
        fields.insert("requestId".into(), sanitize_value(id.clone()));
    }
    if let Some(error) = message.get("error") {
        fields.insert("error".into(), sanitize_value(error.clone()));
    }
    let session_id = message
        .pointer("/params/sessionId")
        .or_else(|| message.pointer("/result/sessionId"))
        .and_then(Value::as_str);
    if let Some(session_id) = session_id {
        let mut hasher = DefaultHasher::new();
        session_id.hash(&mut hasher);
        fields.insert(
            "sessionIdHash".into(),
            json!(format!("{:016x}", hasher.finish())),
        );
    }
    Value::Object(fields)
}

pub fn safe_stderr(line: &str) -> String {
    redact_text(line)
}

fn sanitize_value(value: Value) -> Value {
    match value {
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| {
                    let lower = key.to_ascii_lowercase();
                    let value = if [
                        "authorization",
                        "apikey",
                        "api_key",
                        "password",
                        "token",
                        "prompt",
                        "content",
                        "patch",
                    ]
                    .iter()
                    .any(|sensitive| lower.contains(sensitive))
                    {
                        json!("<REDACTED>")
                    } else {
                        sanitize_value(value)
                    };
                    (key, value)
                })
                .collect(),
        ),
        Value::Array(values) => {
            Value::Array(values.into_iter().take(50).map(sanitize_value).collect())
        }
        Value::String(value) => Value::String(redact_text(&value)),
        value => value,
    }
}

fn redact_text(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    if [
        "authorization",
        "api_key",
        "apikey",
        "bearer ",
        "token=",
        "password",
        "sk-",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        "<REDACTED sensitive text>".to_owned()
    } else {
        truncate(value)
    }
}

fn truncate(value: &str) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(MAX_STRING_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}

fn rotate(directory: &Path) {
    let oldest = directory.join(format!("{LOG_FILE_NAME}.{}", RETAINED_LOGS - 1));
    let _ = fs::remove_file(oldest);
    for index in (1..RETAINED_LOGS - 1).rev() {
        let from = directory.join(format!("{LOG_FILE_NAME}.{index}"));
        let to = directory.join(format!("{LOG_FILE_NAME}.{}", index + 1));
        let _ = fs::rename(from, to);
    }
    let _ = fs::rename(
        directory.join(LOG_FILE_NAME),
        directory.join(format!("{LOG_FILE_NAME}.1")),
    );
}

fn append_file(path: &Path, output: &mut impl Write) -> Result<(), String> {
    if !path.is_file() {
        return Ok(());
    }
    let mut input = fs::File::open(path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    let mut buffer = Vec::new();
    input
        .read_to_end(&mut buffer)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    output
        .write_all(&buffer)
        .map_err(|error| format!("Could not write the diagnostics export: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{Diagnostics, rpc_fields, safe_stderr};
    use serde_json::json;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT_TEST_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let id = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("loopcode-diagnostics-{}-{id}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn rpc_envelopes_exclude_prompt_content_and_keep_error_metadata() {
        let sent = rpc_fields(
            "harness-1",
            Some("codex"),
            Some("thread-1"),
            "client_to_agent",
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "session/prompt",
                "params": { "prompt": "private request" }
            }),
        );
        assert_eq!(sent.get("method"), Some(&json!("session/prompt")));
        assert!(sent.get("params").is_none());
        assert!(!sent.to_string().contains("private request"));

        let received = rpc_fields(
            "harness-1",
            Some("codex"),
            Some("thread-1"),
            "agent_to_client",
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "error": {
                    "code": -32603,
                    "message": "Internal error",
                    "data": { "detail": "turn active", "token": "secret" }
                }
            }),
        );
        assert_eq!(received["error"]["code"], json!(-32603));
        assert_eq!(received["error"]["data"]["detail"], json!("turn active"));
        assert_eq!(received["error"]["data"]["token"], json!("<REDACTED>"));
    }

    #[test]
    fn stderr_and_exports_are_redacted() {
        assert_eq!(
            safe_stderr("Authorization: Bearer secret"),
            "<REDACTED sensitive text>"
        );
        let directory = TestDirectory::new();
        let diagnostics = Diagnostics::new(directory.0.clone());
        diagnostics.record(
            "error",
            "acp.client.error",
            json!({ "message": "failed", "prompt": "private request", "apiKey": "secret" }),
        );
        let export = directory.0.join("export.jsonl");
        diagnostics
            .export_to(&export)
            .expect("diagnostics should export");
        let contents = fs::read_to_string(export).expect("export should be readable");
        assert!(contents.contains("acp.client.error"));
        assert!(!contents.contains("private request"));
        assert!(!contents.contains("secret"));
        diagnostics.shutdown().expect("writer should stop");
    }

    #[test]
    fn export_flushes_all_buffered_records() {
        let directory = TestDirectory::new();
        let diagnostics = Diagnostics::new(directory.0.clone());
        for request_id in 0..32 {
            diagnostics.record("debug", "acp.rpc.sent", json!({ "requestId": request_id }));
        }

        let export = directory.0.join("export.jsonl");
        diagnostics.export_to(&export).expect("export should flush");
        let contents = fs::read_to_string(export).expect("export should be readable");
        assert_eq!(contents.lines().count(), 32);
        diagnostics.shutdown().expect("writer should stop");
    }

    #[test]
    fn orderly_shutdown_flushes_the_persistent_handle() {
        let directory = TestDirectory::new();
        let diagnostics = Diagnostics::new(directory.0.clone());
        diagnostics.record("debug", "acp.rpc.sent", json!({ "requestId": 7 }));

        diagnostics
            .shutdown()
            .expect("writer should flush and stop");

        let contents = fs::read_to_string(directory.0.join("acp.jsonl"))
            .expect("shutdown log should be readable");
        assert!(contents.contains("\"requestId\":7"));
    }
}
