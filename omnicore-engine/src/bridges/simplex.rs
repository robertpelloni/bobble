use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

/// SimpleX bridge.
///
/// HONESTY NOTE (v1.20.2): this bridge has no transport at all — it
/// never connected to any SimpleX broker or queue. A production bridge
/// (`new` / `with_broker`) therefore fails at `initialize` instead of
/// claiming a connection to a broker URL that was never contacted.
/// The simulated behaviour exists only behind `new_mock()`, for tests.
pub struct SimplexBridge {
    broker_url: String,
    connected: bool,
    mock: bool,
}

impl SimplexBridge {
    pub fn new() -> Self {
        Self::build("smp://simplex.broker.local:5223".to_string(), false)
    }

    pub fn with_broker(broker_url: &str) -> Self {
        Self::build(broker_url.to_string(), false)
    }

    /// Explicit mock transport: initialization always "connects" and
    /// sends always "succeed" by printing. For tests/simulations only.
    pub fn new_mock() -> Self {
        Self::build("smp://simplex.broker.local:5223".to_string(), true)
    }

    fn build(broker_url: String, mock: bool) -> Self {
        Self {
            broker_url,
            connected: false,
            mock,
        }
    }

    pub fn is_mock(&self) -> bool {
        self.mock
    }
}

#[async_trait]
impl ProtocolBridge for SimplexBridge {
    async fn initialize(&mut self) -> Result<()> {
        if !self.mock {
            return Err(anyhow!(
                "SimpleX live transport not implemented: bridge is a mock skeleton and never \
                 connected to broker {} (mock transport: SimplexBridge::new_mock())",
                self.broker_url
            ));
        }
        println!("Initializing mock SimpleX Chat queue bridge via broker {}", self.broker_url);
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
    async fn test_simplex_production_init_fails_honestly() {
        let mut bridge = SimplexBridge::new();
        assert!(bridge.initialize().await.is_err());
        let payload = MessagePayload {
            recipient_id: "smp_queue_xyz".to_string(),
            content: "Hello over SimpleX!".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };
        assert!(bridge.send_message(&payload).await.is_err());
    }

    #[tokio::test]
    async fn test_simplex_mock_init_and_send() {
        let mut bridge = SimplexBridge::new_mock();
        assert!(bridge.is_mock());
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
