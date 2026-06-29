// Natural language intent parser and executor
// Simulates what Aomi + Claude would do locally

use crate::{
    load_client_from_env, tools,
    types::{CoinInput, TopBuyersInput},
};

#[derive(Debug, Clone)]
pub enum IntentType {
    HolderCount,
    Volume24h,
    TopBuyers,
    MessageBuyer,
    TokenSummary,
}

#[derive(Debug, Clone)]
pub struct ParsedIntent {
    pub intent: IntentType,
    pub coin_address: Option<String>,
    pub limit: Option<u32>,
    pub message: Option<String>,
}

/// Simple pattern matching to detect intent from natural language
/// In production, this would be Claude
pub fn parse_user_input(input: &str) -> Option<ParsedIntent> {
    let lower = input.to_lowercase();

    // Extract coin address (0x followed by 40 hex chars)
    let coin_address = extract_address(input);

    // Detect intent keywords
    if lower.contains("holder") || lower.contains("holder count") || lower.contains("unique holder")
    {
        return Some(ParsedIntent {
            intent: IntentType::HolderCount,
            coin_address,
            limit: None,
            message: None,
        });
    }

    if lower.contains("volume") && lower.contains("24h") {
        return Some(ParsedIntent {
            intent: IntentType::Volume24h,
            coin_address,
            limit: None,
            message: None,
        });
    }

    if lower.contains("top")
        && (lower.contains("buyer") || lower.contains("whale") || lower.contains("biggest"))
    {
        let limit = extract_number(&input).unwrap_or(5);
        return Some(ParsedIntent {
            intent: IntentType::TopBuyers,
            coin_address,
            limit: Some(limit),
            message: None,
        });
    }

    if lower.contains("message") && lower.contains("buyer") {
        // Extract message between quotes
        let message = extract_quoted_text(&input);
        return Some(ParsedIntent {
            intent: IntentType::MessageBuyer,
            coin_address,
            limit: None,
            message,
        });
    }

    if coin_address.is_some() {
        return Some(ParsedIntent {
            intent: IntentType::TokenSummary,
            coin_address,
            limit: None,
            message: None,
        });
    }

    None
}

fn extract_address(input: &str) -> Option<String> {
    // Standard Ethereum address: 0x + 40 hex chars (case-insensitive)
    let eth = regex::Regex::new(r"(?i)0x[a-f0-9]{40}")
        .unwrap()
        .find(input)
        .map(|m| m.as_str().to_string());

    if eth.is_some() {
        return eth;
    }

    // ENS name: e.g. vitalik.eth
    regex::Regex::new(r"\b[\w-]+\.eth\b")
        .unwrap()
        .find(input)
        .map(|m| m.as_str().to_string())
}

fn extract_number(input: &str) -> Option<u32> {
    let cleaned = regex::Regex::new(r"0x[a-fA-F0-9]+")
        .ok()?
        .replace_all(input, "")
        .to_string();

    regex::Regex::new(r"\b(\d+)\b")
        .ok()?
        .find(&cleaned)
        .and_then(|m| m.as_str().parse().ok())
}

fn extract_quoted_text(input: &str) -> Option<String> {
    let start = input.find('"')? + 1;
    let end = input[start..].find('"')? + start;
    Some(input[start..end].to_string())
}

/// Execute the parsed intent against actual tools
pub async fn execute_intent(intent: &ParsedIntent) -> Result<String, String> {
    let client = load_client_from_env().map_err(|e| format!("Failed to load client: {}", e))?;

    let address = intent
        .coin_address
        .as_ref()
        .ok_or("No coin address provided")?;

    match &intent.intent {
        IntentType::HolderCount => {
            let input = CoinInput {
                address: address.clone(),
                chain: 8453,
            };

            match tools::get_holder_count(input, &client).await {
                Ok(result) => {
                    let holders = result.unique_holders;
                    let name = &result.symbol;
                    Ok(format!("📊 {} has {} unique holders.", name, holders))
                }
                Err(e) => Err(format!("Failed to fetch holder count: {}", e)),
            }
        }

        IntentType::Volume24h => {
            let input = CoinInput {
                address: address.clone(),
                chain: 8453,
            };

            match tools::get_24h_volume(input, &client).await {
                Ok(result) => Ok(format!(
                    "📈 {}: 24h volume is {} ETH, market cap change {}",
                    result.symbol, result.volume_24h, result.market_cap_delta
                )),
                Err(e) => Err(format!("Failed to fetch volume: {}", e)),
            }
        }

        IntentType::TopBuyers => {
            let input = TopBuyersInput {
                address: address.clone(),
                chain: 8453,
                top_n: intent.limit.unwrap_or(5),
            };

            match tools::get_top_buyers(input, &client).await {
                Ok(buyers) => {
                    if buyers.is_empty() {
                        return Ok("No buyers found.".to_string());
                    }

                    let mut response = "🐋 Top Buyers:\n".to_string();
                    let limit = intent.limit.unwrap_or(5) as usize;
                    for buyer in buyers.iter().take(limit) {
                        response.push_str(&format!(
                            "#{}: {} bought {} ETH\n",
                            buyer.rank, buyer.address, buyer.total_bought
                        ));
                    }
                    Ok(response)
                }
                Err(e) => Err(format!("Failed to fetch top buyers: {}", e)),
            }
        }

        IntentType::MessageBuyer => {
            let message = intent.message.as_ref().ok_or("No message provided")?;

            Ok(format!(
                "✉️ Message prepared for recent buyer of {}: \"{}\"",
                address, message
            ))
        }

        IntentType::TokenSummary => {
            let input = CoinInput {
                address: address.clone(),
                chain: 8453,
            };

            let holders = tools::get_holder_count(input.clone(), &client).await;
            let volume = tools::get_24h_volume(input.clone(), &client).await;

            match (holders, volume) {
                (Ok(h), Ok(v)) => Ok(format!(
                    "📊 {}\n👥 Holders: {}\n📈 24h Volume: {} ETH\n📉 Market cap change: {}",
                    h.symbol, h.unique_holders, v.volume_24h, v.market_cap_delta
                )),
                _ => Err("Could not fetch token summary.".to_string()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_holder_count() {
        let input = "How many holders does 0xF5735B760e2194521377A24a5c1e830aFa83aCDB have?";
        let parsed = parse_user_input(input).unwrap();

        assert!(matches!(parsed.intent, IntentType::HolderCount));
        assert_eq!(
            parsed.coin_address,
            Some("0xF5735B760e2194521377A24a5c1e830aFa83aCDB".to_string())
        );
    }

    #[test]
    fn test_parse_volume() {
        let input = "What's the 24h volume for 0xF5735B760e2194521377A24a5c1e830aFa83aCDB?";
        let parsed = parse_user_input(input).unwrap();

        assert!(matches!(parsed.intent, IntentType::Volume24h));
    }

    #[test]
    fn test_parse_top_buyers() {
        let input = "Show top 5 buyers of 0xF5735B760e2194521377A24a5c1e830aFa83aCDB";
        let parsed = parse_user_input(input).unwrap();

        assert!(matches!(parsed.intent, IntentType::TopBuyers));
        assert_eq!(parsed.limit, Some(5));
    }

    #[test]
    fn test_parse_message() {
        let input =
            "Message the buyer of 0xF5735B760e2194521377A24a5c1e830aFa83aCDB with \"Great coin!\"";
        let parsed = parse_user_input(input).unwrap();

        assert!(matches!(parsed.intent, IntentType::MessageBuyer));
        assert_eq!(parsed.message, Some("Great coin!".to_string()));
    }
}
