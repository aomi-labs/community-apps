use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::time::Duration;
use zora_aomi_tools::{
    agent::{execute_intent, parse_user_input, CoinMonitor, CoinWatch, MonitorConfig},
    load_client_from_env, tools,
    types::{CoinInput, TopBuyersInput},
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let command = Command::parse(std::env::args().skip(1))?;

    match command {
        Command::Help => {
            println!("{}", usage());
        }

        Command::Serve => {
            zora_aomi_tools::server::run_from_env().await?;
        }

        Command::Monitor {
            config,
            interval_secs,
        } => {
            let coins = load_monitor_config(&config)?;
            let client = load_client_from_env()?;
            let monitor_config = MonitorConfig {
                coins,
                poll_interval: Duration::from_secs(interval_secs),
            };
            CoinMonitor::from_env(client, monitor_config).run().await?;
        }

        Command::Query { query } => {
            let intent = parse_user_input(&query)
                .ok_or_else(|| anyhow::anyhow!("Could not understand query"))?;
            let output = execute_intent(&intent).await.map_err(anyhow::Error::msg)?;
            println!("{}", output.replace("\\n", "\n"));
        }

        Command::HolderCount { address, chain } => {
            let client = load_client_from_env()?;
            let output = tools::get_holder_count(CoinInput { address, chain }, &client).await?;
            print_json(&output)?;
        }

        Command::Volume24h { address, chain } => {
            let client = load_client_from_env()?;
            let output = tools::get_24h_volume(CoinInput { address, chain }, &client).await?;
            print_json(&output)?;
        }

        Command::TopBuyers {
            address,
            chain,
            top_n,
        } => {
            let client = load_client_from_env()?;
            let output = tools::get_top_buyers(
                TopBuyersInput {
                    address,
                    chain,
                    top_n,
                },
                &client,
            )
            .await?;
            print_json(&output)?;
        }
    }

    Ok(())
}

// ── Commands ──────────────────────────────────────────────────────────────────

enum Command {
    Help,
    Serve,
    Monitor {
        config: String,
        interval_secs: u64,
    },
    Query {
        query: String,
    },
    HolderCount {
        address: String,
        chain: u64,
    },
    Volume24h {
        address: String,
        chain: u64,
    },
    TopBuyers {
        address: String,
        chain: u64,
        top_n: u32,
    },
}

impl Command {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self> {
        let mut args = args.collect::<Vec<_>>();

        if args.is_empty() || matches!(args[0].as_str(), "-h" | "--help" | "help") {
            return Ok(Self::Help);
        }

        let command = args.remove(0);
        match command.as_str() {
            "serve" | "server" => Ok(Self::Serve),

            "monitor" => {
                let mut config = "monitor.json".to_string();
                let mut interval_secs = 60u64;
                let mut i = 0;
                let mut positional = 0usize;
                while i < args.len() {
                    match args[i].as_str() {
                        "--config" => {
                            i += 1;
                            if let Some(v) = args.get(i) {
                                config = v.clone();
                            }
                        }
                        "--interval" => {
                            i += 1;
                            if let Some(v) = args.get(i) {
                                interval_secs =
                                    v.parse::<u64>().context("--interval must be an integer")?;
                            }
                        }
                        v if !v.starts_with("--") => {
                            match positional {
                                0 => config = v.to_string(),
                                1 => {
                                    interval_secs = v
                                        .parse::<u64>()
                                        .context("interval_secs must be an integer")?;
                                }
                                _ => {}
                            }
                            positional += 1;
                        }
                        _ => {}
                    }
                    i += 1;
                }
                Ok(Self::Monitor {
                    config,
                    interval_secs,
                })
            }

            "query" | "ask" => {
                if args.is_empty() {
                    bail!("missing required argument: query\n\n{}", usage());
                }
                Ok(Self::Query {
                    query: args.join(" "),
                })
            }

            "holder-count" | "get-holder-count" => {
                let address = required_arg(&args, 0, "address")?;
                let chain = optional_chain(&args, 1)?;
                Ok(Self::HolderCount { address, chain })
            }

            "24h-volume" | "get-24h-volume" => {
                let address = required_arg(&args, 0, "address")?;
                let chain = optional_chain(&args, 1)?;
                Ok(Self::Volume24h { address, chain })
            }

            "top-buyers" | "get-top-buyers" => {
                let address = required_arg(&args, 0, "address")?;
                let top_n = optional_u32(&args, 1, 10, "top_n")?;
                let chain = optional_chain(&args, 2)?;
                Ok(Self::TopBuyers {
                    address,
                    chain,
                    top_n,
                })
            }

            _ => bail!("unknown command: {command}\n\n{}", usage()),
        }
    }
}

// ── Monitor config loader ─────────────────────────────────────────────────────

fn load_monitor_config(path: &str) -> Result<Vec<CoinWatch>> {
    #[derive(serde::Deserialize)]
    struct RawWatch {
        name: String,
        address: String,
        chain: Option<u64>,
        holder_delta_threshold: Option<u64>,
        volume_spike_pct: Option<f64>,
    }

    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read monitor config: {path}"))?;
    let watches: Vec<RawWatch> =
        serde_json::from_str(&raw).with_context(|| format!("invalid JSON in {path}"))?;

    Ok(watches
        .into_iter()
        .map(|w| CoinWatch {
            name: w.name,
            address: w.address,
            chain: w
                .chain
                .unwrap_or_else(zora_aomi_tools::types::default_chain),
            holder_delta_threshold: w.holder_delta_threshold.unwrap_or(1),
            volume_spike_pct: w.volume_spike_pct.unwrap_or(10.0),
        })
        .collect())
}

// ── Argument helpers ──────────────────────────────────────────────────────────

fn required_arg(args: &[String], index: usize, name: &str) -> Result<String> {
    args.get(index)
        .cloned()
        .with_context(|| format!("missing required argument: {name}\n\n{}", usage()))
}

fn optional_chain(args: &[String], index: usize) -> Result<u64> {
    optional_u64(
        args,
        index,
        zora_aomi_tools::types::default_chain(),
        "chain",
    )
}

fn optional_u64(args: &[String], index: usize, default: u64, name: &str) -> Result<u64> {
    args.get(index)
        .map(|value| {
            value
                .parse::<u64>()
                .with_context(|| format!("{name} must be an integer"))
        })
        .unwrap_or(Ok(default))
}

fn optional_u32(args: &[String], index: usize, default: u32, name: &str) -> Result<u32> {
    args.get(index)
        .map(|value| {
            value
                .parse::<u32>()
                .with_context(|| format!("{name} must be an integer"))
        })
        .unwrap_or(Ok(default))
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

// ── Usage ─────────────────────────────────────────────────────────────────────

fn usage() -> &'static str {
    "Usage:
  cargo run -- serve                                   Start HTTP + GraphQL server
  cargo run -- monitor [config.json] [interval_secs]  Start autonomous monitor
  cargo run -- query \"<plain-English query>\"          Ask the local NLP agent
  cargo run -- holder-count <address> [chain]          Fetch holder count
  cargo run -- 24h-volume   <address> [chain]          Fetch 24h volume
  cargo run -- top-buyers   <address> [top_n] [chain]  Fetch top buyers

Examples:
  PORT=3001 cargo run -- serve
  cargo run -- monitor monitor.json 60
  cargo run -- query \"Show top 5 buyers of 0x445e9c...\"
  cargo run -- holder-count 0x445e9c... 8453
  cargo run -- 24h-volume   0x445e9c...
  cargo run -- top-buyers   0x445e9c... 5"
}
