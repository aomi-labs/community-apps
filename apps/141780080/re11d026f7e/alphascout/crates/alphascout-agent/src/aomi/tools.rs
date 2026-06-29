#[derive(Debug)]
pub enum AgentTool {
    ScanMarkets,
    AnalyzeOdds,
    HighConviction,
    Unknown,
}

pub fn detect_tool(message: &str) -> AgentTool {
    let lower = message.to_lowercase();

    if lower.contains("scan") {
        AgentTool::ScanMarkets
    } else if lower.contains("analyze") || lower.contains("odds") {
        AgentTool::AnalyzeOdds
    } else if lower.contains("high conviction") {
        AgentTool::HighConviction
    } else {
        AgentTool::Unknown
    }
}
