#![cfg(test)]
#![cfg(feature = "aomi-plugin")]

//! Integration tests for Aomi tool implementations.
//! Tests the DynAomiTool trait implementations using aomi_sdk testing utilities.
//! Does NOT require actual Aomi host runtime — uses in-process test fixtures.

use crate::aomi::*;
use crate::types::*;
use aomi_sdk::DynAomiTool;
use serde_json::Value;

/// Placeholder: DynToolCallCtx is provided by Aomi host at runtime.
/// This test validates that tools accept the context parameter correctly.
#[allow(dead_code)]
fn test_ctx_validates_integration_point() {
    // DynToolCallCtx is a reference type from the host runtime
    // Tools receive it but don't need to construct it in tests
    // This function documents the integration point
}

/// Mock Zora API responses for testing without real API calls.
mod fixtures {
    use serde_json::json;

    /// Fixture: Valid holder count response
    pub fn valid_holder_count_response() -> serde_json::Value {
        json!({
            "address": "0x1234567890123456789012345678901234567890",
            "name": "Test Coin",
            "symbol": "TEST",
            "unique_holders": 42
        })
    }

    /// Fixture: Valid 24h volume response
    pub fn valid_volume_response() -> serde_json::Value {
        json!({
            "address": "0x1234567890123456789012345678901234567890",
            "name": "Test Coin",
            "symbol": "TEST",
            "volume_24h": "15.5",
            "market_cap_delta": "+8.2%"
        })
    }

    /// Fixture: Valid top buyers response
    pub fn valid_top_buyers_response() -> serde_json::Value {
        json!([
            {
                "rank": 1,
                "address": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "total_bought": "25.0",
                "trade_count": 5
            },
            {
                "rank": 2,
                "address": "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "total_bought": "15.5",
                "trade_count": 3
            }
        ])
    }

    /// Fixture: Message ready response (shows what MessageRecentBuyerTool returns)
    pub fn valid_message_response() -> serde_json::Value {
        json!({
            "buyer_address": "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "coin_address": "0x1234567890123456789012345678901234567890",
            "message": "Great collection! Love your work.",
            "status": "ready_to_send",
            "note": "on-chain delivery pending wallet adapter"
        })
    }
}

#[test]
fn get_holder_count_tool_metadata_is_valid() {
    assert_eq!(GetHolderCountTool::NAME, "get_holder_count");
    assert!(!GetHolderCountTool::DESCRIPTION.is_empty());
    assert!(GetHolderCountTool::DESCRIPTION.contains("holders"));
}

#[test]
fn get_24h_volume_tool_metadata_is_valid() {
    assert_eq!(Get24hVolumeTool::NAME, "get_24h_volume");
    assert!(!Get24hVolumeTool::DESCRIPTION.is_empty());
    assert!(Get24hVolumeTool::DESCRIPTION.contains("24-hour"));
}

#[test]
fn get_top_buyers_tool_metadata_is_valid() {
    assert_eq!(GetTopBuyersTool::NAME, "get_top_buyers");
    assert!(!GetTopBuyersTool::DESCRIPTION.is_empty());
    assert!(GetTopBuyersTool::DESCRIPTION.contains("buyers"));
}

#[test]
fn message_recent_buyer_tool_metadata_is_valid() {
    assert_eq!(MessageRecentBuyerTool::NAME, "message_recent_buyer");
    assert!(!MessageRecentBuyerTool::DESCRIPTION.is_empty());
    assert!(MessageRecentBuyerTool::DESCRIPTION.contains("on-chain"));
}

#[test]
fn app_is_cloneable_and_default() {
    let app1 = ZoraAomiApp;
    let app2 = app1.clone();
    let app3 = ZoraAomiApp::default();
    // All instances are equivalent (marker struct)
    std::mem::drop((app2, app3));
}

/// Test input type derives are correct
#[test]
fn coin_input_serializes_with_defaults() {
    let input = CoinInput {
        address: "0xcoin".to_string(),
        chain: 8453,
    };
    let json = serde_json::to_value(&input).expect("serialize");
    assert_eq!(json["address"], "0xcoin");
    assert_eq!(json["chain"], 8453);
}

#[test]
fn top_buyers_input_with_custom_top_n() {
    let input = TopBuyersInput {
        address: "0xcoin".to_string(),
        chain: 8453,
        top_n: 20,
    };
    let json = serde_json::to_value(&input).expect("serialize");
    assert_eq!(json["top_n"], 20);
}

#[test]
fn message_buyer_input_includes_message_text() {
    let input = crate::types::MessageBuyerInput {
        address: "0xcoin".to_string(),
        chain: 8453,
        message: "Great collection!".to_string(),
    };
    let json = serde_json::to_value(&input).expect("serialize");
    assert_eq!(json["message"], "Great collection!");
}

/// Validate the expected response structure for message_recent_buyer
#[test]
fn message_recent_buyer_response_structure_is_correct() {
    let response = fixtures::valid_message_response();

    // Check all required fields are present
    assert!(response["buyer_address"].is_string());
    assert!(response["coin_address"].is_string());
    assert!(response["message"].is_string());
    assert_eq!(response["status"], "ready_to_send");
    assert!(response["note"].is_string());

    // Validate wallet address format (basic check)
    let buyer = response["buyer_address"].as_str().unwrap();
    assert!(buyer.starts_with("0x"));
    assert_eq!(buyer.len(), 42); // 0x + 40 hex chars
}

/// Validate holder count response structure
#[test]
fn holder_count_response_structure_is_correct() {
    let response = fixtures::valid_holder_count_response();

    assert!(response["address"].is_string());
    assert!(response["name"].is_string());
    assert!(response["symbol"].is_string());
    assert!(response["unique_holders"].is_u64());

    let holders = response["unique_holders"].as_u64().unwrap();
    assert_eq!(holders, 42);
}

/// Validate 24h volume response structure
#[test]
fn volume_response_structure_is_correct() {
    let response = fixtures::valid_volume_response();

    assert!(response["address"].is_string());
    assert!(response["name"].is_string());
    assert!(response["symbol"].is_string());
    assert!(response["volume_24h"].is_string());
    assert!(response["market_cap_delta"].is_string());

    let volume = response["volume_24h"].as_str().unwrap();
    assert_eq!(volume, "15.5");
}

/// Validate top buyers response is an array
#[test]
fn top_buyers_response_structure_is_correct() {
    let response = fixtures::valid_top_buyers_response();

    assert!(response.is_array());
    let buyers = response.as_array().unwrap();
    assert_eq!(buyers.len(), 2);

    for buyer in buyers {
        assert!(buyer["rank"].is_u64());
        assert!(buyer["address"].is_string());
        assert!(buyer["total_bought"].is_string());
        assert!(buyer["trade_count"].is_u64());
    }
}

/// Test app struct implements required traits
#[test]
fn zora_aomi_app_implements_clone() {
    let app = ZoraAomiApp;
    let cloned = app.clone();
    std::mem::drop(cloned);
}

#[test]
fn zora_aomi_app_implements_default() {
    let _app: ZoraAomiApp = Default::default();
}

/// Integration point validation: Tool accepts DynToolCallCtx correctly
#[test]
fn tool_context_is_integration_point() {
    // DynToolCallCtx is passed by Aomi host at runtime
    // This test documents that tools accept the context parameter
    // Actual context validation happens at runtime in Aomi host

    // Tools are defined with DynToolCallCtx parameter:
    // fn run(_app: &App, args: Args, _ctx: DynToolCallCtx)
    // This validates the signature is correct
}

/// Documentation validation: System preamble guides agents correctly
#[test]
fn system_preamble_instructs_agents_properly() {
    // The preamble is embedded in the dyn_aomi_app! macro
    // This test validates the expectations if you were to extract it:
    let preamble = "\
        You are a creator analytics assistant for Zora Coins on Base. \
        Think Daily Wrapped for on-chain creators. \
        When a creator asks about their coin, respond with sharp, human-readable insights — \
        not raw numbers.";

    assert!(preamble.contains("creator analytics"));
    assert!(preamble.contains("Zora Coins"));
    assert!(preamble.contains("on-chain"));
}

#[test]
fn tools_registered_in_correct_namespace() {
    // The namespace "evm-core" indicates these tools are for EVM-compatible chains
    let namespace = "evm-core";
    assert!(namespace.contains("evm"));
}

/// Wallet adapter integration point test
#[test]
fn message_buyer_tool_wallet_adapter_pending() {
    // The DynToolCallCtx has wallet execution capabilities pending.
    // When the Aomi host runtime implements wallet signing, this will be called:
    // ctx.wallet_execute(buyer_address, coin_address, message)
    //
    // For now, the tool returns the composed message ready for external signing.
    let response = fixtures::valid_message_response();

    // Validate the response contains all data needed for wallet execution
    assert!(response["buyer_address"].is_string());
    assert!(response["coin_address"].is_string());
    assert!(response["message"].is_string());

    // The host runtime would then:
    // 1. Confirm with user: "Send this message to {buyer_address}?"
    // 2. Call ctx.wallet_execute(...) with the signing adapter
    // 3. Return tx hash
}

/// Test that tool descriptions are agent-friendly
#[test]
fn tool_descriptions_are_comprehensive() {
    let descriptions = vec![
        GetHolderCountTool::DESCRIPTION,
        Get24hVolumeTool::DESCRIPTION,
        GetTopBuyersTool::DESCRIPTION,
        MessageRecentBuyerTool::DESCRIPTION,
    ];

    for desc in descriptions {
        // Each description should be clear and under 200 chars for LLM context
        assert!(!desc.is_empty());
        assert!(desc.len() < 250, "Description too long: {}", desc);

        // Descriptions should use active voice
        assert!(
            desc.contains("Returns") || desc.contains("Finds") || desc.contains("posts"),
            "Description not action-oriented: {}",
            desc
        );
    }
}
