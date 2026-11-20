use anyhow::{anyhow, Result};
use async_trait::async_trait;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;

/// Protocol bridge for Nostr (Notes and Other Stuff Transmitted by Relays - NIP-01, NIP-04).
///
/// HONESTY NOTE (v1.20.2): this bridge never opened a WebSocket to any
/// relay, never signed an event, and its "public key" was a hardcoded
/// synthetic string — publishing was a `println!`. A production bridge
/// (`new` / `with_relays`) therefore fails at `initialize` instead of
/// claiming a relay-pool connection. The simulated behaviour exists
/// only behind `new_mock()`, for tests.
pub struct NostrBridge {
    relays: Vec<String>,
    connected: bool,
    public_key_hex: Option<String>,
    mock: bool,
}

impl NostrBridge {
    pub fn new() -> Self {
        Self::build(Self::default_relays(), false)
    }

    pub fn with_relays(relays: Vec<String>) -> Self {
        Self::build(relays, false)
    }

    /// Explicit mock transport: initialization always "connects" and
    /// sends always "succeed" by printing. For tests/simulations only.
    pub fn new_mock() -> Self {
        Self::build(Self::default_relays(), true)
    }

    fn default_relays() -> Vec<String> {
        vec![
            "wss://relay.damus.io".to_string(),
            "wss://nos.lol".to_string(),
            "wss://relay.nostr.band".to_string(),
        ]
    }

    fn build(relays: Vec<String>, mock: bool) -> Self {
        Self {
            relays,
            connected: false,
            public_key_hex: None,
            mock,
        }
    }

    pub fn is_mock(&self) -> bool {
        self.mock
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn relay_count(&self) -> usize {
        self.relays.len()
    }
}

impl Default for NostrBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolBridge for NostrBridge {
    async fn initialize(&mut self) -> Result<()> {
        if !self.mock {
            return Err(anyhow!(
                "Nostr live transport not implemented: bridge is a mock skeleton and never \
                 connected to any of its {} configured relays (mock transport: NostrBridge::new_mock())",
                self.relays.len()
            ));
        }
        println!("Initializing mock Nostr bridge across {} relays", self.relays.len());
        // Synthetic 32-byte Schnorr public key (mock only — not a real identity)
        self.public_key_hex = Some("3bf0c63fcb93463407af97b5e0928875e804d02e62572836f09f45ea042b8782".to_string());
        self.connected = true;
        Ok(())
    }

    async fn send_message(&self, payload: &MessagePayload) -> Result<()> {
        if !self.connected {
            return Err(anyhow!("Nostr bridge not connected to relay pool"));
        }

        if payload.recipient_id.trim().is_empty() {
            return Err(anyhow!("Nostr recipient public key / npub cannot be empty"));
        }

        if payload.content.trim().is_empty() {
            return Err(anyhow!("Nostr event message content cannot be empty"));
        }

        println!(
            "Mock publishing of Nostr NIP-04 encrypted direct message event to npub/hex {}: {}",
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
    async fn test_nostr_production_init_fails_honestly() {
        let mut bridge = NostrBridge::new();
        assert!(bridge.initialize().await.is_err());
        assert!(!bridge.is_connected());
    }

    #[tokio::test]
    async fn test_nostr_mock_bridge_lifecycle() {
        let mut bridge = NostrBridge::new_mock();
        assert!(bridge.is_mock());
        assert!(!bridge.is_connected());
        assert_eq!(bridge.relay_count(), 3);

        bridge.initialize().await.unwrap();
        assert!(bridge.is_connected());

        let payload = MessagePayload {
            recipient_id: "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6".to_string(),
            content: "Encrypted Nostr DM via relay pool".to_string(),
            fallback_protocol: "none".to_string(),
        };

        assert!(bridge.send_message(&payload).await.is_ok());

        bridge.disconnect().await.unwrap();
        assert!(!bridge.is_connected());
        assert!(bridge.send_message(&payload).await.is_err());
    }

    #[tokio::test]
    async fn test_nostr_validation_rules() {
        let mut bridge = NostrBridge::new_mock();
        bridge.initialize().await.unwrap();

        let empty_recipient = MessagePayload {
            recipient_id: "".to_string(),
            content: "valid content".to_string(),
            fallback_protocol: "none".to_string(),
        };
        assert!(bridge.send_message(&empty_recipient).await.is_err());

        let empty_content = MessagePayload {
            recipient_id: "npub1xyz".to_string(),
            content: "   ".to_string(),
            fallback_protocol: "none".to_string(),
        };
        assert!(bridge.send_message(&empty_content).await.is_err());
    }
}
