use crate::market::limitless::fetch_live_markets;

use crate::market::analyzer::{analyze_market, analyze_odds, detect_high_edge};

use super::reasoning::reason;
use super::tools::AgentTool;

pub async fn execute(message: String) -> String {
    let tool = reason(&message);

    match tool {
        AgentTool::ScanMarkets => {
            match fetch_live_markets().await {
                Ok(markets) => {
                    if let Some(first) = markets.first() {
                        let thesis = crate::ai::thesis::build_thesis(first);

                        // You can log or attach thesis later
                        format!(
                            "{}\n\n---\nAI Thesis Generated:\n{}",
                            analyze_market(&markets),
                            thesis
                        )
                    } else {
                        "No markets found.".to_string()
                    }
                }

                Err(err) => format!("Market error: {}", err),
            }
        }

        AgentTool::AnalyzeOdds => match fetch_live_markets().await {
            Ok(markets) => analyze_odds(&markets),
            Err(err) => format!("Analysis error: {}", err),
        },

        AgentTool::HighConviction => match fetch_live_markets().await {
            Ok(markets) => detect_high_edge(&markets),
            Err(err) => format!("Scanner failed: {}", err),
        },

        AgentTool::Unknown => "Unknown AlphaScout command.".to_string(),
    }
}
