use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

pub struct SimplexBridge {
    broker_url: String,
    connected: bool,
}

impl SimplexBridge {
    pub fn new() -> Self {
        Self {
            broker_url: "smp://simplex.broker.local:5223".to_string(),
            connected: false,
        }
    }

    pub fn with_broker(broker_url: &str) -> Self {
        Self {
            broker_url: broker_url.to_string(),
            connected: false,
        }
    }
}

#[async_trait]
impl ProtocolBridge for SimplexBridge {
    async fn initialize(&mut self) -> Result<()> {
        println!("Initializing SimpleX Chat queue bridge via broker {}", self.broker_url);
        self.connected = true;
        Ok(())
    }

    async fn send_message(&self, payload: &MessagePayload) -> Result<()> {
        if !self.connected {
            return Err(anyhow!("SimpleX bridge not connected to broker"));
        }
        println!(
            "Mock SimpleX dispatch to queue {}: {}",
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
    async fn test_simplex_init_and_send() {
        let mut bridge = SimplexBridge::new();
        assert!(bridge.initialize().await.is_ok());

        let payload = MessagePayload {
            recipient_id: "smp_queue_xyz".to_string(),
            content: "Hello over SimpleX!".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        assert!(bridge.send_message(&payload).await.is_ok());
        assert!(bridge.disconnect().await.is_ok());
        assert!(bridge.send_message(&payload).await.is_err());
    }
}
