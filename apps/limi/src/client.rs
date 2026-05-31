use serde_json::Value;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub(crate) const BASE_URL: &str = "https://api.limitless.exchange";

pub(crate) fn rt() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Runtime::new().map_err(|e| format!("[limi] tokio runtime: {e}"))
}

/// Percent-encode a URL path/query segment per RFC 3986.
pub(crate) fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            use std::fmt::Write;
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

// ── Rate limiter ───────────────────────────────────────────────────────────
// Max one outstanding request and minimum 300 ms between calls.
// The Aomi FFI dispatches one tool at a time (synchronous run() calls),
// so a single Mutex<Instant> gives us safe sequential spacing.

static LAST_CALL: Mutex<Option<Instant>> = Mutex::new(None);
const MIN_INTERVAL: Duration = Duration::from_millis(300);

async fn rate_gate() -> Result<(), String> {
    let sleep_for = {
        let mut guard = LAST_CALL
            .lock()
            .map_err(|e| format!("[limi] rate-limiter lock poisoned: {e}"))?;
        let now = Instant::now();
        let sleep_for = guard
            .map(|last| {
                let elapsed = now.duration_since(last);
                if elapsed < MIN_INTERVAL {
                    MIN_INTERVAL - elapsed
                } else {
                    Duration::ZERO
                }
            })
            .unwrap_or(Duration::ZERO);
        // Mark the intended call time (now + sleep) as the last call slot.
        *guard = Some(now + sleep_for);
        sleep_for
    };
    if sleep_for > Duration::ZERO {
        tokio::time::sleep(sleep_for).await;
    }
    Ok(())
}

/// Unauthenticated GET with rate limiting and one retry on 429.
pub(crate) async fn get(path: &str) -> Result<Value, String> {
    rate_gate().await?;
    match get_once(path).await {
        Err(e) if e.contains("429") => {
            // Back off and retry once.
            tokio::time::sleep(Duration::from_millis(600)).await;
            rate_gate().await?;
            get_once(path).await
        }
        other => other,
    }
}

async fn get_once(path: &str) -> Result<Value, String> {
    let url = format!("{BASE_URL}{path}");
    let resp = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("[limi] HTTP {path}: {e}"))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("[limi] body read {path}: {e}"))?;
    if !status.is_success() {
        return Err(format!("[limi] {path} → {status}: {body}"));
    }
    serde_json::from_str(&body).map_err(|e| {
        format!(
            "[limi] non-JSON from {path} ({e}): {}",
            body.chars().take(200).collect::<String>()
        )
    })
}
