use async_trait::async_trait;
use anyhow::Result;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

pub struct MatrixBridge;

impl MatrixBridge {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProtocolBridge for MatrixBridge {
    async fn initialize(&mut self) -> Result<()> {
        // TODO: Implement Matrix Rust SDK initialization
        Ok(())
    }

    async fn send_message(&self, _payload: &MessagePayload) -> Result<()> {
        // TODO: Implement Matrix API message dispatch
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        // TODO: Implement Matrix SDK disconnect
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
        let bridge = MatrixBridge::new();
        let payload = MessagePayload {
            recipient_id: "room123".to_string(),
            content: "hello matrix".to_string(),
            fallback_protocol: "none".to_string(),
        };
        let result = bridge.send_message(&payload).await;
        assert!(result.is_ok());
    }
}
