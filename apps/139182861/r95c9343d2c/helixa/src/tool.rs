use crate::client::*;
use aomi_sdk::*;
use serde_json::{Value, json};

pub(crate) struct SearchAgents;
pub(crate) struct GetAgentProfile;
pub(crate) struct CheckCred;
pub(crate) struct CompareAgents;

impl DynAomiTool for SearchAgents {
    type App = HelixaApp;
    type Args = SearchAgentsArgs;
    const NAME: &'static str = "search_agents";
    const DESCRIPTION: &'static str = "Search Helixa AgentDNA for agents, humans, and organizations by name, wallet, skill, or keyword before deciding who to trust or route work to.";

    fn run(_app: &HelixaApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        if args.query.trim().is_empty() {
            return Err("query is required".to_string());
        }
        let client = HelixaClient::new()?;
        let value = client.get_json(&HelixaClient::search_path(
            &args.query,
            clamp_limit(args.limit),
        ))?;
        Ok(normalize_search(value))
    }
}

impl DynAomiTool for GetAgentProfile {
    type App = HelixaApp;
    type Args = GetAgentProfileArgs;
    const NAME: &'static str = "get_agent_profile";
    const DESCRIPTION: &'static str = "Fetch the full Helixa AgentDNA profile for one agent token ID, including identity, wallets, Cred, Aura, traits, credentials, services, and public links.";

    fn run(_app: &HelixaApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let client = HelixaClient::new()?;
        let value = client.get_json(&HelixaClient::agent_path(args.token_id))?;
        Ok(normalize_agent_profile(value))
    }
}

impl DynAomiTool for CheckCred {
    type App = HelixaApp;
    type Args = CheckCredArgs;
    const NAME: &'static str = "check_cred";
    const DESCRIPTION: &'static str = "Check a Helixa agent's Cred score and tier when deciding whether the agent is safe to route work, funds, or collaboration to.";

    fn run(_app: &HelixaApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        let client = HelixaClient::new()?;
        let value = client.get_json(&HelixaClient::cred_path(args.token_id))?;
        Ok(normalize_cred(value))
    }
}

impl DynAomiTool for CompareAgents {
    type App = HelixaApp;
    type Args = CompareAgentsArgs;
    const NAME: &'static str = "compare_agents";
    const DESCRIPTION: &'static str = "Compare two to five Helixa agents by Cred, verification, traits, services, and wallet identity to pick the strongest candidate for a task.";

    fn run(_app: &HelixaApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        if args.token_ids.len() < 2 || args.token_ids.len() > 5 {
            return Err("compare_agents requires 2 to 5 token_ids".to_string());
        }
        let client = HelixaClient::new()?;
        let mut agents = Vec::new();
        for id in args.token_ids {
            let profile = normalize_agent_profile(client.get_json(&HelixaClient::agent_path(id))?);
            let cred = normalize_cred(client.get_json(&HelixaClient::cred_path(id))?);
            let score = cred
                .get("score")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            agents
                .push(json!({ "token_id": id, "score": score, "profile": profile, "cred": cred }));
        }
        agents.sort_by(|a, b| {
            b.get("score")
                .and_then(Value::as_i64)
                .cmp(&a.get("score").and_then(Value::as_i64))
        });
        let best_for_trust = agents.first().cloned().unwrap_or_else(|| json!(null));
        let mut capability_ranked = agents.clone();
        capability_ranked.sort_by(|a, b| capability_signal(b).cmp(&capability_signal(a)));
        let best_for_capabilities = capability_ranked
            .first()
            .cloned()
            .unwrap_or_else(|| json!(null));
        Ok(json!({
            "agents": agents,
            "best_for_trust": best_for_trust,
            "best_for_capabilities": best_for_capabilities,
            "notes": [
                "Cred is a routing signal, not a guarantee.",
                "Review services, wallet ownership, and credentials before high-value work."
            ]
        }))
    }
}

fn capability_signal(agent: &Value) -> i64 {
    if let Some(profile) = agent.get("profile") {
        array_len(profile.pointer("/credentials/skills"))
            + array_len(profile.pointer("/credentials/domains"))
            + array_len(profile.get("traits"))
            + object_len(profile.get("services"))
    } else {
        0
    }
}

fn array_len(value: Option<&Value>) -> i64 {
    value
        .and_then(Value::as_array)
        .map(|items| items.len() as i64)
        .unwrap_or_default()
}

fn object_len(value: Option<&Value>) -> i64 {
    value
        .and_then(Value::as_object)
        .map(|items| items.len() as i64)
        .unwrap_or_default()
}
