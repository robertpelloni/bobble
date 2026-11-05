use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

pub struct ToxBridge {
    bootstrap_node: String,
    dht_connected: bool,
}

impl ToxBridge {
    pub fn new() -> Self {
        Self {
            bootstrap_node: "nodes.tox.chat:33445".to_string(),
            dht_connected: false,
        }
    }

    pub fn with_bootstrap_node(node: &str) -> Self {
        Self {
            bootstrap_node: node.to_string(),
            dht_connected: false,
        }
    }
}

#[async_trait]
impl ProtocolBridge for ToxBridge {
    async fn initialize(&mut self) -> Result<()> {
        println!("Connecting to Tox DHT bootstrap node {}", self.bootstrap_node);
        self.dht_connected = true;
        Ok(())
    }

    async fn send_message(&self, payload: &MessagePayload) -> Result<()> {
        if !self.dht_connected {
            return Err(anyhow!("Tox bridge DHT not connected"));
        }
        println!(
            "Mock Tox P2P dispatch to {}: {}",
            payload.recipient_id, payload.content
        );
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.dht_connected = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tox_init_and_send() {
        let mut bridge = ToxBridge::new();
        assert!(bridge.initialize().await.is_ok());

        let payload = MessagePayload {
            recipient_id: "56A1AB62751249313A2C59A370E7F0FC4F47AD26B830B9A1A5D21AC3E9A390382324DC7642D0".to_string(),
            content: "P2P Tox test message".to_string(),
            fallback_protocol: "Signal".to_string(),
        };

        assert!(bridge.send_message(&payload).await.is_ok());
        assert!(bridge.disconnect().await.is_ok());
        assert!(bridge.send_message(&payload).await.is_err());
    }
}
