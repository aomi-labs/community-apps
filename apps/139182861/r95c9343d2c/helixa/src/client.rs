use aomi_sdk::schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;

const BASE_URL: &str = "https://api.helixa.xyz";

#[derive(Clone, Debug, Default)]
pub(crate) struct HelixaApp;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SearchAgentsArgs {
    /// Search query: agent name, wallet address, human name, organization, skill, or keyword.
    pub query: String,
    /// Maximum results to return. Defaults to 5 and clamps to 20.
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetAgentProfileArgs {
    /// Helixa agent token ID on Base.
    pub token_id: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CheckCredArgs {
    /// Helixa agent token ID on Base.
    pub token_id: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CompareAgentsArgs {
    /// Two to five Helixa agent token IDs to compare.
    pub token_ids: Vec<u64>,
}

pub(crate) struct HelixaClient {
    http: reqwest::blocking::Client,
    base_url: String,
}

impl HelixaClient {
    pub(crate) fn new() -> Result<Self, String> {
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(12))
            .user_agent("aomi-helixa/0.1.0")
            .build()
            .map_err(|err| format!("failed to create Helixa HTTP client: {err}"))?;
        Ok(Self {
            http,
            base_url: BASE_URL.to_string(),
        })
    }

    pub(crate) fn search_path(query: &str, limit: u32) -> String {
        format!("/api/v2/search?q={}&limit={}", url_component(query), limit)
    }

    pub(crate) fn agent_path(token_id: u64) -> String {
        format!("/api/v2/agent/{token_id}")
    }

    pub(crate) fn cred_path(token_id: u64) -> String {
        format!("/api/v2/agent/{token_id}/cred")
    }

    pub(crate) fn get_json(&self, path: &str) -> Result<Value, String> {
        let url = format!("{}{}", self.base_url, path);
        let res = self
            .http
            .get(&url)
            .send()
            .map_err(|err| format!("Helixa API request failed: {err}"))?;
        let status = res.status();
        let value: Value = res
            .json()
            .map_err(|err| format!("Helixa API returned invalid JSON: {err}"))?;
        if !status.is_success() {
            let msg = value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("Helixa API error");
            return Err(format!("{msg} (HTTP {status})"));
        }
        Ok(value)
    }
}

pub(crate) fn clamp_limit(limit: Option<u32>) -> u32 {
    limit.unwrap_or(5).clamp(1, 20)
}

pub(crate) fn normalize_search(value: Value) -> Value {
    json!({
        "query": value.get("query"),
        "total": value.get("total"),
        "agents": normalize_list(value.get("agents")),
        "humans": normalize_list(value.get("humans")),
        "organizations": normalize_list(value.get("organizations")),
        "principals": normalize_list(value.get("principals")),
    })
}

fn normalize_list(value: Option<&Value>) -> Vec<Value> {
    value.and_then(Value::as_array).map(|items| {
        items.iter().map(|item| json!({
            "entity_type": item.get("entityType"),
            "id": item.get("id").or_else(|| item.get("tokenId")),
            "token_id": item.get("tokenId"),
            "name": item.get("name"),
            "framework": item.get("framework"),
            "description": item.get("description"),
            "cred_score": item.get("credScore"),
            "tier": item.get("tier"),
            "tier_label": item.get("tierLabel"),
            "verified": item.get("verified"),
            "skills": item.get("skills"),
            "service_categories": item.get("serviceCategories"),
            "suggested_actions": item.get("suggested_actions"),
            "profile_urls": {
                "profile": item.get("suggested_actions").and_then(|actions| actions.get("profile")),
                "cred": item.get("suggested_actions").and_then(|actions| actions.get("cred")),
                "card": item.get("suggested_actions").and_then(|actions| actions.get("card")),
                "public_profile": item.get("suggested_actions").and_then(|actions| actions.get("publicProfile")),
            },
        })).collect()
    }).unwrap_or_default()
}

pub(crate) fn normalize_agent_profile(value: Value) -> Value {
    json!({
        "identity": {
            "token_id": value.get("tokenId"),
            "name": value.get("name"),
            "framework": value.get("framework"),
            "mint_origin": value.get("mintOrigin"),
            "verified": value.get("verified"),
            "soulbound": value.get("soulbound"),
            "minted_at": value.get("mintedAt"),
            "generation": value.get("generation"),
        },
        "wallets": {
            "agent_address": value.get("agentAddress"),
            "owner": value.get("owner"),
            "operator": value.get("operator"),
        },
        "cred": {
            "score": value.get("credScore"),
            "points": value.get("points"),
            "ethos_score": value.get("ethosScore"),
            "talent_score": value.get("talentScore"),
        },
        "aura": {
            "personality": value.get("personality"),
            "narrative": value.get("narrative"),
        },
        "traits": value.get("traits"),
        "credentials": {
            "socials": value.get("socials"),
            "skills": value.get("skills"),
            "domains": value.get("domains"),
            "linked_token": value.get("linkedToken"),
        },
        "services": value.get("services"),
        "metadata": value.get("metadata"),
        "links": {
            "explorer": value.get("explorer"),
            "public_profile": value.get("tokenId").and_then(Value::as_u64).map(|id| format!("https://helixa.xyz/agent/{id}")),
            "api_profile": value.get("tokenId").and_then(Value::as_u64).map(|id| format!("https://api.helixa.xyz/api/v2/agent/{id}")),
        }
    })
}

pub(crate) fn normalize_cred(value: Value) -> Value {
    let score = value
        .get("credScore")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let tier = value.get("tier").and_then(Value::as_str).unwrap_or("");
    json!({
        "token_id": value.get("tokenId"),
        "name": value.get("name"),
        "score": score,
        "tier": tier,
        "tier_label": value.get("tierLabel"),
        "scale": value.get("scale"),
        "full_report_endpoint": value.get("fullReportEndpoint"),
        "recommendation": trust_recommendation(score, tier),
    })
}

pub(crate) fn trust_recommendation(score: i64, tier: &str) -> &'static str {
    match tier.to_ascii_uppercase().as_str() {
        "PREFERRED" => "highest-trust candidate",
        "PRIME" => "strong candidate for trusted routing",
        "QUALIFIED" => "acceptable for normal collaboration with review",
        "MARGINAL" => "use only for low-risk exploration",
        "JUNK" => "do not route paid or sensitive work",
        _ if score >= 91 => "highest-trust candidate",
        _ if score >= 76 => "strong candidate for trusted routing",
        _ if score >= 51 => "acceptable for normal collaboration with review",
        _ if score >= 26 => "use only for low-risk exploration",
        _ => "do not route paid or sensitive work",
    }
}

pub(crate) fn url_component(input: &str) -> String {
    let mut out = String::new();
    for byte in input.trim().bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_paths_are_stable() {
        assert_eq!(
            HelixaClient::search_path("bendr", 3),
            "/api/v2/search?q=bendr&limit=3"
        );
        assert_eq!(
            HelixaClient::search_path("aomi & helixa", 5),
            "/api/v2/search?q=aomi%20%26%20helixa&limit=5"
        );
        assert_eq!(HelixaClient::agent_path(1), "/api/v2/agent/1");
        assert_eq!(HelixaClient::cred_path(1), "/api/v2/agent/1/cred");
    }

    #[test]
    fn trust_recommendation_matches_tiers() {
        assert_eq!(
            trust_recommendation(12, "JUNK"),
            "do not route paid or sensitive work"
        );
        assert_eq!(
            trust_recommendation(45, "MARGINAL"),
            "use only for low-risk exploration"
        );
        assert_eq!(
            trust_recommendation(66, "QUALIFIED"),
            "acceptable for normal collaboration with review"
        );
        assert_eq!(
            trust_recommendation(80, "PRIME"),
            "strong candidate for trusted routing"
        );
        assert_eq!(
            trust_recommendation(95, "PREFERRED"),
            "highest-trust candidate"
        );
    }
}
