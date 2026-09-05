//! Resolving the user's connected wallet from the host session.
//!
//! The connected wallet is authoritative and comes from the host, never from
//! the model. A tool argument can only ever *agree* with the session — it can
//! never select a different address.
//!
//! This matters most on the write path. `somm_aurora_deposit` used to take the
//! wallet as a required LLM-supplied argument: the execution would be created
//! for whatever address the model produced, while the deposit transfer is
//! signed by whatever wallet is actually connected. `/api/execution`'s guards
//! (cap, kill-switch, rate limit) are all keyed *per wallet*, so a mismatched
//! pair passes every one of them cleanly and still moves the user's funds
//! against someone else's execution. Nothing downstream would catch it.
//!
//! Extracted here (rather than left private in `tool.rs`) because all three
//! tool modules need the same rule.

use aomi_sdk::DynToolCallCtx;
use serde_json::Value;

/// Recursively find the first EVM address (0x + 40 hex) anywhere in the
/// host-injected session attributes — the user's connected wallet — regardless
/// of the exact key path the host nests it under. Prefers an explicit `address`.
pub(crate) fn is_evm_address(s: &str) -> bool {
    let t = s.trim();
    t.len() == 42 && t.starts_with("0x") && t[2..].chars().all(|c| c.is_ascii_hexdigit())
}

pub(crate) fn find_evm_address(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => {
            if is_evm_address(s) {
                Some(s.trim().to_string())
            } else {
                None
            }
        }
        Value::Array(a) => a.iter().find_map(find_evm_address),
        Value::Object(o) => o
            .get("address")
            .and_then(find_evm_address)
            .or_else(|| o.values().find_map(find_evm_address)),
        _ => None,
    }
}

/// The connected wallet from the CANONICAL path only —
/// `state_attributes.domain.evm.address`, the address the `evm-core` namespace
/// injects. Validated as an address rather than merely non-empty: a malformed
/// value there would otherwise be forwarded verbatim as the wallet.
///
/// This is the only resolution the write path may use. See
/// [`resolve_connected_wallet`] for why the scan is not acceptable there.
pub(crate) fn canonical_connected_wallet(ctx: &DynToolCallCtx) -> Option<String> {
    ctx.attribute_string(&["domain", "evm", "address"])
        .filter(|s| is_evm_address(s))
        .map(|s| s.trim().to_string())
}

/// The connected wallet, canonical path first, then a scan of every attribute
/// for any EVM address — tolerating hosts that nest it under a different key.
///
/// **Read paths only.** The scan returns the first EVM address found anywhere
/// in the host attributes, and there is no way to tell the user's wallet from a
/// token contract, a venue, or a recipient that happens to be sitting in an
/// attribute. On a read that is a harmless convenience; on a fund-moving path a
/// host that omitted `domain.evm.address` would fund against whatever unrelated
/// address it happened to carry. (Review note from henrio123 on #9.)
pub(crate) fn resolve_connected_wallet(ctx: &DynToolCallCtx) -> Option<String> {
    canonical_connected_wallet(ctx)
        .or_else(|| ctx.state_attributes.values().find_map(find_evm_address))
}

fn same_address(a: &str, b: &str) -> bool {
    // EVM addresses are case-insensitive; checksum casing is display-only, so
    // the same address in two casings must compare equal.
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// Resolve the wallet for a **fund-moving** tool.
///
/// The session is the only source. An argument is accepted only when it names
/// the same address; a mismatch is refused rather than silently preferred
/// either way, because at that point we cannot tell whether the model
/// misremembered an address or is being steered to one. No session wallet means
/// no deposit — there is nothing safe to fall back to.
pub(crate) fn require_session_wallet(
    ctx: &DynToolCallCtx,
    arg: Option<&str>,
) -> Result<String, String> {
    // Canonical path ONLY — never the scan. A host that omits
    // domain.evm.address while carrying some other address in an attribute must
    // fail closed here, not fund against that address.
    let Some(session) = canonical_connected_wallet(ctx) else {
        return Err(
            "wallet_error: no connected wallet on this session. Ask the user to \
                    connect their wallet in the app — do not ask them to paste an address."
                .to_string(),
        );
    };

    if let Some(a) = arg.map(str::trim).filter(|s| !s.is_empty())
        && !same_address(a, &session)
    {
        return Err(
            "wallet_mismatch: the requested wallet is not the connected wallet. Refusing \
             to act. Omit the wallet argument — it is resolved from the session."
                .to_string(),
        );
    }

    Ok(session)
}

/// Resolve the wallet for a **read-only** tool.
///
/// Same mismatch rule, but a missing session falls back to an explicit argument
/// so read tools keep working in contexts where the host has not injected an
/// address. Reads move no funds, so the fallback costs nothing.
pub(crate) fn read_wallet(ctx: &DynToolCallCtx, arg: Option<&str>) -> Result<String, String> {
    let arg = arg.map(str::trim).filter(|s| !s.is_empty());

    match (resolve_connected_wallet(ctx), arg) {
        (Some(session), Some(a)) if !same_address(a, &session) => Err(
            "wallet_mismatch: the requested wallet is not the connected wallet. Omit the \
             wallet argument — it is resolved from the session."
                .to_string(),
        ),
        (Some(session), _) => Ok(session),
        (None, Some(a)) => Ok(a.to_string()),
        (None, None) => Err(
            "wallet_error: no connected wallet on this session. Ask the user \
                             to connect their wallet in the app."
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aomi_sdk::testing::TestCtxBuilder;
    use serde_json::json;

    const CONNECTED: &str = "0xAbCdEf0123456789aBcDeF0123456789AbCdEf01";
    const OTHER: &str = "0x1111111111111111111111111111111111111111";

    fn ctx_with_wallet(addr: &str) -> DynToolCallCtx {
        TestCtxBuilder::new("t")
            .attribute("domain", json!({ "evm": { "address": addr } }))
            .build()
    }

    fn ctx_without_wallet() -> DynToolCallCtx {
        TestCtxBuilder::new("t").build()
    }

    // ── write path ──────────────────────────────────────────────────────────

    #[test]
    fn write_prefers_the_session_wallet_and_ignores_an_absent_arg() {
        assert_eq!(
            require_session_wallet(&ctx_with_wallet(CONNECTED), None).unwrap(),
            CONNECTED
        );
    }

    #[test]
    fn write_accepts_an_arg_that_agrees_regardless_of_casing() {
        let lower = CONNECTED.to_lowercase();
        assert_eq!(
            require_session_wallet(&ctx_with_wallet(CONNECTED), Some(&lower)).unwrap(),
            CONNECTED,
            "checksum casing is display-only; the same address must compare equal"
        );
    }

    #[test]
    fn write_refuses_a_wallet_that_is_not_the_connected_one() {
        // The case that motivated this: /api/execution's cap, kill-switch and
        // rate limit are all keyed per wallet, so a mismatched pair passes every
        // guard while the transfer is signed by the connected wallet.
        let err = require_session_wallet(&ctx_with_wallet(CONNECTED), Some(OTHER)).unwrap_err();
        assert!(err.starts_with("wallet_mismatch:"), "got: {err}");
    }

    #[test]
    fn write_refuses_when_there_is_no_connected_wallet() {
        // Must NOT fall back to the model-supplied address on a fund-moving tool.
        let err = require_session_wallet(&ctx_without_wallet(), Some(OTHER)).unwrap_err();
        assert!(err.starts_with("wallet_error:"), "got: {err}");
    }

    // ── read path ───────────────────────────────────────────────────────────

    #[test]
    fn read_prefers_session_and_refuses_a_mismatched_arg() {
        assert_eq!(
            read_wallet(&ctx_with_wallet(CONNECTED), None).unwrap(),
            CONNECTED
        );
        let err = read_wallet(&ctx_with_wallet(CONNECTED), Some(OTHER)).unwrap_err();
        assert!(err.starts_with("wallet_mismatch:"), "got: {err}");
    }

    #[test]
    fn read_falls_back_to_the_arg_only_when_the_session_has_none() {
        assert_eq!(
            read_wallet(&ctx_without_wallet(), Some(OTHER)).unwrap(),
            OTHER
        );
        assert!(read_wallet(&ctx_without_wallet(), None).is_err());
    }

    // ── address discovery ───────────────────────────────────────────────────

    #[test]
    fn write_refuses_to_resolve_via_the_scan_fallback() {
        // Review note (#9, henrio123): the scan finds the first EVM address
        // ANYWHERE in host attributes. On a read that is a harmless
        // convenience; on the write path a host that omits
        // domain.evm.address while carrying some other address — a token
        // contract, a recipient, a venue — would fund against THAT address.
        // The write path takes the canonical path only.
        let ctx = TestCtxBuilder::new("t")
            .attribute("token", json!({ "address": OTHER }))
            .build();
        let err = require_session_wallet(&ctx, None).unwrap_err();
        assert!(err.starts_with("wallet_error:"), "got: {err}");
    }

    #[test]
    fn read_still_accepts_the_scan_fallback() {
        // Reads move no funds, so the convenience is kept there.
        let ctx = TestCtxBuilder::new("t")
            .attribute("wallets", json!([{ "address": CONNECTED }]))
            .build();
        assert_eq!(read_wallet(&ctx, None).unwrap(), CONNECTED);
    }

    #[test]
    fn finds_a_nested_address_under_an_unexpected_key_path() {
        let ctx = TestCtxBuilder::new("t")
            .attribute("wallets", json!([{ "evm": { "address": CONNECTED } }]))
            .build();
        assert_eq!(resolve_connected_wallet(&ctx).as_deref(), Some(CONNECTED));
    }

    #[test]
    fn ignores_values_that_are_not_evm_addresses() {
        let ctx = TestCtxBuilder::new("t")
            .attribute("domain", json!({ "evm": { "address": "not-an-address" } }))
            .attribute("note", json!("0xdeadbeef"))
            .build();
        assert_eq!(resolve_connected_wallet(&ctx), None);
    }
}
