// Agent demo: simulates what user sees when they input a wallet address
// Uses Claude API to generate natural language responses
// Run: ZORA_API_KEY=... ANTHROPIC_API_KEY=... cargo run --bin agent_demo -- 0xF5735B760e2194521377A24a5c1e830aFa83aCDB

use std::env;
use zora_aomi_tools::{
    load_client_from_env, tools,
    types::{CoinInput, TopBuyersInput},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: agent_demo <coin_address>");
        eprintln!("Example: agent_demo 0xF5735B760e2194521377A24a5c1e830aFa83aCDB");
        std::process::exit(1);
    }

    let coin_address = &args[1];
    let client = load_client_from_env()?;

    println!("\n💬 Aomi Agent\n");
    println!("{}", "─".repeat(60));

    let coin_input = CoinInput {
        address: coin_address.to_string(),
        chain: 8453, // base
    };

    // Collect data from all tools
    let mut analysis = String::new();

    // Get holder count
    print!("📊 Fetching holder count... ");
    std::io::Write::flush(&mut std::io::stdout())?;
    match tools::get_holder_count(coin_input.clone(), &client).await {
        Ok(result) => {
            println!("✓");
            analysis.push_str(&format!(
                "Coin: {} ({})\n- Unique Holders: {}\n",
                result.name, result.symbol, result.unique_holders
            ));
        }
        Err(e) => println!("✗ Error: {}", e),
    }

    // Get 24h volume
    print!("📈 Fetching 24h volume... ");
    std::io::Write::flush(&mut std::io::stdout())?;
    match tools::get_24h_volume(coin_input.clone(), &client).await {
        Ok(result) => {
            println!("✓");
            analysis.push_str(&format!(
                "- 24h Volume: {}\n- Market Cap Delta: {}\n",
                result.volume_24h, result.market_cap_delta
            ));
        }
        Err(e) => println!("✗ Error: {}", e),
    }

    // Get top buyers
    print!("🐋 Fetching top buyers... ");
    std::io::Write::flush(&mut std::io::stdout())?;
    let buyers_input = TopBuyersInput {
        address: coin_address.to_string(),
        chain: 8453, // base
        top_n: 5,
    };
    match tools::get_top_buyers(buyers_input, &client).await {
        Ok(buyers) => {
            println!("✓");
            analysis.push_str("- Top Buyers:\n");
            for buyer in buyers.iter().take(3) {
                analysis.push_str(&format!(
                    "  #{}: {} bought {}\n",
                    buyer.rank, buyer.address, buyer.total_bought
                ));
            }
        }
        Err(e) => println!("✗ Error: {}", e),
    }

    println!("\n{}", "─".repeat(60));
    println!("\n🤖 Agent Response:\n");

    // Generate natural language response using Claude
    generate_agent_response(&analysis).await?;

    println!("\n{}", "─".repeat(60));

    Ok(())
}

async fn generate_agent_response(data: &str) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("ANTHROPIC_API_KEY")?;

    let prompt = format!(
        r#"You are a creator analytics assistant for Zora Coins on Base. 
Think Daily Wrapped for on-chain creators.
When a creator asks about their coin, respond with sharp, human-readable insights — not raw numbers.

Examples of good responses:
- "12 new holders today, your top buyer is 0xabc…ef."
- "Volume is up 40% in the last 24h — your biggest trade was 2.1 ETH."

Convert this data into a natural, concise 1-2 sentence insight:

{}

Keep it conversational and human. Max 2 sentences."#,
        data
    );

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 150,
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        }))
        .send()
        .await?;

    if response.status().is_success() {
        let body: serde_json::Value = response.json().await?;
        if let Some(content) = body["content"][0]["text"].as_str() {
            println!("{}", content);
        }
    } else {
        println!("Could not generate response (Claude API error)");
        println!("{}", data);
    }

    Ok(())
}
