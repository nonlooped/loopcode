use serde_json::{Map, Value, json};
use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, OpenOptions},
    hash::{Hash, Hasher},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    time::{SystemTime, UNIX_EPOCH},
};

const LOG_FILE_NAME: &str = "acp.jsonl";
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const RETAINED_LOGS: usize = 5;
const MAX_STRING_CHARS: usize = 2_000;
const LOG_QUEUE_CAPACITY: usize = 1024;

enum DiagnosticCommand {
    Record {
        level: String,
        event: String,
        fields: Value,
    },
    Flush(mpsc::Sender<()>),
}

#[derive(Clone)]
pub struct Diagnostics {
    directory: PathBuf,
    write_lock: Arc<Mutex<()>>,
    sender: mpsc::SyncSender<DiagnosticCommand>,
}

impl Diagnostics {
    pub fn new(directory: PathBuf) -> Self {
        let write_lock = Arc::new(Mutex::new(()));
        let (sender, receiver) = mpsc::sync_channel(LOG_QUEUE_CAPACITY);
        let writer_directory = directory.clone();
        let writer_lock = Arc::clone(&write_lock);
        let _ = std::thread::Builder::new()
            .name("loopcode-diagnostics".into())
            .spawn(move || diagnostics_writer(receiver, &writer_directory, &writer_lock));
        Self {
            directory,
            write_lock,
            sender,
        }
    }

    pub fn record(&self, level: &str, event: &str, fields: Value) {
        let _ = self.sender.try_send(DiagnosticCommand::Record {
            level: level.to_owned(),
            event: event.to_owned(),
            fields,
        });
    }

    pub fn export_to(&self, destination: &Path) -> Result<(), String> {
        let (flushed, confirmation) = mpsc::channel();
        self.sender
            .send(DiagnosticCommand::Flush(flushed))
            .map_err(|_| "The diagnostics writer stopped".to_owned())?;
        confirmation
            .recv()
            .map_err(|_| "The diagnostics writer stopped".to_owned())?;
        let _guard = self
            .write_lock
            .lock()
            .map_err(|_| "The diagnostics log is unavailable".to_owned())?;
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
        Ok(())
    }
}

fn diagnostics_writer(
    receiver: mpsc::Receiver<DiagnosticCommand>,
    directory: &Path,
    write_lock: &Mutex<()>,
) {
    while let Ok(command) = receiver.recv() {
        match command {
            DiagnosticCommand::Record {
                level,
                event,
                fields,
            } => {
                let Ok(_guard) = write_lock.lock() else {
                    continue;
                };
                write_record(directory, &level, &event, fields);
            }
            DiagnosticCommand::Flush(confirmation) => {
                let _ = confirmation.send(());
            }
        }
    }
}

fn write_record(directory: &Path, level: &str, event: &str, fields: Value) {
    if fs::create_dir_all(directory).is_err() {
        return;
    }
    let path = directory.join(LOG_FILE_NAME);
    if path
        .metadata()
        .is_ok_and(|metadata| metadata.len() >= MAX_LOG_BYTES)
    {
        rotate(directory);
    }

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
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
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
        Value::String(value) => Value::String(safe_stderr(&value)),
        value => value,
    }
}

pub(crate) fn safe_stderr(value: &str) -> String {
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
    use std::fs;

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
        let directory = tempfile::tempdir().expect("test directory should be created");
        let diagnostics = Diagnostics::new(directory.path().to_path_buf());
        diagnostics.record(
            "error",
            "acp.client.error",
            json!({ "message": "failed", "prompt": "private request", "apiKey": "secret" }),
        );
        let export = directory.path().join("export.jsonl");
        diagnostics
            .export_to(&export)
            .expect("diagnostics should export");
        let contents = fs::read_to_string(export).expect("export should be readable");
        assert!(contents.contains("acp.client.error"));
        assert!(!contents.contains("private request"));
        assert!(!contents.contains("secret"));
    }
}
