use super::limitless::Market;

pub fn analyze_market(markets: &[Market]) -> String {
    let mut output = String::new();

    for market in markets {
        let signal = if market.probability > 0.7 {
            "🔥 High Conviction"
        } else {
            "📊 Neutral"
        };

        output.push_str(&format!(
            "{}\nOdds: {:.2}\nSignal: {}\n\n",
            market.title, market.probability, signal
        ));
    }

    output
}

pub fn analyze_odds(markets: &[Market]) -> String {
    let mut output = String::from("📈 AlphaScout Odds Analysis\n\n");

    for market in markets {
        let edge = market.probability * 100.0;

        let analysis = if edge > 70.0 {
            "Bullish momentum detected"
        } else if edge > 60.0 {
            "Moderate confidence"
        } else {
            "Weak setup"
        };

        output.push_str(&format!(
            "🎯 {}\n\
                 Probability: {:.0}%\n\
                 AI Signal: {}\n\n",
            market.title, edge, analysis
        ));
    }

    output
}

pub fn detect_high_edge(markets: &[Market]) -> String {
    let mut output = String::from("🚨 High Edge Opportunities\n\n");

    for market in markets {
        if market.probability > 0.75 {
            output.push_str(&format!(
                "🔥 {}\n\
                     {:.0}% conviction\n\n",
                market.title,
                market.probability * 100.0
            ));
        }
    }

    output
}
