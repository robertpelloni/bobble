use async_trait::async_trait;
use anyhow::Result;
use crate::dispatcher::MessagePayload;

#[async_trait]
pub trait ProtocolBridge: Send + Sync {
    async fn initialize(&mut self) -> Result<()>;
    async fn send_message(&self, payload: &MessagePayload) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
}
