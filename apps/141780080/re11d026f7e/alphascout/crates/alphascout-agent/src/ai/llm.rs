use reqwest::Client;
use serde_json::{Value, json};

pub async fn ask_llm(prompt: &str, api_key: &str) -> Result<String, reqwest::Error> {
    let client = Client::new();

    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&json!({
            "model": "gpt-4o-mini",
            "messages": [
                {
                    "role": "system",
                    "content": "You are AlphaScout, a crypto market intelligence agent."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send()
        .await?
        .json::<Value>()
        .await?;

    let output = res["choices"]
        .get(0)
        .and_then(|c| c["message"]["content"].as_str())
        .unwrap_or("")
        .to_string();

    Ok(output)
}
