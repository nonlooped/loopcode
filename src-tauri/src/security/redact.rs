//! Shared secret-shaped redaction for export, durable events, and provider history.

use crate::security::path::{classify_path, PathClass};
use serde_json::Value;
use std::path::Path;

/// Keys whose values are always replaced in JSON payloads.
const SENSITIVE_KEYS: &[&str] = &[
    "password",
    "secret",
    "api_key",
    "apikey",
    "access_token",
    "refresh_token",
    "authorization",
    "token",
    "private_key",
    "client_secret",
];

fn is_sensitive_key(key: &str) -> bool {
    // Normalize to lowercase snake_case so camelCase keys like `apiKey`,
    // `accessToken`, `refreshToken`, `clientSecret` match the snake_case
    // entries in SENSITIVE_KEYS. Without this, `accessToken` lowercases to
    // `accesstoken` and never matches `access_token`.
    let lower = camel_to_snake(key);
    for s in SENSITIVE_KEYS {
        if lower == *s {
            return true;
        }
        // "token" must not match usage fields like input_tokens / total_tokens.
        if *s == "token" {
            if lower.ends_with("_token") || lower.starts_with("token_") || lower.contains("_token_")
            {
                return true;
            }
            continue;
        }
        if lower.contains(s) {
            return true;
        }
    }
    false
}

/// Convert a key to lowercase snake_case: `apiKey` → `api_key`,
/// `accessToken` → `access_token`, `API-Key` → `api_key`.
fn camel_to_snake(key: &str) -> String {
    let mut out = String::with_capacity(key.len() + 4);
    let mut prev_lower = false;
    for (i, c) in key.chars().enumerate() {
        let lower = c.to_ascii_lowercase();
        if i > 0 && c.is_ascii_uppercase() && prev_lower {
            out.push('_');
        }
        if c == '-' {
            out.push('_');
        } else {
            out.push(lower);
        }
        prev_lower = lower.is_ascii_lowercase() || lower.is_ascii_digit();
    }
    out
}

/// Redact free-form text (pattern stub, no regex dependency).
pub fn redact_text(input: &str) -> String {
    let mut out = input.to_string();

    out = redact_prefix_token(&out, "sk-");
    out = redact_bearer(&out);
    for label in [
        "api_key",
        "api-key",
        "access_token",
        "access-token",
        "refresh_token",
        "client_secret",
        "private_key",
        "authorization",
        "secret",
        "password",
        "token",
    ] {
        out = redact_assignment(&out, label);
    }
    out
}

fn redact_prefix_token(input: &str, prefix: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let lower_prefix = prefix.to_ascii_lowercase();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &input[i..];
        let rest_lower = rest.to_ascii_lowercase();
        if let Some(pos) = rest_lower.find(&lower_prefix) {
            result.push_str(&rest[..pos]);
            let start = pos + prefix.len();
            let token_rest = &rest[start..];
            let end = token_rest
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
                .unwrap_or(token_rest.len());
            if end >= 10 {
                result.push_str("[REDACTED]");
                i += pos + prefix.len() + end;
            } else {
                result.push_str(&rest[..pos + prefix.len()]);
                i += pos + prefix.len();
            }
        } else {
            result.push_str(rest);
            break;
        }
    }
    result
}

fn redact_bearer(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if let Some(rel) = lower[i..].find("bearer ") {
            let abs = i + rel;
            result.push_str(&input[i..abs]);
            result.push_str("[REDACTED]");
            let after = abs + "bearer ".len();
            let rest = &input[after..];
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                .unwrap_or(rest.len());
            i = after + end;
        } else {
            result.push_str(&input[i..]);
            break;
        }
    }
    result
}

fn redact_assignment(input: &str, label: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let label_l = label.to_ascii_lowercase();
    let mut result = String::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if let Some(rel) = lower[i..].find(&label_l) {
            let abs = i + rel;
            result.push_str(&input[i..abs]);
            result.push_str(label);
            let after_label = abs + label.len();
            let rest = &input[after_label..];
            let trimmed = rest.trim_start();
            let ws = rest.len() - trimmed.len();
            if let Some(stripped) = trimmed
                .strip_prefix('=')
                .or_else(|| trimmed.strip_prefix(':'))
            {
                result.push_str(&rest[..ws]);
                let sep = if trimmed.starts_with('=') { "=" } else { ":" };
                result.push_str(sep);
                let val = stripped.trim_start();
                let val_ws = stripped.len() - val.len();
                result.push_str(&stripped[..val_ws]);
                result.push_str("[REDACTED]");
                let end = val
                    .find(|c: char| c.is_whitespace() || c == ',' || c == '"' || c == '}')
                    .unwrap_or(val.len());
                i = after_label + ws + 1 + val_ws + end;
            } else {
                result.push_str(&input[abs..after_label]);
                i = after_label;
            }
        } else {
            result.push_str(&input[i..]);
            break;
        }
    }
    result
}

/// Deep-redact a JSON value (object keys + string leaves).
pub fn redact_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if is_sensitive_key(k) {
                    out.insert(k.clone(), Value::String("[REDACTED]".into()));
                } else {
                    out.insert(k.clone(), redact_json_value(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(redact_json_value).collect()),
        Value::String(s) => Value::String(redact_text(s)),
        other => other.clone(),
    }
}

/// Redact a payload JSON string; if parse fails, apply text redaction.
pub fn redact_payload_json(payload: &str) -> String {
    match serde_json::from_str::<Value>(payload) {
        Ok(v) => redact_json_value(&v).to_string(),
        Err(_) => redact_text(payload),
    }
}

/// Redact tool result JSON before persisting or feeding provider history.
///
/// Applies key/pattern redaction and, when the tool targeted a protected workspace
/// path, replaces file body fields even after an approval grant.
pub fn redact_tool_result_content(
    content: &Value,
    project_root: Option<&Path>,
    tool_path: Option<&str>,
) -> Value {
    let mut redacted = redact_json_value(content);
    if let (Some(root), Some(rel)) = (project_root, tool_path) {
        if classify_path(root, rel).class == PathClass::ProtectedInWorkspace {
            if let Value::Object(ref mut map) = redacted {
                if map.contains_key("content") {
                    map.insert(
                        "content".into(),
                        Value::String("[REDACTED protected path]".into()),
                    );
                }
            }
        }
    }
    redacted
}

fn tool_path_from_args(args: &Value) -> Option<&str> {
    args.get("path").and_then(|v| v.as_str())
}

/// Convenience wrapper using tool arguments for path extraction.
pub fn redact_tool_result_for_event(
    content: &Value,
    project_root: Option<&Path>,
    tool_args: &Value,
) -> Value {
    redact_tool_result_content(content, project_root, tool_path_from_args(tool_args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn redacts_api_key_field() {
        let v = json!({"message": "hi", "api_key": "sk-secretvalue12345"});
        let r = redact_json_value(&v);
        assert_eq!(r["api_key"], "[REDACTED]");
        assert_eq!(r["message"], "hi");
    }

    #[test]
    fn redacts_camelcase_keys() {
        let v = json!({
            "apiKey": "sk-camel12345",
            "accessToken": "tok",
            "refreshToken": "rtok",
            "clientSecret": "cs",
            "password": "pw",
        });
        let r = redact_json_value(&v);
        assert_eq!(r["apiKey"], "[REDACTED]", "apiKey should be redacted");
        assert_eq!(r["accessToken"], "[REDACTED]");
        assert_eq!(r["refreshToken"], "[REDACTED]");
        assert_eq!(r["clientSecret"], "[REDACTED]");
        assert_eq!(r["password"], "[REDACTED]");
    }

    #[test]
    fn does_not_redact_usage_token_fields() {
        // camelCase usage fields must not match the "token" sentinel.
        let v = json!({
            "inputTokens": 100,
            "outputTokens": 50,
            "totalTokens": 150,
        });
        let r = redact_json_value(&v);
        assert_eq!(r["inputTokens"], 100);
        assert_eq!(r["outputTokens"], 50);
        assert_eq!(r["totalTokens"], 150);
    }

    #[test]
    fn redacts_sk_and_bearer() {
        let t = redact_text("Bearer abc.def.ghi-token and sk-abcdefghijklmnopqrst");
        assert!(t.contains("[REDACTED]"), "got: {t}");
        assert!(!t.contains("abc.def.ghi"));
        assert!(!t.contains("sk-abcdefghij"));
    }

    #[test]
    fn protected_path_body_redacted_even_when_granted() {
        let mut root = std::env::temp_dir();
        root.push(format!(
            "lc-redact-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(".env"), "SECRET=1").unwrap();

        let content = json!({"status": "ok", "path": ".env", "content": "SECRET=1"});
        let r = redact_tool_result_content(&content, Some(&root), Some(".env"));
        assert_eq!(r["content"], "[REDACTED protected path]");
    }
}
