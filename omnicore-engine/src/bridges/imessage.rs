use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

pub struct IMessageBridge {
    server_url: String,
    pub password: Option<String>,
    connected: bool,
    pub max_retries: u32,
}

impl IMessageBridge {
    pub fn new() -> Self {
        Self {
            server_url: "http://127.0.0.1:1234".to_string(),
            password: None,
            connected: false,
            max_retries: 2,
        }
    }

    pub fn with_config(server_url: &str, password: Option<&str>) -> Self {
        Self {
            server_url: server_url.to_string(),
            password: password.map(|s| s.to_string()),
            connected: false,
            max_retries: 2,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
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

        if payload.recipient_id.trim().is_empty() {
            return Err(anyhow!("Cannot send iMessage to empty recipient"));
        }

        if payload.content.trim().is_empty() {
            return Err(anyhow!("Cannot send empty iMessage content"));
        }

        // Retry loop on transient dispatch failures
        let mut last_err = None;
        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(50 * attempt as u64)).await;
            }

            println!(
                "iMessage relay dispatch attempt {}/{} to {}: {}",
                attempt + 1, self.max_retries + 1, payload.recipient_id, payload.content
            );
            // Simulated network call
            if !self.server_url.is_empty() {
                return Ok(());
            } else {
                last_err = Some(anyhow!("Invalid empty relay server URL"));
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow!("iMessage relay dispatch failed after retries")))
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

    #[tokio::test]
    async fn test_imessage_validation_rules() {
        let mut bridge = IMessageBridge::new();
        bridge.initialize().await.unwrap();

        // Empty recipient
        let res1 = bridge.send_message(&MessagePayload {
            recipient_id: "  ".to_string(),
            content: "hello".to_string(),
            fallback_protocol: "none".to_string(),
        }).await;
        assert!(res1.is_err());
        assert!(res1.unwrap_err().to_string().contains("empty recipient"));

        // Empty content
        let res2 = bridge.send_message(&MessagePayload {
            recipient_id: "user@icloud.com".to_string(),
            content: "".to_string(),
            fallback_protocol: "none".to_string(),
        }).await;
        assert!(res2.is_err());
        assert!(res2.unwrap_err().to_string().contains("empty iMessage content"));
    }
}
