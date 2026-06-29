// Simple test runner for real API calls with output
// Run: ZORA_API_KEY=your_key cargo run --bin run_real_api_tests

use zora_aomi_tools::{
    aomi::*,
    tools,
    types::{CoinInput, TopBuyersInput, MessageBuyerInput},
};
use zora_coins_sdk::ZoraCoinsClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("ZORA_API_KEY").expect("Set ZORA_API_KEY env var");
    let client = ZoraCoinsClient::new(api_key);

    println!("🚀 Testing against real Zora API...\n");

    // Test 1: Holder Count
    println!("━━━ TEST 1: Get Holder Count ━━━");
    let holder_input = CoinInput {
        address: "0xF5735B760e2194521377A24a5c1e830aFa83aCDB".to_string(),
        chain: "base".to_string(),
    };
    match tools::get_holder_count(holder_input, &client).await {
        Ok(result) => println!("✓ Response:\n{:#?}\n", result),
        Err(e) => println!("✗ Error: {}\n", e),
    }

    // Test 2: 24h Volume
    println!("━━━ TEST 2: Get 24h Volume ━━━");
    let volume_input = CoinInput {
        address: "0xF5735B760e2194521377A24a5c1e830aFa83aCDB".to_string(),
        chain: "base".to_string(),
    };
    match tools::get_24h_volume(volume_input, &client).await {
        Ok(result) => println!("✓ Response:\n{:#?}\n", result),
        Err(e) => println!("✗ Error: {}\n", e),
    }

    // Test 3: Top Buyers
    println!("━━━ TEST 3: Get Top Buyers ━━━");
    let buyers_input = TopBuyersInput {
        address: "0xF5735B760e2194521377A24a5c1e830aFa83aCDB".to_string(),
        chain: "base".to_string(),
        limit: 5,
    };
    match tools::get_top_buyers(buyers_input, &client).await {
        Ok(result) => println!("✓ Response:\n{:#?}\n", result),
        Err(e) => println!("✗ Error: {}\n", e),
    }

    // Test 4: Message Recent Buyer
    println!("━━━ TEST 4: Message Recent Buyer ━━━");
    let msg_input = MessageBuyerInput {
        address: "0xF5735B760e2194521377A24a5c1e830aFa83aCDB".to_string(),
        chain: "base".to_string(),
        message: "Your coin is mooning! 🚀".to_string(),
    };
    match tools::get_recent_buyer_for_message(msg_input, &client).await {
        Ok(result) => println!("✓ Response:\n{:#?}\n", result),
        Err(e) => println!("✗ Error: {}\n", e),
    }

    println!("✅ All tests completed!");
    Ok(())
}
