use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use tokio::time;

use crate::{
    client::ZoraClient,
    tools,
    types::{CoinInput, TopBuyersInput},
};

// ── Watch configuration ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CoinWatch {
    /// Human-readable label shown in pings, e.g. "Aether Series"
    pub name: String,
    /// EVM contract address of the Zora coin
    pub address: String,
    /// Chain ID (8453 = Base mainnet)
    pub chain: u64,
    /// Fire NewHolders event when holder count grows by at least this many
    pub holder_delta_threshold: u64,
    /// Fire VolumeSpike event when 24h volume grows by at least this percent
    pub volume_spike_pct: f64,
}

pub struct MonitorConfig {
    pub coins: Vec<CoinWatch>,
    pub poll_interval: Duration,
}

// ── Events ────────────────────────────────────────────────────────────────────

pub enum MonitorEvent {
    NewHolders {
        coin_name: String,
        coin_address: String,
        total_holders: u64,
        new_holders: u64,
    },
    VolumeSpike {
        coin_name: String,
        coin_address: String,
        volume_24h: f64,
        pct_change: f64,
    },
    NewTopBuyer {
        coin_name: String,
        coin_address: String,
        wallet: String,
    },
}

impl MonitorEvent {
    /// "Daily Wrapped"-style one-liner for notifications.
    pub fn to_ping(&self) -> String {
        match self {
            MonitorEvent::NewHolders {
                coin_name,
                new_holders,
                total_holders,
                ..
            } => {
                format!(
                    "📈 {} — {} new holder{} today (total: {})",
                    coin_name,
                    new_holders,
                    if *new_holders == 1 { "" } else { "s" },
                    total_holders,
                )
            }
            MonitorEvent::VolumeSpike {
                coin_name,
                volume_24h,
                pct_change,
                ..
            } => {
                format!(
                    "🔥 {} — volume spike +{:.0}% (24h: {:.4} ETH)",
                    coin_name, pct_change, volume_24h,
                )
            }
            MonitorEvent::NewTopBuyer {
                coin_name, wallet, ..
            } => {
                let short = if wallet.len() >= 10 {
                    format!("{}…{}", &wallet[..6], &wallet[wallet.len() - 4..])
                } else {
                    wallet.clone()
                };
                format!("🐋 {} — new top buyer: {}", coin_name, short)
            }
        }
    }
}

// ── Notifier trait ────────────────────────────────────────────────────────────

#[async_trait]
pub trait Notifier: Send + Sync {
    async fn notify(&self, event: &MonitorEvent) -> Result<()>;
}

/// Prints events to stdout. Works with zero config — good for local testing.
pub struct StdoutNotifier;

#[async_trait]
impl Notifier for StdoutNotifier {
    async fn notify(&self, event: &MonitorEvent) -> Result<()> {
        let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        println!("[{ts}] {}", event.to_ping());
        Ok(())
    }
}

/// POSTs `{ "text": "..." }` to any webhook URL.
/// Telegram bot webhooks, Slack incoming webhooks, and Discord webhooks all
/// accept this shape out of the box.
pub struct WebhookNotifier {
    pub url: String,
    http: reqwest::Client,
}

impl WebhookNotifier {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Notifier for WebhookNotifier {
    async fn notify(&self, event: &MonitorEvent) -> Result<()> {
        self.http
            .post(&self.url)
            .json(&serde_json::json!({ "text": event.to_ping() }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

// ── Internal state ────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct CoinSnapshot {
    holder_count: u64,
    volume_24h: f64,
    top_buyer: Option<String>,
    /// False on first poll — avoids false-positive triggers on startup.
    initialized: bool,
}

// ── Monitor ───────────────────────────────────────────────────────────────────

pub struct CoinMonitor {
    client: ZoraClient,
    config: MonitorConfig,
    snapshots: HashMap<String, CoinSnapshot>,
    notifier: Box<dyn Notifier>,
}

impl CoinMonitor {
    pub fn new(client: ZoraClient, config: MonitorConfig, notifier: Box<dyn Notifier>) -> Self {
        Self {
            client,
            config,
            snapshots: HashMap::new(),
            notifier,
        }
    }

    /// Reads `MONITOR_WEBHOOK_URL` from env; falls back to stdout if unset.
    pub fn from_env(client: ZoraClient, config: MonitorConfig) -> Self {
        let notifier: Box<dyn Notifier> = match std::env::var("MONITOR_WEBHOOK_URL") {
            Ok(url) => {
                println!("🔔 Webhook notifier → {url}");
                Box::new(WebhookNotifier::new(url))
            }
            Err(_) => {
                println!("🔔 Stdout notifier (set MONITOR_WEBHOOK_URL to forward pings)");
                Box::new(StdoutNotifier)
            }
        };
        Self::new(client, config, notifier)
    }

    /// Starts the polling loop. Runs indefinitely; cancel with Ctrl-C.
    pub async fn run(mut self) -> Result<()> {
        let secs = self.config.poll_interval.as_secs();
        println!(
            "🤖 Monitor running — {} coin(s) watched, polling every {}s\n",
            self.config.coins.len(),
            secs,
        );
        for w in &self.config.coins {
            println!(
                "   • {} (threshold: +{} holders | +{:.0}% volume)",
                w.name, w.holder_delta_threshold, w.volume_spike_pct,
            );
        }
        println!();

        let mut ticker = time::interval(self.config.poll_interval);
        loop {
            ticker.tick().await;
            let watches: Vec<CoinWatch> = self.config.coins.clone();
            for watch in &watches {
                if let Err(e) = self.poll_coin(watch).await {
                    eprintln!("[monitor] ⚠  {}: {e}", watch.name);
                }
            }
        }
    }

    async fn poll_coin(&mut self, watch: &CoinWatch) -> Result<()> {
        let coin_input = CoinInput {
            address: watch.address.clone(),
            chain: watch.chain,
        };

        // Fetch all three data points concurrently
        let (holders_res, volume_res, buyers_res) = tokio::join!(
            tools::get_holder_count(coin_input.clone(), &self.client),
            tools::get_24h_volume(coin_input.clone(), &self.client),
            tools::get_top_buyers(
                TopBuyersInput {
                    address: watch.address.clone(),
                    chain: watch.chain,
                    top_n: 1,
                },
                &self.client,
            ),
        );

        let holders = holders_res?;
        let volume = volume_res?;
        let buyers = buyers_res?;

        let curr_holders = holders.unique_holders;
        let curr_volume: f64 = volume.volume_24h.parse().unwrap_or(0.0);
        let curr_top_buyer = buyers.first().map(|b| b.address.clone());

        let prev = self
            .snapshots
            .entry(watch.address.clone())
            .or_default()
            .clone();

        if prev.initialized {
            // ── Trigger: new holders ──────────────────────────────────────────
            let delta = curr_holders.saturating_sub(prev.holder_count);
            if delta >= watch.holder_delta_threshold {
                self.notifier
                    .notify(&MonitorEvent::NewHolders {
                        coin_name: watch.name.clone(),
                        coin_address: watch.address.clone(),
                        total_holders: curr_holders,
                        new_holders: delta,
                    })
                    .await?;
            }

            // ── Trigger: volume spike ─────────────────────────────────────────
            if prev.volume_24h > 0.0 {
                let pct = (curr_volume - prev.volume_24h) / prev.volume_24h * 100.0;
                if pct >= watch.volume_spike_pct {
                    self.notifier
                        .notify(&MonitorEvent::VolumeSpike {
                            coin_name: watch.name.clone(),
                            coin_address: watch.address.clone(),
                            volume_24h: curr_volume,
                            pct_change: pct,
                        })
                        .await?;
                }
            }

            // ── Trigger: new top buyer ────────────────────────────────────────
            if let Some(ref buyer) = curr_top_buyer {
                if prev.top_buyer.as_deref() != Some(buyer.as_str()) {
                    self.notifier
                        .notify(&MonitorEvent::NewTopBuyer {
                            coin_name: watch.name.clone(),
                            coin_address: watch.address.clone(),
                            wallet: buyer.clone(),
                        })
                        .await?;
                }
            }
        }

        // ── Persist snapshot ──────────────────────────────────────────────────
        self.snapshots.insert(
            watch.address.clone(),
            CoinSnapshot {
                holder_count: curr_holders,
                volume_24h: curr_volume,
                top_buyer: curr_top_buyer,
                initialized: true,
            },
        );

        Ok(())
    }
}
