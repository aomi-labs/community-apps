use aomi_sdk::{DynAomiTool, DynToolCallCtx};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::client::{host_handoff, AgentexApp, AuthenticatedHostIdentity};

const APP_NAME: &str = "agentex";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentRef {
    /// ERC-8004 registry reference for the agent.
    pub agent_registry: String,
    /// Agent ID inside the registry.
    pub agent_id: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetAgentStateArgs {
    /// Optional exchange-round agents. When present, Agentex returns the ring order.
    #[serde(default)]
    pub agents: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AomiTradeSource {
    /// Hosted Aomi app name. The Rust wrapper fills this from the current app contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    /// Aomi session ID for the current agent. The Rust wrapper fills this from the host session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Optional Aomi thread ID supplied by the host session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AomiTradeContext {
    /// Optional session or thread metadata. The Rust wrapper normalizes this from host context.
    #[serde(default)]
    pub source: AomiTradeSource,
    /// Structured trade execution context with TxHash, venue, pair, side, size, fill price, reasoning, and reflection.
    pub trade: Value,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PrepareWhitelistedTradeArgs {
    /// Target chain ID.
    pub chain_id: u64,
    /// Agentex whitelisted venue ID.
    pub whitelisted_venue_id: String,
    /// Pair to trade, such as ETH/USDC.
    pub pair: String,
    /// buy or sell.
    pub side: String,
    /// Order size.
    pub size: String,
    /// Optional maximum slippage in basis points.
    #[serde(default)]
    pub max_slippage_bps: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecordTradeExecutionArgs {
    /// Aomi-generated trade context after transaction simulation and signing.
    pub trade_context: AomiTradeContext,
    /// Explicit confirmation required after Aomi returns a transaction hash.
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PrepareExperienceSaleArgs {
    /// Aomi-generated trade context containing exactly one executed trade.
    pub trade_context: AomiTradeContext,
    /// Seller ERC-8004 identity. When omitted, the wrapper resolves it from host session state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_agent: Option<AgentRef>,
    /// Listing price amount.
    pub price_amount: String,
    /// Payment asset, such as USDFC.
    pub payment_asset: String,
    /// Optional output directory for generated Agentex artifacts.
    #[serde(default)]
    pub out_dir: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PublishExperienceSaleArgs {
    /// Aomi-generated trade context containing exactly one executed trade.
    pub trade_context: AomiTradeContext,
    /// Seller ERC-8004 identity. When omitted, the wrapper resolves it from host session state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_agent: Option<AgentRef>,
    /// Experience encryption key supplied by the host.
    pub key: String,
    /// Whitelisted venue decoder ID.
    pub decoder_id: String,
    /// Seller nonce for registry attestation.
    pub seller_nonce: String,
    /// ISO timestamp after which the attestation expires.
    pub attestation_deadline: String,
    /// Agentex registry contract address.
    pub registry_address: String,
    /// Listing price amount.
    pub price_amount: String,
    /// Payment asset, such as USDFC.
    pub payment_asset: String,
    /// Optional output directory for generated Agentex artifacts.
    #[serde(default)]
    pub out_dir: Option<String>,
    /// Require live Filecoin-backed storage before listing.
    #[serde(default)]
    pub live: bool,
    /// Explicit confirmation required for public writes.
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListingPathArgs {
    /// Agentex listing JSON path.
    pub listing_path: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseExperienceAccessArgs {
    /// Agentex listing JSON path.
    pub listing_path: String,
    /// Buyer ERC-8004 identity. When omitted, the wrapper resolves it from host session state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buyer_agent: Option<AgentRef>,
    /// Filecoin Pay payment reference created by the host or settlement flow.
    pub filecoin_pay_reference: String,
    /// Existing Arkhai escrow UID when the host created settlement externally.
    #[serde(default)]
    pub escrow_id: Option<String>,
    /// Buyer-specific decryption key envelope.
    pub key_envelope: String,
    /// Delivery proof text or reference.
    pub delivery_proof: String,
    /// Explicit confirmation required for purchase settlement.
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyAndStoreExperienceArgs {
    /// Agentex purchase receipt path.
    pub purchase_receipt_path: String,
    /// Experience decryption key supplied by the host.
    pub key: String,
    /// Optional Agentex buyer-state storage directory.
    #[serde(default)]
    pub store_dir: Option<String>,
    /// Explicit confirmation required before writing into buyer memory.
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecordExperienceFeedbackArgs {
    /// Agentex purchase receipt path.
    pub purchase_receipt_path: String,
    /// Numeric usefulness score from the buyer.
    pub score: f64,
    /// Optional concise buyer note.
    #[serde(default)]
    pub note: Option<String>,
}

macro_rules! service_tool {
    ($tool:ident, $args:ty, $name:literal, $description:literal) => {
        pub struct $tool;

        impl DynAomiTool for $tool {
            type App = AgentexApp;
            type Args = $args;
            const NAME: &'static str = $name;
            const DESCRIPTION: &'static str = $description;

            fn run(app: &AgentexApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
                app.post_tool(Self::NAME, &args)
            }
        }
    };
}

service_tool!(
    GetAgentState,
    GetAgentStateArgs,
    "get_agent_state",
    "Read durable Agentex state, available tools, and optional exchange-round state for the current Aomi trading agent."
);

service_tool!(
    PrepareWhitelistedTrade,
    PrepareWhitelistedTradeArgs,
    "prepare_whitelisted_trade",
    "Prepare a whitelisted venue trade intent for Aomi transaction simulation and signing."
);

pub struct RecordTradeExecution;

impl DynAomiTool for RecordTradeExecution {
    type App = AgentexApp;
    type Args = RecordTradeExecutionArgs;
    const NAME: &'static str = "record_trade_execution";
    const DESCRIPTION: &'static str =
        "Record the Aomi-executed trade context and TxHash after explicit confirmation.";

    fn run(app: &AgentexApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let args = RecordTradeExecutionArgs {
            trade_context: normalize_trade_context(args.trade_context, &ctx)?,
            confirm: args.confirm,
        };
        if !args.confirm {
            return Ok(host_handoff(Self::NAME, json!(args)));
        }
        let host_identity = authenticated_host_identity(&ctx, false)?;
        app.post_tool_as_host(Self::NAME, &args, &host_identity)
    }
}

pub struct PrepareExperienceSale;

impl DynAomiTool for PrepareExperienceSale {
    type App = AgentexApp;
    type Args = PrepareExperienceSaleArgs;
    const NAME: &'static str = "prepare_experience_sale";
    const DESCRIPTION: &'static str =
        "Preview the exact trade experience, public summary, listing terms, and risks before publishing.";

    fn run(app: &AgentexApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let args = PrepareExperienceSaleArgs {
            trade_context: normalize_trade_context(args.trade_context, &ctx)?,
            seller_agent: Some(resolve_host_agent(args.seller_agent.as_ref(), "seller_agent", &ctx)?),
            price_amount: args.price_amount,
            payment_asset: args.payment_asset,
            out_dir: args.out_dir,
        };
        let host_identity = authenticated_host_identity(&ctx, true)?;
        app.post_tool_as_host(Self::NAME, &args, &host_identity)
    }
}

pub struct PublishExperienceSale;

impl DynAomiTool for PublishExperienceSale {
    type App = AgentexApp;
    type Args = PublishExperienceSaleArgs;
    const NAME: &'static str = "publish_experience_sale";
    const DESCRIPTION: &'static str =
        "Publish the current agent's encrypted experience, proof, attestation, and listing after confirmation.";

    fn run(app: &AgentexApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let args = PublishExperienceSaleArgs {
            trade_context: normalize_trade_context(args.trade_context, &ctx)?,
            seller_agent: Some(resolve_host_agent(args.seller_agent.as_ref(), "seller_agent", &ctx)?),
            key: args.key,
            decoder_id: args.decoder_id,
            seller_nonce: args.seller_nonce,
            attestation_deadline: args.attestation_deadline,
            registry_address: args.registry_address,
            price_amount: args.price_amount,
            payment_asset: args.payment_asset,
            out_dir: args.out_dir,
            live: args.live,
            confirm: args.confirm,
        };
        if !args.confirm {
            return Ok(host_handoff(Self::NAME, json!(args)));
        }
        let host_identity = authenticated_host_identity(&ctx, true)?;
        app.post_tool_as_host(Self::NAME, &args, &host_identity)
    }
}

service_tool!(
    EvaluateExperienceListing,
    ListingPathArgs,
    "evaluate_experience_listing",
    "Inspect a listing's public trade summary, proof bindings, price, and delivery terms before purchase."
);

pub struct PurchaseExperienceAccess;

impl DynAomiTool for PurchaseExperienceAccess {
    type App = AgentexApp;
    type Args = PurchaseExperienceAccessArgs;
    const NAME: &'static str = "purchase_experience_access";
    const DESCRIPTION: &'static str =
        "Purchase decryption access for the current buyer agent after explicit confirmation.";

    fn run(app: &AgentexApp, args: Self::Args, ctx: DynToolCallCtx) -> Result<Value, String> {
        let args = PurchaseExperienceAccessArgs {
            listing_path: args.listing_path,
            buyer_agent: Some(resolve_host_agent(args.buyer_agent.as_ref(), "buyer_agent", &ctx)?),
            filecoin_pay_reference: args.filecoin_pay_reference,
            escrow_id: args.escrow_id,
            key_envelope: args.key_envelope,
            delivery_proof: args.delivery_proof,
            confirm: args.confirm,
        };
        if !args.confirm {
            return Ok(host_handoff(Self::NAME, json!(args)));
        }
        let host_identity = authenticated_host_identity(&ctx, true)?;
        app.post_tool_as_host(Self::NAME, &args, &host_identity)
    }
}

pub struct VerifyAndStoreExperience;

impl DynAomiTool for VerifyAndStoreExperience {
    type App = AgentexApp;
    type Args = VerifyAndStoreExperienceArgs;
    const NAME: &'static str = "verify_and_store_experience";
    const DESCRIPTION: &'static str =
        "Verify decrypted content and store it in Agentex buyer state after confirmation.";

    fn run(app: &AgentexApp, args: Self::Args, _ctx: DynToolCallCtx) -> Result<Value, String> {
        if !args.confirm {
            return Ok(host_handoff(Self::NAME, json!(args)));
        }
        app.post_tool(Self::NAME, &args)
    }
}

service_tool!(
    RecordExperienceFeedback,
    RecordExperienceFeedbackArgs,
    "record_experience_feedback",
    "Record buyer feedback after verified experience delivery."
);

fn normalize_trade_context(
    mut trade_context: AomiTradeContext,
    ctx: &DynToolCallCtx,
) -> Result<AomiTradeContext, String> {
    trade_context.source.app = Some(APP_NAME.to_string());
    trade_context.source.session_id = Some(ctx.session_id.clone());
    trade_context.source.thread_id = host_thread_id(ctx);
    Ok(trade_context)
}

fn resolve_host_agent(
    provided: Option<&AgentRef>,
    field_name: &str,
    ctx: &DynToolCallCtx,
) -> Result<AgentRef, String> {
    let host = host_agent(ctx)?;
    if let Some(provided) = provided {
        if provided != &host {
            return Err(format!("{field_name} must match the host session identity"));
        }
    }
    Ok(host)
}

fn authenticated_host_identity(ctx: &DynToolCallCtx, require_agent: bool) -> Result<AuthenticatedHostIdentity, String> {
    let agent = if require_agent { Some(host_agent(ctx)?) } else { optional_host_agent(ctx) };
    Ok(AuthenticatedHostIdentity {
        session_id: ctx.session_id.clone(),
        thread_id: host_thread_id(ctx),
        agent_registry: agent.as_ref().map(|agent| agent.agent_registry.clone()),
        agent_id: agent.as_ref().map(|agent| agent.agent_id.clone()),
    })
}

fn host_agent(ctx: &DynToolCallCtx) -> Result<AgentRef, String> {
    let agent_registry = attribute_string_any(
        ctx,
        &[
            &["agent", "agentRegistry"],
            &["agent", "agent_registry"],
            &["identity", "agentRegistry"],
            &["identity", "agent_registry"],
            &["agentRegistry"],
            &["agent_registry"],
        ],
    )
    .ok_or_else(|| "host session identity missing agent registry".to_string())?;
    let agent_id = attribute_string_any(
        ctx,
        &[
            &["agent", "agentId"],
            &["agent", "agent_id"],
            &["identity", "agentId"],
            &["identity", "agent_id"],
            &["agentId"],
            &["agent_id"],
        ],
    )
    .ok_or_else(|| "host session identity missing agent id".to_string())?;

    Ok(AgentRef {
        agent_registry,
        agent_id,
    })
}

fn optional_host_agent(ctx: &DynToolCallCtx) -> Option<AgentRef> {
    let agent_registry = attribute_string_any(
        ctx,
        &[
            &["agent", "agentRegistry"],
            &["agent", "agent_registry"],
            &["identity", "agentRegistry"],
            &["identity", "agent_registry"],
            &["agentRegistry"],
            &["agent_registry"],
        ],
    )?;
    let agent_id = attribute_string_any(
        ctx,
        &[
            &["agent", "agentId"],
            &["agent", "agent_id"],
            &["identity", "agentId"],
            &["identity", "agent_id"],
            &["agentId"],
            &["agent_id"],
        ],
    )?;
    Some(AgentRef {
        agent_registry,
        agent_id,
    })
}

fn host_thread_id(ctx: &DynToolCallCtx) -> Option<String> {
    attribute_string_any(
        ctx,
        &[
            &["aomi", "threadId"],
            &["aomi", "thread_id"],
            &["threadId"],
            &["thread_id"],
        ],
    )
}

fn attribute_string_any(ctx: &DynToolCallCtx, paths: &[&[&str]]) -> Option<String> {
    paths.iter().find_map(|path| ctx.attribute_string(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aomi_sdk::testing::TestCtxBuilder;
    use serde_json::json;

    fn host_ctx(tool_name: &str) -> DynToolCallCtx {
        TestCtxBuilder::new(tool_name)
            .session_id("host-session")
            .attribute(
                "agent",
                json!({
                    "agentRegistry": "eip155:8453:0xhost",
                    "agentId": "42"
                }),
            )
            .attribute(
                "aomi",
                json!({
                    "threadId": "host-thread"
                }),
            )
            .build()
    }

    #[test]
    fn resolves_trade_context_from_host_session() {
        let ctx = host_ctx("record_trade_execution");
        let resolved = normalize_trade_context(
            AomiTradeContext {
                source: AomiTradeSource {
                    app: Some("spoofed-app".to_string()),
                    session_id: Some("spoofed-session".to_string()),
                    thread_id: Some("spoofed-thread".to_string()),
                },
                trade: json!({"trade_tx_hash": "0xabc"}),
            },
            &ctx,
        )
        .expect("host context should normalize trade context");

        assert_eq!(resolved.source.app.as_deref(), Some("agentex"));
        assert_eq!(resolved.source.session_id.as_deref(), Some("host-session"));
        assert_eq!(resolved.source.thread_id.as_deref(), Some("host-thread"));
    }

    #[test]
    fn rejects_spoofed_host_agent_identity() {
        let ctx = host_ctx("publish_experience_sale");
        let error = resolve_host_agent(
            Some(&AgentRef {
                agent_registry: "eip155:8453:0xspoofed".to_string(),
                agent_id: "999".to_string(),
            }),
            "seller_agent",
            &ctx,
        )
        .expect_err("spoofed agent identity should be rejected");

        assert!(error.contains("seller_agent"));
        assert!(error.contains("host session identity"));
    }

    #[test]
    fn resolves_host_agent_identity_when_arg_is_missing() {
        let ctx = host_ctx("purchase_experience_access");
        let resolved = resolve_host_agent(None, "buyer_agent", &ctx).expect("host identity should be available");

        assert_eq!(resolved.agent_registry, "eip155:8453:0xhost");
        assert_eq!(resolved.agent_id, "42");
    }
}
