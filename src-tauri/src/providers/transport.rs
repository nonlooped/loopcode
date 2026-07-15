//! Provider HTTP transport — live HTTPS + deterministic fixtures for tests.

use crate::providers::adapter::HttpRequestSpec;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Execute an HTTP request (probe or agent turn). Secrets must never be logged.
pub trait HttpTransport: Send + Sync {
    fn execute(&self, req: &HttpRequestSpec) -> Result<(u16, String), String>;

    /// Cooperative cancel for delayed / multi-chunk fixture transports.
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Live HTTPS (and HTTP) via reqwest blocking client.
pub struct LiveHttpTransport {
    client: reqwest::blocking::Client,
    cancelled: Arc<AtomicBool>,
}

impl LiveHttpTransport {
    pub fn new() -> Result<Self, String> {
        Self::with_cancel(Arc::new(AtomicBool::new(false)))
    }

    pub fn with_cancel(cancelled: Arc<AtomicBool>) -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(30))
            .user_agent(concat!("LoopCode/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        Ok(Self { client, cancelled })
    }

    pub fn cancelled_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }

    pub fn request_cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }
}

impl Default for LiveHttpTransport {
    fn default() -> Self {
        Self::new().expect("default LiveHttpTransport")
    }
}

impl HttpTransport for LiveHttpTransport {
    fn execute(&self, req: &HttpRequestSpec) -> Result<(u16, String), String> {
        if self.cancelled.load(Ordering::SeqCst) {
            return Err("request cancelled".into());
        }
        let method = match req.method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "PUT" => reqwest::Method::PUT,
            "PATCH" => reqwest::Method::PATCH,
            "DELETE" => reqwest::Method::DELETE,
            _ => reqwest::Method::POST,
        };
        let mut builder = self.client.request(method, &req.url);
        for (k, v) in &req.headers {
            // Never put secrets into logs; headers are only used on the wire.
            builder = builder.header(k.as_str(), v.as_str());
        }
        if !req.body.is_null() {
            builder = builder.json(&req.body);
        }
        let resp = builder.send().map_err(|e| {
            if self.cancelled.load(Ordering::SeqCst) {
                "request cancelled".to_string()
            } else {
                format!("network: {e}")
            }
        })?;
        if self.cancelled.load(Ordering::SeqCst) {
            return Err("request cancelled".into());
        }
        let status = resp.status().as_u16();
        let body = resp.text().map_err(|e| format!("read body: {e}"))?;
        Ok((status, body))
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Scripted fixture transport for CI and unit tests (no network).
pub struct FixtureHttpTransport {
    queue: Mutex<VecDeque<(u16, String)>>,
    /// Optional per-chunk delay so cancel can race mid-request.
    chunk_delay: Option<Duration>,
    cancelled: Arc<AtomicBool>,
}

impl FixtureHttpTransport {
    pub fn new(responses: Vec<(u16, String)>) -> Self {
        Self {
            queue: Mutex::new(VecDeque::from(responses)),
            chunk_delay: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn single(status: u16, body: impl Into<String>) -> Self {
        Self::new(vec![(status, body.into())])
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.chunk_delay = Some(delay);
        self
    }

    pub fn with_cancel_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.cancelled = flag;
        self
    }

    pub fn cancelled_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }

    pub fn request_cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn remaining(&self) -> usize {
        self.queue.lock().map(|q| q.len()).unwrap_or(0)
    }
}

impl HttpTransport for FixtureHttpTransport {
    fn execute(&self, _req: &HttpRequestSpec) -> Result<(u16, String), String> {
        if let Some(delay) = self.chunk_delay {
            // Sleep in small slices so cancel is observed without waiting full delay.
            let slice = Duration::from_millis(20);
            let mut left = delay;
            while left > Duration::ZERO {
                if self.cancelled.load(Ordering::SeqCst) {
                    return Err("request cancelled".into());
                }
                let step = if left > slice { slice } else { left };
                thread::sleep(step);
                left = left.saturating_sub(step);
            }
        }
        if self.cancelled.load(Ordering::SeqCst) {
            return Err("request cancelled".into());
        }
        let mut q = self
            .queue
            .lock()
            .map_err(|_| "fixture transport lock poisoned".to_string())?;
        q.pop_front()
            .ok_or_else(|| "fixture transport exhausted".to_string())
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Whether live network probes/agent turns are enabled.
///
/// Default: live HTTPS when not under cargo test; fixtures unless
/// `LOOPCODE_LIVE_PROBE=1` or `LOOPCODE_LIVE_HTTP=1`.
pub fn live_http_enabled() -> bool {
    matches!(
        std::env::var("LOOPCODE_LIVE_PROBE")
            .or_else(|_| std::env::var("LOOPCODE_LIVE_HTTP"))
            .as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// Build the default production transport (live when enabled, else fixture probe-compatible).
pub fn default_agent_transport(
    cancel: Arc<AtomicBool>,
) -> Result<Arc<dyn HttpTransport>, String> {
    if live_http_enabled() {
        Ok(Arc::new(LiveHttpTransport::with_cancel(cancel)?))
    } else {
        // Production without explicit live flag still uses live HTTPS for agent
        // turns when a real API key is present — fixtures are for probes/tests.
        // Agent entry points pass Live when secrets exist; this helper is for
        // explicit live-mode paths.
        Ok(Arc::new(LiveHttpTransport::with_cancel(cancel)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::adapter::HttpRequestSpec;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn fixture_returns_scripted_response() {
        let t = FixtureHttpTransport::single(200, r#"{"ok":true}"#);
        let (status, body) = t
            .execute(&HttpRequestSpec {
                method: "POST".into(),
                url: "https://example.test/v1".into(),
                headers: HashMap::new(),
                body: json!({}),
            })
            .unwrap();
        assert_eq!(status, 200);
        assert!(body.contains("ok"));
    }

    #[test]
    fn fixture_cancel_during_delay() {
        let flag = Arc::new(AtomicBool::new(false));
        let t = FixtureHttpTransport::single(200, "late")
            .with_delay(Duration::from_secs(5))
            .with_cancel_flag(Arc::clone(&flag));
        let handle = thread::spawn({
            let t = FixtureHttpTransport::single(200, "late")
                .with_delay(Duration::from_secs(5))
                .with_cancel_flag(Arc::clone(&flag));
            move || {
                t.execute(&HttpRequestSpec {
                    method: "POST".into(),
                    url: "https://example.test".into(),
                    headers: HashMap::new(),
                    body: json!({}),
                })
            }
        });
        thread::sleep(Duration::from_millis(50));
        flag.store(true, Ordering::SeqCst);
        let err = handle.join().unwrap().unwrap_err();
        assert!(err.contains("cancel"), "{err}");
        let _ = t; // keep
    }
}
