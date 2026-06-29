use super::tools::AgentTool;

pub fn reason(message: &str) -> AgentTool {
    let lower = message.to_lowercase();

    if lower.contains("scan") || lower.contains("market") {
        AgentTool::ScanMarkets
    } else if lower.contains("odds") || lower.contains("probability") || lower.contains("analyze") {
        AgentTool::AnalyzeOdds
    } else if lower.contains("high conviction")
        || lower.contains("best trade")
        || lower.contains("alpha")
    {
        AgentTool::HighConviction
    } else {
        AgentTool::Unknown
    }
}
