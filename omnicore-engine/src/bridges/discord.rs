use async_trait::async_trait;
use anyhow::Result;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

use serenity::all::{Http};

/// Discord bridge.
///
/// HONESTY NOTE (v1.20.2): `initialize` only *constructs* a serenity
/// `Http` object around a bot token. `send_message` never calls it —
/// no channel send was ever wired — so a production bridge returns an
/// explicit not-implemented error instead of the previous behaviour
/// (print "send attempt 1/3" and report success). The simulated send
/// exists only behind `new_mock()`, for tests.
pub struct DiscordBridge {
    http: Option<Http>,
    max_retries: u32,
    bot_token: String,
    mock: bool,
}

impl DiscordBridge {
    pub fn new() -> Self {
        Self::build("mock_discord_bot_token".to_string(), 3, false)
    }

    pub fn with_config(bot_token: &str, max_retries: u32) -> Self {
        Self::build(bot_token.to_string(), max_retries, false)
    }

    /// Explicit mock transport: sends "succeed" by printing, without any
    /// network delivery. For tests/simulations only.
    pub fn new_mock() -> Self {
        Self::build("mock_discord_bot_token".to_string(), 3, true)
    }

    fn build(bot_token: String, max_retries: u32, mock: bool) -> Self {
        Self {
            http: None,
            max_retries,
            bot_token,
            mock,
        }
    }

    pub fn is_mock(&self) -> bool {
        self.mock
    }

    /// True only when an Http object has been constructed. This does
    /// NOT mean the bot token was validated or can deliver messages.
    pub fn is_connected(&self) -> bool {
        self.http.is_some()
    }
}

#[async_trait]
impl ProtocolBridge for DiscordBridge {
    async fn initialize(&mut self) -> Result<()> {
        // Constructs an Http client around the bot token. The token is
        // not validated against Discord and no gateway connection opens.
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

        if self.mock {
            println!(
                "Mock Discord send to {}: {}",
                payload.recipient_id, payload.content
            );
            return Ok(());
        }

        // No channel send was ever wired to the Http client. Fail
        // honestly instead of printing a fake attempt and returning
        // success.
        let _ = self.max_retries;
        Err(anyhow::anyhow!(
            "Discord live delivery not implemented: no channel send is wired to the bot HTTP client \
             (mock transport: DiscordBridge::new_mock())"
        ))
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
    async fn test_discord_send_not_implemented_in_production() {
        let mut bridge = DiscordBridge::new();
        bridge.initialize().await.unwrap();

        let payload = MessagePayload {
            recipient_id: "channel123".to_string(),
            content: "hello discord".to_string(),
            fallback_protocol: "none".to_string(),
        };
        let result = bridge.send_message(&payload).await;
        assert!(result.is_err(), "production Discord send must not fake delivery");
        assert!(result.unwrap_err().to_string().contains("not implemented"));

        // Empty content validation still fires.
        let empty_payload = MessagePayload {
            recipient_id: "channel123".to_string(),
            content: "".to_string(),
            fallback_protocol: "none".to_string(),
        };
        assert!(bridge.send_message(&empty_payload).await.is_err());
    }

    #[tokio::test]
    async fn test_discord_mock_send() {
        let mut bridge = DiscordBridge::new_mock();
        assert!(bridge.is_mock());
        bridge.initialize().await.unwrap();

        let payload = MessagePayload {
            recipient_id: "channel123".to_string(),
            content: "hello discord".to_string(),
            fallback_protocol: "none".to_string(),
        };
        assert!(bridge.send_message(&payload).await.is_ok());
    }
}
