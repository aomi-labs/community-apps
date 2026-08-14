//! Guards on `somm-agent.pricing.toml`.
//!
//! Aomi matches `[resources.X]` against the tool's `const NAME`. A key that
//! matches nothing does not error at load or at call time — it just silently
//! never charges, which is indistinguishable from the tool being free. That is
//! the failure this file exists to catch, because nothing else will.
//!
//! Deliberately parses with string operations rather than a TOML crate: the
//! shapes checked here are fixed and trivial, and a dev-dependency to read six
//! table headers is not worth the supply-chain surface.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sidecar() -> String {
    fs::read_to_string(repo_root().join("somm-agent.pricing.toml"))
        .expect("somm-agent.pricing.toml must exist beside the app manifest")
}

/// `[resources.NAME]` table headers, in file order.
fn priced_resources(toml: &str) -> Vec<String> {
    toml.lines()
        .map(str::trim)
        .filter_map(|line| {
            let inner = line.strip_prefix("[resources.")?.strip_suffix(']')?;
            Some(inner.to_string())
        })
        .collect()
}

/// Every `const NAME: &'static str = "...";` across the tool sources.
fn exported_tool_names() -> Vec<String> {
    let src = repo_root().join("src");
    let mut names = Vec::new();
    for entry in fs::read_dir(&src).expect("src/ must be readable") {
        let path = entry.expect("readable dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("readable source file");
        for line in text.lines() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("const NAME: &'static str = ") else {
                continue;
            };
            let rest = rest.trim_end_matches(';').trim();
            if let Some(name) = rest.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
                names.push(name.to_string());
            }
        }
    }
    names
}

#[test]
fn every_priced_resource_is_a_real_tool() {
    let priced = priced_resources(&sidecar());
    let exported = exported_tool_names();

    assert!(!priced.is_empty(), "sidecar prices nothing");
    assert!(
        !exported.is_empty(),
        "found no tool names in src/ — parser broken?"
    );

    for name in &priced {
        assert!(
            exported.contains(name),
            "priced resource `{name}` matches no tool `const NAME`. Aomi would \
             silently never charge for it. Exported tools: {exported:?}",
        );
    }
}

#[test]
fn continuation_tools_are_not_priced() {
    // One user deposit walks deposit -> submit_signature -> report_deposit.
    // Pricing the continuations bills a single user action three times.
    let priced = priced_resources(&sidecar());
    for name in [
        "somm_aurora_deposit",
        "somm_aurora_submit_signature",
        "somm_aurora_report_deposit",
    ] {
        assert!(
            !priced.contains(&name.to_string()),
            "`{name}` must stay free — see the sidecar's unpriced section",
        );
    }
}

#[test]
fn checking_a_balance_is_free() {
    let priced = priced_resources(&sidecar());
    assert!(
        !priced.contains(&"get_credit_balance".to_string()),
        "charging a user to check their balance is hostile",
    );
}

#[test]
fn prices_use_only_the_supported_v1_field() {
    // V1 accepts `pricing = { flat = <credits> }` and nothing else. Fields like
    // flat_usd / charge_on / cost_plus_bps are rejected by Aomi's parser, and a
    // rejected sidecar means the app ships unpriced.
    for line in sidecar().lines() {
        let line = line.trim();
        if !line.starts_with("pricing") {
            continue;
        }
        assert!(line.contains("flat ="), "unsupported pricing shape: {line}",);
        for banned in ["flat_usd", "charge_on", "cost_plus_bps", "tiers"] {
            assert!(
                !line.contains(banned),
                "`{banned}` is not accepted by V1: {line}"
            );
        }
    }
}

#[test]
fn beneficiary_is_a_valid_checksummed_address_on_the_configured_chain() {
    let toml = sidecar();

    let value = toml
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("value = "))
        .expect("beneficiary must declare a value")
        .trim_matches('"')
        .to_string();

    assert!(
        value.starts_with("0x"),
        "beneficiary must be 0x-prefixed: {value}"
    );
    assert_eq!(value.len(), 42, "beneficiary must be 20 bytes: {value}");
    assert!(
        value[2..].chars().all(|c| c.is_ascii_hexdigit()),
        "beneficiary has non-hex characters: {value}",
    );
    assert!(
        value[2..].chars().any(|c| c.is_ascii_uppercase()),
        "beneficiary looks un-checksummed (all lowercase); paste the EIP-55 form",
    );
    assert_ne!(
        value.to_lowercase(),
        "0x0000000000000000000000000000000000000000",
        "beneficiary must not be the zero address",
    );
    assert!(
        !value.to_uppercase().contains("TODO") && !value.to_uppercase().contains("REPLACE"),
        "placeholder beneficiary must never ship: {value}",
    );

    assert!(
        toml.contains(r#"chain = "eip155:84532""#),
        "staging beneficiary must be pinned to Base Sepolia until the production \
         x402 settlement network is confirmed — never default to mainnet",
    );
}

#[test]
fn every_beneficiary_reference_resolves() {
    let toml = sidecar();
    let declared: Vec<String> = toml
        .lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("name = "))
        .map(|v| v.trim_matches('"').to_string())
        .collect();

    let referenced: Vec<String> = toml
        .lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("beneficiary = "))
        .map(|v| v.trim_matches('"').to_string())
        .collect();

    assert!(!referenced.is_empty(), "no resource names a beneficiary");
    for name in referenced {
        assert!(
            declared.contains(&name),
            "resource references undeclared beneficiary `{name}`; declared: {declared:?}",
        );
    }
}
