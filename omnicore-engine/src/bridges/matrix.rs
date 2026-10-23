use async_trait::async_trait;
use anyhow::Result;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

use matrix_sdk::{Client};

pub struct MatrixBridge {
    client: Option<Client>,
}

impl MatrixBridge {
    pub fn new() -> Self {
        Self { client: None }
    }
}

#[async_trait]
impl ProtocolBridge for MatrixBridge {
    async fn initialize(&mut self) -> Result<()> {
        // Proof-of-concept initialization targeting a mock matrix homeserver.
        // In a full implementation, we would load an authenticated session here.
        let client = Client::builder()
            .homeserver_url("https://matrix.org")
            .build()
            .await?;

        self.client = Some(client);
        Ok(())
    }

    async fn send_message(&self, payload: &MessagePayload) -> Result<()> {
        let _client = self.client.as_ref().ok_or_else(|| anyhow::anyhow!("Matrix client not initialized"))?;

        // Mock routing logic.
        // If authenticated, we would do:
        // let room_id = <&RoomId>::try_from(payload.recipient_id.as_str())?;
        // let room = client.get_room(room_id).unwrap();
        // room.send(RoomMessageEventContent::text_plain(&payload.content)).await?;

        println!("Mock Matrix send to {}: {}", payload.recipient_id, payload.content);
        Ok(())
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
    }
}
