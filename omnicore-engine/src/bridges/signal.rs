use async_trait::async_trait;
use anyhow::Result;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

pub struct SignalBridge;

impl SignalBridge {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProtocolBridge for SignalBridge {
    async fn initialize(&mut self) -> Result<()> {
        // TODO: Implement IPC connection to local signald
        Ok(())
    }

    async fn send_message(&self, _payload: &MessagePayload) -> Result<()> {
        // TODO: Implement signald message dispatch
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        // TODO: Implement signald disconnect
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_signal_init() {
        let mut bridge = SignalBridge::new();
        let result = bridge.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_signal_send() {
        let bridge = SignalBridge::new();
        let payload = MessagePayload {
            recipient_id: "+1234567890".to_string(),
            content: "hello signal".to_string(),
            fallback_protocol: "none".to_string(),
        };
        let result = bridge.send_message(&payload).await;
        assert!(result.is_ok());
    }
}
