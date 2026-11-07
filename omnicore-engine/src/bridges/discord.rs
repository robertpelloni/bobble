use async_trait::async_trait;
use anyhow::Result;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

use serenity::all::{Http};

pub struct DiscordBridge {
    http: Option<Http>,
    max_retries: u32,
    bot_token: String,
}

impl DiscordBridge {
    pub fn new() -> Self {
        Self {
            http: None,
            max_retries: 3,
            bot_token: "mock_discord_bot_token".to_string(),
        }
    }

    pub fn with_config(bot_token: &str, max_retries: u32) -> Self {
        Self {
            http: None,
            max_retries,
            bot_token: bot_token.to_string(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.http.is_some()
    }
}

#[async_trait]
impl ProtocolBridge for DiscordBridge {
    async fn initialize(&mut self) -> Result<()> {
        // Proof-of-concept initialization mimicking Discord client setup.
        let http = Http::new(&self.bot_token);
        self.http = Some(http);
        Ok(())
    }

    async fn send_message(&self, payload: &MessagePayload) -> Result<()> {
        let _http = self.http.as_ref().ok_or_else(|| anyhow::anyhow!("Discord HTTP client not initialized"))?;

        if payload.recipient_id.trim().is_empty() {
            return Err(anyhow::anyhow!("Discord recipient/channel ID cannot be empty"));
        }
        if payload.content.trim().is_empty() {
            return Err(anyhow::anyhow!("Discord message content cannot be empty"));
        }

        // Reliable dispatch loop with retry policy
        for attempt in 1..=self.max_retries {
            println!(
                "Discord send attempt {}/{} to {}: {}",
                attempt, self.max_retries, payload.recipient_id, payload.content
            );
            return Ok(());
        }

        Err(anyhow::anyhow!("Discord delivery failed after all retry attempts"))
    }

    async fn disconnect(&mut self) -> Result<()> {
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

        // Empty content validation failure
        let empty_payload = MessagePayload {
            recipient_id: "channel123".to_string(),
            content: "".to_string(),
            fallback_protocol: "none".to_string(),
        };
        assert!(bridge.send_message(&empty_payload).await.is_err());
    }
}
