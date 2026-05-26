use std::time::Duration;

use hmac::{Hmac, Mac};
use reqwest::blocking::Client;
use serde::Serialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

#[cfg(test)]
pub(crate) static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Clone, Debug)]
pub struct AuthenticatedHostIdentity {
    pub session_id: String,
    pub thread_id: Option<String>,
    pub agent_registry: Option<String>,
    pub agent_id: Option<String>,
}

#[derive(Clone)]
pub struct AgentexApp {
    client: Client,
    base_url: String,
}

impl Default for AgentexApp {
    fn default() -> Self {
        let base_url = std::env::var("AGENTEX_SERVICE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8787".to_string());
        Self::new(base_url)
    }
}

impl AgentexApp {
    pub fn try_new(base_url: impl Into<String>) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| format!("Failed to build Agentex HTTP client: {error}"))?;
        Ok(Self {
            client,
            base_url: base_url.into(),
        })
    }

    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url: base_url.into(),
        }
    }

    pub fn tool_url(&self, tool_name: &str) -> String {
        format!("{}/tool/{}", self.base_url.trim_end_matches('/'), tool_name)
    }

    pub fn manifest_url(&self) -> String {
        format!("{}/api/aomi/manifest", self.base_url.trim_end_matches('/'))
    }

    pub fn post_tool<T: Serialize>(&self, tool_name: &str, args: &T) -> Result<Value, String> {
        self.post_tool_with_identity(tool_name, args, None)
    }

    pub fn post_tool_as_host<T: Serialize>(
        &self,
        tool_name: &str,
        args: &T,
        host_identity: &AuthenticatedHostIdentity,
    ) -> Result<Value, String> {
        self.post_tool_with_identity(tool_name, args, Some(host_identity))
    }

    fn post_tool_with_identity<T: Serialize>(
        &self,
        tool_name: &str,
        args: &T,
        host_identity: Option<&AuthenticatedHostIdentity>,
    ) -> Result<Value, String> {
        let body = serde_json::to_string(args)
            .map_err(|error| format!("Failed to serialize Agentex tool args: {error}"))?;
        let response = self
            .client
            .post(self.tool_url(tool_name))
            .headers(build_authenticated_header_map(
                tool_name,
                &body,
                host_identity,
            )?)
            .body(body)
            .send()
            .map_err(|error| format!("Agentex service request failed: {error}"))?;
        let status = response.status();
        let body = response
            .text()
            .map_err(|error| format!("Agentex service response read failed: {error}"))?;
        if !status.is_success() {
            return Err(normalize_error(status.as_u16(), &body));
        }
        let value = parse_json_response("Agentex service", &body)?;
        Ok(Self::normalize_result(value))
    }

    pub fn get_manifest(&self) -> Result<Value, String> {
        let response = self
            .client
            .get(self.manifest_url())
            .send()
            .map_err(|error| format!("Agentex manifest request failed: {error}"))?;
        let status = response.status();
        let body = response
            .text()
            .map_err(|error| format!("Agentex manifest response read failed: {error}"))?;
        if !status.is_success() {
            return Err(normalize_error(status.as_u16(), &body));
        }
        let value = parse_json_response("Agentex manifest", &body)?;
        Ok(Self::normalize_result(value))
    }

    pub fn normalize_result(value: Value) -> Value {
        normalize_value(value, true)
    }
}

fn build_authenticated_header_map(
    tool_name: &str,
    raw_body: &str,
    host_identity: Option<&AuthenticatedHostIdentity>,
) -> Result<reqwest::header::HeaderMap, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    for (name, value) in build_authenticated_headers(tool_name, raw_body, host_identity)? {
        let header_name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|error| format!("Invalid Agentex header name {name}: {error}"))?;
        let header_value = reqwest::header::HeaderValue::from_str(&value)
            .map_err(|error| format!("Invalid Agentex header value for {name}: {error}"))?;
        headers.insert(header_name, header_value);
    }
    Ok(headers)
}

fn parse_json_response(context: &str, body: &str) -> Result<Value, String> {
    serde_json::from_str::<Value>(body)
        .map_err(|error| format!("{context} returned invalid JSON: {error}"))
}

fn normalize_error(status: u16, body: &str) -> String {
    let parsed = serde_json::from_str::<Value>(body).ok();
    let message = parsed
        .as_ref()
        .and_then(|value| {
            value
                .get("error")
                .and_then(Value::as_str)
                .or_else(|| value.get("message").and_then(Value::as_str))
        })
        .or_else(|| {
            let trimmed = body.trim();
            (!trimmed.is_empty()).then_some(trimmed)
        })
        .unwrap_or("unknown Agentex service error");
    format!("Agentex service error {status}: {message}")
}

pub fn host_handoff(action: &str, args: Value) -> Value {
    json!({
        "status": "host_action_required",
        "SYSTEM_NEXT_ACTION": {
            "type": action,
            "preserve_args": normalize_value(args, false)
        }
    })
}

fn normalize_value(value: Value, top_level: bool) -> Value {
    match value {
        Value::Object(object) => {
            let mut normalized = Map::new();
            for (key, entry) in object {
                if is_sensitive_key(&key) {
                    if top_level {
                        continue;
                    }
                    normalized.insert(key, Value::String("[REDACTED]".to_string()));
                } else {
                    normalized.insert(key, normalize_value(entry, false));
                }
            }
            Value::Object(normalized)
        }
        Value::Array(entries) => Value::Array(
            entries
                .into_iter()
                .map(|entry| normalize_value(entry, false))
                .collect(),
        ),
        other => other,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "key" | "keyenvelope" | "privatekey" | "decoderkey" | "apikey" | "token" | "secret"
    )
}

fn build_authenticated_headers(
    tool_name: &str,
    raw_body: &str,
    host_identity: Option<&AuthenticatedHostIdentity>,
) -> Result<Vec<(String, String)>, String> {
    let Some(host_identity) = host_identity else {
        return Ok(Vec::new());
    };
    let secret = std::env::var("AGENTEX_HOST_IDENTITY_SECRET")
        .map_err(|_| "AGENTEX_HOST_IDENTITY_SECRET environment variable is not set".to_string())?;
    let signature = build_host_identity_signature(tool_name, raw_body, host_identity, &secret)?;
    let mut headers = vec![
        (
            "x-agentex-session-id".to_string(),
            host_identity.session_id.clone(),
        ),
        ("x-agentex-identity-signature".to_string(), signature),
    ];
    if let Some(thread_id) = &host_identity.thread_id {
        headers.push(("x-agentex-thread-id".to_string(), thread_id.clone()));
    }
    match (&host_identity.agent_registry, &host_identity.agent_id) {
        (Some(agent_registry), Some(agent_id)) => {
            headers.push((
                "x-agentex-agent-registry".to_string(),
                agent_registry.clone(),
            ));
            headers.push(("x-agentex-agent-id".to_string(), agent_id.clone()));
        }
        (None, None) => {}
        _ => {
            return Err(
                "authenticated host identity requires both agent registry and agent id".to_string(),
            )
        }
    }
    Ok(headers)
}

fn build_host_identity_signature(
    tool_name: &str,
    raw_body: &str,
    host_identity: &AuthenticatedHostIdentity,
    secret: &str,
) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|error| format!("Failed to initialize Agentex identity signer: {error}"))?;
    mac.update(signature_payload(tool_name, raw_body, host_identity).as_bytes());
    Ok(bytes_to_hex(&mac.finalize().into_bytes()))
}

fn signature_payload(
    tool_name: &str,
    raw_body: &str,
    host_identity: &AuthenticatedHostIdentity,
) -> String {
    [
        tool_name,
        host_identity.session_id.as_str(),
        host_identity.thread_id.as_deref().unwrap_or(""),
        host_identity.agent_registry.as_deref().unwrap_or(""),
        host_identity.agent_id.as_deref().unwrap_or(""),
        body_sha256(raw_body).as_str(),
    ]
    .join("\n")
}

fn body_sha256(raw_body: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_body.as_bytes());
    bytes_to_hex(&hasher.finalize())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    #[test]
    fn builds_service_urls() {
        let app = AgentexApp::new("http://localhost:8787/");
        assert_eq!(
            app.tool_url("get_agent_state"),
            "http://localhost:8787/tool/get_agent_state"
        );
        assert_eq!(
            app.manifest_url(),
            "http://localhost:8787/api/aomi/manifest"
        );
    }

    #[test]
    fn normalization_removes_secret_echoes() {
        let normalized = AgentexApp::normalize_result(json!({
            "status": "ready",
            "key": "secret",
            "privateKey": "secret"
        }));
        assert_eq!(normalized, json!({ "status": "ready" }));
    }

    #[test]
    fn host_handoff_redacts_secret_fields() {
        let handoff = host_handoff(
            "publish_experience_sale",
            json!({
                "confirm": false,
                "key": "secret",
                "keyEnvelope": "secret-envelope",
                "privateKey": "secret-private-key",
                "filecoinPayReference": "payment-ref",
                "pair": "ETH/USDC"
            }),
        );

        assert_eq!(handoff["status"], "host_action_required");
        assert_eq!(
            handoff["SYSTEM_NEXT_ACTION"]["type"],
            "publish_experience_sale"
        );
        assert_eq!(
            handoff["SYSTEM_NEXT_ACTION"]["preserve_args"]["key"],
            "[REDACTED]"
        );
        assert_eq!(
            handoff["SYSTEM_NEXT_ACTION"]["preserve_args"]["keyEnvelope"],
            "[REDACTED]"
        );
        assert_eq!(
            handoff["SYSTEM_NEXT_ACTION"]["preserve_args"]["privateKey"],
            "[REDACTED]"
        );
        assert_eq!(
            handoff["SYSTEM_NEXT_ACTION"]["preserve_args"]["filecoinPayReference"],
            "payment-ref"
        );
        assert_eq!(
            handoff["SYSTEM_NEXT_ACTION"]["preserve_args"]["pair"],
            "ETH/USDC"
        );
    }

    #[test]
    fn authenticated_host_identity_headers_include_signature() {
        let _env_guard = TEST_ENV_LOCK
            .lock()
            .expect("env lock should not be poisoned");
        let previous_secret = std::env::var("AGENTEX_HOST_IDENTITY_SECRET").ok();
        std::env::set_var("AGENTEX_HOST_IDENTITY_SECRET", "host-secret");

        let headers = build_authenticated_headers(
            "prepare_experience_sale",
            r#"{"priceAmount":"5"}"#,
            Some(&AuthenticatedHostIdentity {
                session_id: "session-alpha".to_string(),
                thread_id: Some("thread-alpha".to_string()),
                agent_registry: Some("eip155:8453:0xregistry".to_string()),
                agent_id: Some("1".to_string()),
            }),
        )
        .expect("authenticated headers should build");
        let headers: std::collections::HashMap<_, _> = headers.into_iter().collect();

        assert_eq!(
            headers.get("x-agentex-session-id"),
            Some(&"session-alpha".to_string())
        );
        assert_eq!(
            headers.get("x-agentex-thread-id"),
            Some(&"thread-alpha".to_string())
        );
        assert_eq!(
            headers.get("x-agentex-agent-registry"),
            Some(&"eip155:8453:0xregistry".to_string())
        );
        assert_eq!(headers.get("x-agentex-agent-id"), Some(&"1".to_string()));
        assert!(headers.contains_key("x-agentex-identity-signature"));

        restore_secret(previous_secret);
    }

    #[test]
    fn post_tool_reports_plaintext_error_body() {
        let (base_url, request) = capture_one_response(
            b"HTTP/1.1 502 Bad Gateway\r\ncontent-type: text/plain\r\ncontent-length: 14\r\n\r\nupstream down\n",
        );
        let app = AgentexApp::new(base_url);

        let error = app
            .post_tool("get_agent_state", &json!({}))
            .expect_err("non-JSON error response should fail");

        let request = request.join().expect("request capture should complete");
        assert!(request.contains("POST /tool/get_agent_state "));
        assert!(error.contains("Agentex service error 502"));
        assert!(error.contains("upstream down"));
    }

    fn capture_one_response(response: &'static [u8]) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        listener
            .set_nonblocking(true)
            .expect("test server should support nonblocking accept");
        let addr = listener
            .local_addr()
            .expect("test server should expose address");
        let handle = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(2);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error)
                        if error.kind() == ErrorKind::WouldBlock && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("test server accept failed: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("test stream should support read timeout");
            let mut buffer = [0_u8; 8192];
            let bytes = stream
                .read(&mut buffer)
                .expect("test server should read request");
            stream
                .write_all(response)
                .expect("test server should write response");
            String::from_utf8_lossy(&buffer[..bytes]).to_string()
        });
        (format!("http://{addr}"), handle)
    }

    fn restore_secret(previous_secret: Option<String>) {
        if let Some(secret) = previous_secret {
            std::env::set_var("AGENTEX_HOST_IDENTITY_SECRET", secret);
        } else {
            std::env::remove_var("AGENTEX_HOST_IDENTITY_SECRET");
        }
    }
}
