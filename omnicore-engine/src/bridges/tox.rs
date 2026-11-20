use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

/// Tox bridge.
///
/// HONESTY NOTE (v1.20.2): this bridge has no transport at all — it
/// never bootstrapped into the Tox DHT or contacted any node. A
/// production bridge (`new` / `with_bootstrap_node`) therefore fails
/// at `initialize` instead of claiming a DHT connection. The simulated
/// behaviour exists only behind `new_mock()`, for tests.
pub struct ToxBridge {
    bootstrap_node: String,
    dht_connected: bool,
    mock: bool,
}

impl ToxBridge {
    pub fn new() -> Self {
        Self::build("nodes.tox.chat:33445".to_string(), false)
    }

    pub fn with_bootstrap_node(node: &str) -> Self {
        Self::build(node.to_string(), false)
    }

    /// Explicit mock transport: initialization always "connects" and
    /// sends always "succeed" by printing. For tests/simulations only.
    pub fn new_mock() -> Self {
        Self::build("nodes.tox.chat:33445".to_string(), true)
    }

    fn build(bootstrap_node: String, mock: bool) -> Self {
        Self {
            bootstrap_node,
            dht_connected: false,
            mock,
        }
    }

    pub fn is_mock(&self) -> bool {
        self.mock
    }
}

#[async_trait]
impl ProtocolBridge for ToxBridge {
    async fn initialize(&mut self) -> Result<()> {
        if !self.mock {
            return Err(anyhow!(
                "Tox live transport not implemented: bridge is a mock skeleton and never \
                 bootstrapped into the DHT via {} (mock transport: ToxBridge::new_mock())",
                self.bootstrap_node
            ));
        }
        println!("Connecting mock Tox bridge to DHT bootstrap node {}", self.bootstrap_node);
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
    async fn test_tox_production_init_fails_honestly() {
        let mut bridge = ToxBridge::new();
        assert!(bridge.initialize().await.is_err());
        let payload = MessagePayload {
            recipient_id: "56A1AB62751249313A2C59A370E7F0FC4F47AD26B830B9A1A5D21AC3E9A390382324DC7642D0".to_string(),
            content: "P2P Tox test message".to_string(),
            fallback_protocol: "Signal".to_string(),
        };
        assert!(bridge.send_message(&payload).await.is_err());
    }

    #[tokio::test]
    async fn test_tox_mock_init_and_send() {
        let mut bridge = ToxBridge::new_mock();
        assert!(bridge.is_mock());
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
