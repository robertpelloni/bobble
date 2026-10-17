use async_trait::async_trait;
use anyhow::Result;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

pub struct DiscordBridge;

impl DiscordBridge {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProtocolBridge for DiscordBridge {
    async fn initialize(&mut self) -> Result<()> {
        // TODO: Implement Discord Gateway WebSocket initialization
        Ok(())
    }

    async fn send_message(&self, _payload: &MessagePayload) -> Result<()> {
        // TODO: Implement Discord REST API message dispatch
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        // TODO: Implement Discord disconnect
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discord_init() {
        let mut bridge = DiscordBridge::new();
        let result = bridge.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_discord_send() {
        let bridge = DiscordBridge::new();
        let payload = MessagePayload {
            recipient_id: "channel123".to_string(),
            content: "hello discord".to_string(),
            fallback_protocol: "none".to_string(),
        };
        let result = bridge.send_message(&payload).await;
        assert!(result.is_ok());
    }
}
