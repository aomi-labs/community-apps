pub mod agent;
#[cfg(feature = "aomi-plugin")]
pub mod aomi;
#[cfg(feature = "aomi-plugin")]
mod aomi_tests;
pub mod client;
pub mod frontend;
pub mod runtime;
pub mod server;
pub mod tools;
pub mod types;

pub use client::ZoraClient;
pub use runtime::{CommentReceipt, WalletRuntime};

use anyhow::{Context, Result};

pub fn load_client_from_env() -> Result<ZoraClient> {
    dotenvy::dotenv().ok();
    let api_key = std::env::var("ZORA_API_KEY").context("ZORA_API_KEY must be set")?;
    ZoraClient::new(api_key)
}
