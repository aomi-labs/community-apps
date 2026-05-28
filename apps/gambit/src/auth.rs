// Inlined HMAC-SHA256 + ISO timestamp helpers, copied verbatim from
// aomi-sdk/ext/src/hmac_auth.rs (the `hmac-auth` feature of aomi-ext).
// Aomi-ext is not yet published to crates.io and the upstream main branch
// has untracked submodule pointers that break `cargo` git-deps, so we keep
// these primitives local to gambit until aomi-ext ships as a crate.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

fn hmac_sha256_base64(key: &[u8], msg: &[u8]) -> String {
    BASE64.encode(hmac_sha256(key, msg))
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    BASE64
        .decode(s.as_bytes())
        .map_err(|e| format!("base64 decode error: {e}"))
}

fn iso_timestamp_ms() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    let (year, month, day) = days_to_ymd(secs / 86400);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.{millis:03}Z")
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y_final = if m <= 2 { y + 1 } else { y };
    (y_final, m, d)
}

/// Compute the `lmts-signature` value for one Limitless API request.
///
/// `secret_b64` is the raw string from the dashboard (already base64).
/// `path_with_query` MUST include leading `/` and any `?query=...` suffix.
/// `body` is `""` for GET / DELETE without bodies.
pub fn sign(
    secret_b64: &str,
    timestamp: &str,
    method: &str,
    path_with_query: &str,
    body: &str,
) -> Result<String, String> {
    let key = base64_decode(secret_b64)
        .map_err(|e| format!("[gambit] base64 decode secret: {e}"))?;
    let prehash = format!("{timestamp}\n{method}\n{path_with_query}\n{body}");
    Ok(hmac_sha256_base64(&key, prehash.as_bytes()))
}

/// ISO-8601 UTC timestamp with millisecond precision.
pub fn iso_timestamp() -> String {
    iso_timestamp_ms()
}
