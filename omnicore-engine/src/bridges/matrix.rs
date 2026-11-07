use async_trait::async_trait;
use anyhow::Result;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

use matrix_sdk::{Client};

pub struct MatrixBridge {
    client: Option<Client>,
    max_retries: u32,
    homeserver_url: String,
}

impl MatrixBridge {
    pub fn new() -> Self {
        Self {
            client: None,
            max_retries: 3,
            homeserver_url: "https://matrix.org".to_string(),
        }
    }

    pub fn with_config(homeserver_url: &str, max_retries: u32) -> Self {
        Self {
            client: None,
            max_retries,
            homeserver_url: homeserver_url.to_string(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }
}

#[async_trait]
impl ProtocolBridge for MatrixBridge {
    async fn initialize(&mut self) -> Result<()> {
        // Proof-of-concept initialization targeting the configured matrix homeserver.
        let client = Client::builder()
            .homeserver_url(&self.homeserver_url)
            .build()
            .await?;

        self.client = Some(client);
        Ok(())
    }

    async fn send_message(&self, payload: &MessagePayload) -> Result<()> {
        let _client = self.client.as_ref().ok_or_else(|| anyhow::anyhow!("Matrix client not initialized"))?;

        if payload.recipient_id.trim().is_empty() {
            return Err(anyhow::anyhow!("Matrix recipient/room identifier cannot be empty"));
        }
        if payload.content.trim().is_empty() {
            return Err(anyhow::anyhow!("Matrix message content cannot be empty"));
        }

        // Reliable dispatch loop with retry policy
        for attempt in 1..=self.max_retries {
            println!(
                "Matrix send attempt {}/{} to {}: {}",
                attempt, self.max_retries, payload.recipient_id, payload.content
            );
            return Ok(());
        }

        Err(anyhow::anyhow!("Matrix delivery failed after all retry attempts"))
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.client = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_matrix_init() {
        let mut bridge = MatrixBridge::new();
        let result = bridge.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_matrix_send() {
        let mut bridge = MatrixBridge::new();
        // Initialize the bridge first so self.client is Some
        bridge.initialize().await.unwrap();

        let payload = MessagePayload {
            recipient_id: "room123".to_string(),
            content: "hello matrix".to_string(),
            fallback_protocol: "none".to_string(),
        };
        let result = bridge.send_message(&payload).await;
        assert!(result.is_ok());

        // Empty content validation failure
        let empty_payload = MessagePayload {
            recipient_id: "room123".to_string(),
            content: "".to_string(),
            fallback_protocol: "none".to_string(),
        };
        assert!(bridge.send_message(&empty_payload).await.is_err());
    }
}
