//! `webfetch` — single URL fetch with size caps (always approval-gated).
//! HTTPS via reqwest (rustls). Tests may set `LOOPCODE_FETCH_FIXTURE` or inject
//! a custom transport via `web_fetch_with_transport`.

use crate::providers::adapter::HttpRequestSpec;
use crate::providers::transport::{HttpTransport, LiveHttpTransport};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

const MAX_BODY_BYTES: usize = 256 * 1024;

/// Fetch a URL. For tests, set `LOOPCODE_FETCH_FIXTURE` to return a canned body
/// without network. Production uses HTTPS-capable reqwest.
pub fn web_fetch(args: &Value) -> Result<Value, String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("url required")?;
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("only http/https URLs allowed".into());
    }

    if let Ok(fixture) = std::env::var("LOOPCODE_FETCH_FIXTURE") {
        let body = if fixture.len() > MAX_BODY_BYTES {
            fixture.chars().take(MAX_BODY_BYTES).collect::<String>()
        } else {
            fixture
        };
        return Ok(json!({
            "status": "ok",
            "url": url,
            "statusCode": 200,
            "contentType": "text/plain",
            "body": body,
            "truncated": false,
            "fixture": true,
        }));
    }

    let transport = LiveHttpTransport::new()?;
    web_fetch_with_transport(url, &transport)
}

/// Shared path for live + test transports (same size caps and response shaping).
pub fn web_fetch_with_transport(
    url: &str,
    transport: &dyn HttpTransport,
) -> Result<Value, String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("only http/https URLs allowed".into());
    }
    let mut headers = HashMap::new();
    headers.insert(
        "User-Agent".into(),
        format!("LoopCode/{}", env!("CARGO_PKG_VERSION")),
    );
    headers.insert("Accept".into(), "*/*".into());
    let req = HttpRequestSpec {
        method: "GET".into(),
        url: url.to_string(),
        headers,
        body: Value::Null,
    };
    let (status_code, body_raw) = transport.execute(&req)?;
    let truncated = body_raw.len() > MAX_BODY_BYTES;
    let body = if truncated {
        body_raw.chars().take(MAX_BODY_BYTES).collect::<String>()
    } else {
        body_raw
    };
    Ok(json!({
        "status": "ok",
        "url": url,
        "statusCode": status_code,
        "contentType": "application/octet-stream",
        "body": body,
        "truncated": truncated,
        "fixture": false,
        "https": url.starts_with("https://"),
    }))
}

/// Test helper: bound on request timeout (documents production default).
pub fn fetch_timeout() -> Duration {
    Duration::from_secs(30)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::transport::FixtureHttpTransport;

    #[test]
    fn https_path_uses_shared_transport() {
        let t = FixtureHttpTransport::single(200, "secure body");
        let out = web_fetch_with_transport("https://example.test/x", &t).unwrap();
        assert_eq!(out["status"], "ok");
        assert_eq!(out["body"], "secure body");
        assert_eq!(out["https"], true);
        assert_eq!(out["statusCode"], 200);
    }

    #[test]
    fn rejects_non_http_schemes() {
        let t = FixtureHttpTransport::single(200, "x");
        let err = web_fetch_with_transport("ftp://x", &t).unwrap_err();
        assert!(err.contains("http"));
    }
}
