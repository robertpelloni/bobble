use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

pub struct IMessageBridge {
    server_url: String,
    pub password: Option<String>,
    connected: bool,
}

impl IMessageBridge {
    pub fn new() -> Self {
        Self {
            server_url: "http://127.0.0.1:1234".to_string(),
            password: None,
            connected: false,
        }
    }

    pub fn with_config(server_url: &str, password: Option<&str>) -> Self {
        Self {
            server_url: server_url.to_string(),
            password: password.map(|s| s.to_string()),
            connected: false,
        }
    }
}

#[async_trait]
impl ProtocolBridge for IMessageBridge {
    async fn initialize(&mut self) -> Result<()> {
        println!(
            "Connecting to iMessage relay server at {} (auth: {})",
            self.server_url,
            if self.password.is_some() { "configured" } else { "none" }
        );
        self.connected = true;
        Ok(())
    }

    async fn send_message(&self, payload: &MessagePayload) -> Result<()> {
        if !self.connected {
            return Err(anyhow!("iMessage relay server bridge not connected"));
        }

        println!(
            "Mock iMessage relay send to {}: {}",
            payload.recipient_id, payload.content
        );
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_imessage_init_and_send() {
        let mut bridge = IMessageBridge::with_config("http://127.0.0.1:1234", Some("secret"));
        assert!(bridge.initialize().await.is_ok());

        let payload = MessagePayload {
            recipient_id: "user@icloud.com".to_string(),
            content: "Hello from OmniCore iMessage bridge".to_string(),
            fallback_protocol: "SMS".to_string(),
        };

        assert!(bridge.send_message(&payload).await.is_ok());
        assert!(bridge.disconnect().await.is_ok());
        assert!(bridge.send_message(&payload).await.is_err());
    }
}
