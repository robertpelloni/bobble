use async_trait::async_trait;
use anyhow::Result;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

use serenity::all::{Http};

pub struct DiscordBridge {
    http: Option<Http>,
}

impl DiscordBridge {
    pub fn new() -> Self {
        Self { http: None }
    }
}

#[async_trait]
impl ProtocolBridge for DiscordBridge {
    async fn initialize(&mut self) -> Result<()> {
        // Proof-of-concept initialization mimicking Discord client setup.
        // In a real scenario, a valid bot token would be provided via config or secure storage.
        // We will initialize a dummy HTTP client to fulfill the contract for now.
        let dummy_token = "mock_discord_bot_token";
        let http = Http::new(dummy_token);
        self.http = Some(http);
        Ok(())
    }

    async fn send_message(&self, payload: &MessagePayload) -> Result<()> {
        let _http = self.http.as_ref().ok_or_else(|| anyhow::anyhow!("Discord HTTP client not initialized"))?;

        // Mock Serenity protocol message dispatch
        // A real implementation would parse the channel ID and invoke:
        // let channel_id: u64 = payload.recipient_id.parse()?;
        // ChannelId::new(channel_id).say(http, &payload.content).await?;

        println!("Mock Discord send to {}: {}", payload.recipient_id, payload.content);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        // Discard the HTTP client (if there was a Gateway/WebSocket connection, we'd close it here).
        self.http = None;
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
        let mut bridge = DiscordBridge::new();
        bridge.initialize().await.unwrap();

        let payload = MessagePayload {
            recipient_id: "channel123".to_string(),
            content: "hello discord".to_string(),
            fallback_protocol: "none".to_string(),
        };
        let result = bridge.send_message(&payload).await;
        assert!(result.is_ok());
    }
}
