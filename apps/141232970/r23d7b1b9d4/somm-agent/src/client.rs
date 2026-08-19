//! HTTP client for the Agentic Somm API.
//!
//! Thin forwarder: every tool maps to one route on the deployed Somm app
//! (`SOMM_API_BASE_URL`, e.g. https://agentic.somm.finance), authenticated with
//! `Authorization: Bearer <SOMM_API_KEY>`. The key is resolved per call from
//! the host-injected secret vault (see `tool::api_key`), not from process env.
//! This mirrors the TypeScript MCP proxy (`mcp/src/{client,contract}.ts`) one-to-one so the deployed agent and the
//! local MCP server speak to the exact same endpoints.

use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::Value;

const DEFAULT_BASE_URL: &str = "https://agentic.somm.finance";

#[derive(Clone)]
pub struct SommApp {
    client: Client,
    base_url: String,
}

impl Default for SommApp {
    fn default() -> Self {
        let base_url = std::env::var("SOMM_API_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        Self::new(base_url)
    }
}

impl SommApp {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// GET `path` (path may include a query string). Returns the parsed JSON body.
    pub fn get(&self, path: &str, api_key: &str) -> Result<Value, String> {
        self.send(self.client.get(self.url(path)), api_key)
    }

    /// POST `path` with a JSON body. Returns the parsed JSON body.
    pub fn post(&self, path: &str, body: Value, api_key: &str) -> Result<Value, String> {
        self.send(self.client.post(self.url(path)).json(&body), api_key)
    }

    fn send(&self, req: reqwest::blocking::RequestBuilder, api_key: &str) -> Result<Value, String> {
        let resp = req
            .bearer_auth(api_key)
            .header("Accept", "application/json")
            .send()
            .map_err(|e| format!("[somm] request failed: {e}"))?;

        let status = resp.status();
        let text = resp
            .text()
            .map_err(|e| format!("[somm] read body failed: {e}"))?;

        if !status.is_success() {
            // Mirror mapHttpToToolError: short, actionable, no raw HTML/secret dumps.
            let snippet: String = text.chars().take(300).collect();
            return Err(format!("[somm] HTTP {}: {}", status.as_u16(), snippet));
        }

        if text.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| format!("[somm] parse failed: {e}"))
    }
}
