//! Small, bounded JSON-over-HTTPS transport used by community registries.
//!
//! The transport deliberately does not implement authentication or a server
//! protocol. Callers provide an optional bearer token and define the JSON
//! envelope they exchange. Keeping this layer small lets prompt and skill
//! registries share timeout, size, and URL validation behaviour.

use serde_json::Value;
use std::time::Duration;

const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Summary returned by a remote registry synchronization operation.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RemoteSyncReport {
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
    pub uploaded: usize,
}

fn validate_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return Err("remote URL must use http:// or https://".to_string());
    }
    if trimmed.len() > 2048 || trimmed.contains(['\r', '\n']) {
        return Err("remote URL is invalid or too long".to_string());
    }
    Ok(())
}

fn auth_header(token: Option<&str>) -> Option<String> {
    token
        .filter(|value| !value.is_empty())
        .map(|value| format!("Bearer {value}"))
}

/// Fetch a JSON document with bounded timeouts and a response size limit.
pub fn get_json(url: &str, token: Option<&str>) -> Result<Value, String> {
    validate_url(url)?;
    let mut request = ureq::get(url)
        .config()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .build();
    if let Some(value) = auth_header(token) {
        request = request.header("Authorization", &value);
    }
    let response = request.call().map_err(format_http_error)?;
    let mut body = response.into_body();
    let bytes = body
        .read_to_vec()
        .map_err(|error| format!("failed to read remote response: {error}"))?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(format!("remote response exceeds {} MiB", MAX_RESPONSE_BYTES / 1024 / 1024));
    }
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid remote JSON: {error}"))
}

/// Send a JSON document and decode the JSON response.
pub fn post_json(url: &str, token: Option<&str>, payload: &Value) -> Result<Value, String> {
    validate_url(url)?;
    let mut request = ureq::post(url)
        .config()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .build()
        .header("Content-Type", "application/json");
    if let Some(value) = auth_header(token) {
        request = request.header("Authorization", &value);
    }
    let response = request.send_json(payload).map_err(format_http_error)?;
    let mut body = response.into_body();
    let bytes = body
        .read_to_vec()
        .map_err(|error| format!("failed to read remote response: {error}"))?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(format!("remote response exceeds {} MiB", MAX_RESPONSE_BYTES / 1024 / 1024));
    }
    if bytes.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid remote JSON: {error}"))
}

fn format_http_error(error: ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(code) => format!("remote server returned HTTP {code}"),
        ureq::Error::Timeout(_) => "remote request timed out".to_string(),
        other => format!("remote request failed: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn server(body: &'static str, expected_method: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let size = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..size]);
            assert!(request.starts_with(expected_method));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{address}/registry")
    }

    #[test]
    fn validates_urls_and_auth_headers() {
        assert!(validate_url("https://example.test/registry").is_ok());
        assert!(validate_url("ftp://example.test/registry").is_err());
        assert!(validate_url("https://example.test/a\nb").is_err());
        assert_eq!(auth_header(None), None);
        assert_eq!(auth_header(Some("token")), Some("Bearer token".to_string()));
    }

    #[test]
    fn gets_json_with_a_bounded_http_request() {
        let url = server(r#"{"version":1}"#, "GET");
        let value = get_json(&url, None).unwrap();
        assert_eq!(value["version"], 1);
    }

    #[test]
    fn posts_json_and_decodes_the_response() {
        let url = server(r#"{"accepted":true}"#, "POST");
        let value = post_json(&url, Some("secret"), &serde_json::json!({"hello":"world"})).unwrap();
        assert_eq!(value["accepted"], true);
    }

    #[test]
    fn rejects_invalid_json() {
        let url = server("not-json", "GET");
        assert!(get_json(&url, None).unwrap_err().contains("invalid remote JSON"));
    }
}
