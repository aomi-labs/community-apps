use anyhow::Result;
use async_graphql::SimpleObject;
use serde::Serialize;

use crate::{client::ZoraClient, tools::aggregate_top_buyers, types::default_chain};

#[derive(Debug, Clone, SimpleObject, Serialize)]
#[graphql(rename_fields = "camelCase")]
pub struct CreatorStats {
    pub wallet: String,
    pub name: String,
    pub avatar: Option<String>,
    pub total_mints: i32,
    pub volume_eth: f64,
    pub unique_holders: i32,
    pub growth30d: f64,
}

#[derive(Debug, Clone, SimpleObject, Serialize)]
#[graphql(rename_fields = "camelCase")]
pub struct VolumePoint {
    pub date: String,
    pub volume: f64,
}

#[derive(Debug, Clone, SimpleObject, Serialize)]
#[graphql(rename_fields = "camelCase")]
pub struct FrontendTopBuyer {
    pub rank: i32,
    pub wallet: String,
    pub percentage: f64,
    pub amount_eth: f64,
}

#[derive(Debug, Clone, SimpleObject, Serialize)]
#[graphql(rename_fields = "camelCase")]
pub struct Collector {
    pub rank: i32,
    pub wallet: String,
    pub coins_held: i32,
    pub first_purchase: String,
    pub total_spent_eth: f64,
    pub badge: String,
}

#[derive(Debug, Clone, SimpleObject, Serialize)]
#[graphql(rename_fields = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub price_eth: f64,
    pub volume_eth: f64,
    pub holders: i32,
    pub thumbnail: Option<String>,
}

#[derive(Clone)]
pub struct FrontendService {
    client: ZoraClient,
}

impl FrontendService {
    pub fn new(client: ZoraClient) -> Self {
        Self { client }
    }

    pub async fn creator_stats(&self, wallet: String) -> Result<CreatorStats> {
        let profile = self
            .client
            .get_profile_coins(&wallet, 50, &[default_chain()])
            .await?;
        let Some(profile) = profile.profile else {
            return Ok(CreatorStats {
                wallet,
                name: "Zora Creator".to_string(),
                avatar: None,
                total_mints: 0,
                volume_eth: 0.0,
                unique_holders: 0,
                growth30d: 0.0,
            });
        };

        let coins = profile
            .created_coins
            .as_ref()
            .map(|coins| coins.edges.as_slice())
            .unwrap_or_default();

        let volume_eth = coins
            .iter()
            .map(|edge| parse_f64(&edge.node.total_volume))
            .sum::<f64>();
        let unique_holders = coins
            .iter()
            .map(|edge| edge.node.unique_holders as i32)
            .sum::<i32>();
        let growth30d = average(
            coins
                .iter()
                .map(|edge| parse_f64(&edge.node.market_cap_delta_24h)),
        );

        Ok(CreatorStats {
            wallet,
            name: profile.handle,
            avatar: profile
                .avatar
                .and_then(|avatar| avatar.preview_image.medium.or(avatar.preview_image.small)),
            total_mints: profile
                .created_coins
                .as_ref()
                .map(|coins| coins.count as i32)
                .unwrap_or_default(),
            volume_eth,
            unique_holders,
            growth30d,
        })
    }

    pub async fn volume_data(&self, wallet: String) -> Result<Vec<VolumePoint>> {
        let Some(coin_address) = self.resolve_primary_coin(&wallet).await? else {
            return Ok(Vec::new());
        };

        let history = self
            .client
            .get_coin_price_history(&coin_address, default_chain())
            .await?;
        let points = history
            .zora20_token
            .map(|token| {
                if token.one_month.is_empty() {
                    token.one_week
                } else {
                    token.one_month
                }
            })
            .unwrap_or_default();

        Ok(points
            .into_iter()
            .map(|point| VolumePoint {
                date: point.timestamp,
                volume: parse_f64(&point.close_price),
            })
            .collect())
    }

    pub async fn top_buyers(&self, wallet: String, top_n: u32) -> Result<Vec<FrontendTopBuyer>> {
        let Some(coin_address) = self.resolve_primary_coin(&wallet).await? else {
            return Ok(Vec::new());
        };

        let swaps = self
            .client
            .get_coin_swaps(
                &coin_address,
                default_chain(),
                top_n.saturating_mul(3).max(1),
                None,
            )
            .await?;
        let edges = swaps
            .data
            .zora20_token
            .and_then(|token| token.swap_activities)
            .map(|activities| activities.edges)
            .unwrap_or_default();

        let buyers = aggregate_top_buyers(edges, top_n as usize);
        let max = buyers
            .first()
            .map(|buyer| parse_f64(&buyer.total_bought))
            .unwrap_or_default();

        Ok(buyers
            .into_iter()
            .map(|buyer| {
                let amount = parse_f64(&buyer.total_bought);
                FrontendTopBuyer {
                    rank: buyer.rank as i32,
                    wallet: buyer.address,
                    percentage: if max > 0.0 {
                        (amount / max) * 100.0
                    } else {
                        0.0
                    },
                    amount_eth: amount,
                }
            })
            .collect())
    }

    pub async fn collectors(&self, wallet: String, count: u32) -> Result<Vec<Collector>> {
        let Some(coin_address) = self.resolve_primary_coin(&wallet).await? else {
            return Ok(Vec::new());
        };

        let holders = self
            .client
            .get_coin_holders(&coin_address, default_chain(), count)
            .await?;
        let mut collectors = holders
            .zora20_token
            .map(|token| token.token_balances.edges)
            .unwrap_or_default()
            .into_iter()
            .map(|edge| (edge.node.owner_address, parse_f64(&edge.node.balance)))
            .collect::<Vec<_>>();

        collectors.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(collectors
            .into_iter()
            .take(count as usize)
            .enumerate()
            .map(|(index, (wallet, balance))| Collector {
                rank: (index + 1) as i32,
                wallet,
                coins_held: balance.round() as i32,
                first_purchase: String::new(),
                total_spent_eth: 0.0,
                badge: collector_badge(balance),
            })
            .collect())
    }

    pub async fn collections(&self, wallet: String, count: u32) -> Result<Vec<Collection>> {
        let profile = self
            .client
            .get_profile_coins(&wallet, count, &[default_chain()])
            .await?;

        Ok(profile
            .profile
            .and_then(|profile| profile.created_coins)
            .map(|coins| coins.edges)
            .unwrap_or_default()
            .into_iter()
            .map(|edge| {
                let coin = edge.node;
                Collection {
                    id: coin.id,
                    name: coin.name,
                    price_eth: coin
                        .token_price
                        .and_then(|price| price.price_in_pool_token)
                        .map(|price| parse_f64(&price))
                        .unwrap_or_default(),
                    volume_eth: parse_f64(&coin.total_volume),
                    holders: coin.unique_holders as i32,
                    thumbnail: coin.media_content.and_then(|media| {
                        media
                            .preview_image
                            .and_then(|image| image.medium.or(image.small))
                    }),
                }
            })
            .collect())
    }

    async fn resolve_primary_coin(&self, wallet: &str) -> Result<Option<String>> {
        if self
            .client
            .get_coin(wallet, default_chain())
            .await
            .ok()
            .and_then(|response| response.data.zora20_token)
            .is_some()
        {
            return Ok(Some(wallet.to_string()));
        }

        let profile = self
            .client
            .get_profile_coins(wallet, 1, &[default_chain()])
            .await?;

        Ok(profile.profile.and_then(|profile| {
            profile.creator_coin.map(|coin| coin.address).or_else(|| {
                profile
                    .created_coins
                    .and_then(|coins| coins.edges.into_iter().next())
                    .map(|edge| edge.node.address)
            })
        }))
    }
}

fn parse_f64(value: &str) -> f64 {
    value.parse::<f64>().unwrap_or_default()
}

fn average(values: impl Iterator<Item = f64>) -> f64 {
    let mut count = 0.0;
    let mut sum = 0.0;
    for value in values {
        count += 1.0;
        sum += value;
    }
    if count == 0.0 {
        0.0
    } else {
        sum / count
    }
}

fn collector_badge(balance: f64) -> String {
    if balance >= 1000.0 {
        "WHALE"
    } else if balance >= 100.0 {
        "FAN"
    } else {
        "NEW"
    }
    .to_string()
}
