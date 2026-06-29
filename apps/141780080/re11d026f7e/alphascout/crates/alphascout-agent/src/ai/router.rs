pub enum Action {
    Chat,
    ScanMarket,
    AnalyzeOdds,
    ExecuteTrade,
    Unknown,
}

pub fn route_intent(input: &str) -> Action {
    let input = input.to_lowercase();

    if input.contains("scan") {
        Action::ScanMarket
    } else if input.contains("odds") {
        Action::AnalyzeOdds
    } else if input.contains("trade") {
        Action::ExecuteTrade
    } else {
        Action::Chat
    }
}
