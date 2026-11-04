use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use tokio::sync::RwLock;

use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;
use crate::models::{DeliveryState, ProtocolKind};

pub struct BridgeRegistry {
    bridges: HashMap<ProtocolKind, Arc<RwLock<Box<dyn ProtocolBridge>>>>,
}

impl BridgeRegistry {
    pub fn new() -> Self {
        Self {
            bridges: HashMap::new(),
        }
    }

    pub fn register(&mut self, protocol: ProtocolKind, bridge: Box<dyn ProtocolBridge>) {
        self.bridges.insert(protocol, Arc::new(RwLock::new(bridge)));
    }

    pub fn get(&self, protocol: ProtocolKind) -> Option<Arc<RwLock<Box<dyn ProtocolBridge>>>> {
        self.bridges.get(&protocol).cloned()
    }
}

pub struct MultipathRouter {
    registry: Arc<RwLock<BridgeRegistry>>,
}

impl MultipathRouter {
    pub fn new(registry: Arc<RwLock<BridgeRegistry>>) -> Self {
        Self { registry }
    }

    /// Attempts transmission on primary protocol; on failure, sequentially attempts fallback protocols.
    pub async fn route_with_cascade(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
    ) -> Result<(ProtocolKind, DeliveryState)> {
        let registry_guard = self.registry.read().await;

        // 1. Try Primary Protocol
        if let Some(bridge_ref) = registry_guard.get(primary) {
            let bridge = bridge_ref.read().await;
            match bridge.send_message(payload).await {
                Ok(_) => return Ok((primary, DeliveryState::Delivered)),
                Err(e) => {
                    eprintln!(
                        "Primary bridge [{}] dispatch failed: {}. Initiating cascade...",
                        primary.as_str(),
                        e
                    );
                }
            }
        }

        // 2. Cascade across Fallback Protocols sequentially
        for &fallback in fallbacks {
            if let Some(bridge_ref) = registry_guard.get(fallback) {
                let bridge = bridge_ref.read().await;
                match bridge.send_message(payload).await {
                    Ok(_) => {
                        return Ok((
                            fallback,
                            DeliveryState::FallbackTriggered {
                                from: primary,
                                to: fallback,
                                reason: "Primary bridge unavailable or failed".to_string(),
                            },
                        ));
                    }
                    Err(e) => {
                        eprintln!("Fallback bridge [{}] failed: {}. Continuing cascade...", fallback.as_str(), e);
                    }
                }
            }
        }

        Err(anyhow!(
            "Multipath routing failed: all candidate protocols exhausted for recipient {}",
            payload.recipient_id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridges::signal::SignalBridge;
    use crate::bridges::matrix::MatrixBridge;
    use crate::bridges::sms::SmsBridge;

    #[tokio::test]
    async fn test_multipath_router_primary_delivery() {
        let mut registry = BridgeRegistry::new();

        let mut signal = SignalBridge::new();
        signal.initialize().await.unwrap();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "+15550100".to_string(),
            content: "Multipath primary test".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        let (chosen, state) = router
            .route_with_cascade(&payload, ProtocolKind::Signal, &[ProtocolKind::Matrix])
            .await
            .expect("Primary route should succeed");

        assert_eq!(chosen, ProtocolKind::Signal);
        assert_eq!(state, DeliveryState::Delivered);
    }

    #[tokio::test]
    async fn test_multipath_router_cascade_fallback() {
        let mut registry = BridgeRegistry::new();

        // Register disconnected Signal bridge (will fail)
        let signal = SignalBridge::new(); // Not initialized, will fail send
        registry.register(ProtocolKind::Signal, Box::new(signal));

        // Register initialized SMS bridge (will succeed)
        let mut sms = SmsBridge::new();
        let _ = <SmsBridge as ProtocolBridge>::initialize(&mut sms).await;
        registry.register(ProtocolKind::Sms, Box::new(sms));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "+15550199".to_string(),
            content: "Fallback cascade test".to_string(),
            fallback_protocol: "SMS".to_string(),
        };

        let (chosen, state) = router
            .route_with_cascade(&payload, ProtocolKind::Signal, &[ProtocolKind::Sms])
            .await
            .expect("Fallback route should succeed");

        assert_eq!(chosen, ProtocolKind::Sms);
        match state {
            DeliveryState::FallbackTriggered { from, to, .. } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Sms);
            }
            _ => panic!("Expected FallbackTriggered delivery state"),
        }
    }
}
