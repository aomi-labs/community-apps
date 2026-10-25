//! HTTP client for the Agentic Somm API.
//!
//! Thin forwarder: every tool maps to one route on the deployed Somm app,
//! authenticated with `Authorization: Bearer $SOMM_API_KEY`. This mirrors the
//! TypeScript MCP proxy (`mcp/src/{client,contract}.ts`) one-to-one so the
//! deployed agent and the local MCP server speak to the exact same endpoints.
//!
//! ## Where the credentials come from
//!
//! In the hosted runtime the secret source is the **tool-call context**, not
//! the process environment. Aomi resolves an app's secrets per call from its
//! vault; a hosted plugin reading `std::env::var` would find nothing, because
//! nothing put it there. That is why credentials are resolved inside each call
//! rather than once at construction — `DynToolCallCtx` exists only inside `run`.
//!
//! An environment fallback is retained for local development. It must never
//! become the hosted path: when the vault has no key that is a configuration
//! error to report, not a reason to send an unauthenticated request and let the
//! user meet a confusing 401 mid-flow.

use std::time::Duration;

use aomi_sdk::{DynToolCallCtx, Secret, resolve_secret_value};
use reqwest::blocking::Client;
use serde_json::Value;

const DEFAULT_BASE_URL: &str = "https://agentic.somm.finance";

/// Base URL of the Somm app serving the guarded API routes. Optional — the
/// deployed default is correct for every environment except local development.
pub const SOMM_API_BASE_URL_SECRET: Secret = Secret::new(
    "SOMM_API_BASE_URL",
    "Base URL of the Somm app serving the guarded API (e.g. https://agentic.somm.finance).",
    false,
);

/// Bearer credential authorizing this agent against Somm's guarded API.
///
/// `SOMM_API_KEY` is the app-facing *name* of this vault entry, not a shared
/// value: the credential issued to Aomi is distinct from the `SOMM_API_KEY`
/// Somm holds in CI and Vercel. That separation is what lets Aomi's credential
/// be revoked and rotated on its own, makes its use attributable in Somm's
/// audit log, and — on the Somm side — is what carries the paywall bypass, so
/// a call Aomi already meters is not charged twice.
pub const SOMM_API_KEY_SECRET: Secret = Secret::new(
    "SOMM_API_KEY",
    "Bearer key authorizing this agent against Somm's guarded API.",
    true,
);

/// Credentials for the guarded Somm API, resolved per tool call.
#[derive(Clone, Debug)]
pub struct SommCreds {
    base_url: String,
    api_key: String,
}

impl SommCreds {
    /// Resolve from the host vault, falling back to the process environment for
    /// local development.
    ///
    /// Fails closed on a missing key: every route this client calls is guarded,
    /// so proceeding without one yields a 401 the agent would have to explain
    /// to a user, instead of a configuration error an operator can act on.
    pub fn from_ctx(ctx: &DynToolCallCtx) -> Result<Self, String> {
        let base_url = vault_or_env(ctx, SOMM_API_BASE_URL_SECRET.name)
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let api_key = vault_or_env(ctx, SOMM_API_KEY_SECRET.name).ok_or_else(|| {
            "config_error: SOMM_API_KEY is not configured for this app".to_string()
        })?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }
}

/// Vault first, environment second, treating blank as absent in both.
fn vault_or_env(ctx: &DynToolCallCtx, name: &str) -> Option<String> {
    resolve_secret_value(ctx, None, name, "")
        .ok()
        .or_else(|| std::env::var(name).ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Stateless HTTP forwarder. Holds only the connection pool, never a
/// credential, so a long-lived instance cannot pin a key past its rotation.
#[derive(Clone)]
pub struct SommApp {
    client: Client,
}

/// Deliberately hand-written rather than derived: `Client::default()` carries
/// no request timeout, so a derive would silently drop the 30s bound and let a
/// hung upstream stall a tool call indefinitely.
impl Default for SommApp {
    fn default() -> Self {
        Self::new()
    }
}

impl SommApp {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { client }
    }

    /// GET `path` (path may include a query string). Returns the parsed JSON body.
    pub fn get(&self, ctx: &DynToolCallCtx, path: &str) -> Result<Value, String> {
        let creds = SommCreds::from_ctx(ctx)?;
        self.send(self.client.get(url(&creds, path)), &creds)
    }

    /// POST `path` with a JSON body. Returns the parsed JSON body.
    pub fn post(&self, ctx: &DynToolCallCtx, path: &str, body: Value) -> Result<Value, String> {
        let creds = SommCreds::from_ctx(ctx)?;
        self.send(self.client.post(url(&creds, path)).json(&body), &creds)
    }

    fn send(
        &self,
        req: reqwest::blocking::RequestBuilder,
        creds: &SommCreds,
    ) -> Result<Value, String> {
        let resp = req
            .bearer_auth(&creds.api_key)
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

fn url(creds: &SommCreds, path: &str) -> String {
    format!("{}{}", creds.base_url, path)
}
