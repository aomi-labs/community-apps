use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct Market {
    pub title: String,
    pub probability: f64,
}

#[derive(Debug, Deserialize)]
struct Event {
    title: String,
}

pub async fn fetch_live_markets() -> Result<Vec<Market>, reqwest::Error> {
    let url = "https://gamma-api.polymarket.com/events";

    let response = reqwest::get(url).await?.json::<Vec<Event>>().await?;

    let markets = response
        .into_iter()
        .take(10)
        .map(|event| Market {
            title: event.title,
            probability: 0.50,
        })
        .collect();

    Ok(markets)
}
