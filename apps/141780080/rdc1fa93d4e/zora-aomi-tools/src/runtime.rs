use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct CommentReceipt {
    pub tx_hash: String,
}

#[async_trait]
pub trait WalletRuntime: Send + Sync {
    async fn post_zora_comment(&self, coin_address: &str, message: &str) -> Result<CommentReceipt>;
}
