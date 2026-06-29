use crate::market::limitless::Market;

pub fn build_thesis(market: &Market) -> String {
    format!(
        "Market: {}\nProbability: {:.2}%\n\nAlphaScout Thesis:\nMomentum remains strong but users should verify liquidity and volume before entering.",
        market.title, market.probability
    )
}
