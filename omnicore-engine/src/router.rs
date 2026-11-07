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

    pub fn active_bridge_count(&self) -> usize {
        self.bridges.len()
    }

    pub fn available_protocols(&self) -> Vec<ProtocolKind> {
        self.bridges.keys().copied().collect()
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

    #[tokio::test]
    async fn test_multipath_router_multi_tier_cascade_to_tertiary() {
        let mut registry = BridgeRegistry::new();

        // 1. Primary: Uninitialized Signal bridge (fails)
        let signal = SignalBridge::new();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        // 2. Secondary fallback: Uninitialized Matrix bridge (fails)
        let matrix = MatrixBridge::new();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        // 3. Tertiary fallback: Initialized SMS bridge (succeeds)
        let mut sms = SmsBridge::new();
        let _ = <SmsBridge as ProtocolBridge>::initialize(&mut sms).await;
        registry.register(ProtocolKind::Sms, Box::new(sms));

        assert_eq!(registry.active_bridge_count(), 3);
        let protocols = registry.available_protocols();
        assert!(protocols.contains(&ProtocolKind::Signal));
        assert!(protocols.contains(&ProtocolKind::Matrix));
        assert!(protocols.contains(&ProtocolKind::Sms));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "+15550299".to_string(),
            content: "Testing 3-tier cascade".to_string(),
            fallback_protocol: "SMS".to_string(),
        };

        let (chosen, state) = router
            .route_with_cascade(
                &payload,
                ProtocolKind::Signal,
                &[ProtocolKind::Matrix, ProtocolKind::Sms],
            )
            .await
            .expect("Cascade to tertiary fallback should succeed");

        assert_eq!(chosen, ProtocolKind::Sms);
        match state {
            DeliveryState::FallbackTriggered { from, to, .. } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Sms);
            }
            _ => panic!("Expected FallbackTriggered delivery state"),
        }
    }

    #[tokio::test]
    async fn test_multipath_router_complete_exhaustion_failure() {
        let mut registry = BridgeRegistry::new();

        // All bridges uninitialized -> all will fail send
        let signal = SignalBridge::new();
        registry.register(ProtocolKind::Signal, Box::new(signal));
        let matrix = MatrixBridge::new();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "+15559999".to_string(),
            content: "Testing candidate exhaustion".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        let result = router
            .route_with_cascade(&payload, ProtocolKind::Signal, &[ProtocolKind::Matrix])
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("all candidate protocols exhausted"));
    }

    #[tokio::test]
    async fn test_multipath_router_p2p_tox_to_cellular_sms_fallback() {
        use crate::bridges::tox::ToxBridge;

        let mut registry = BridgeRegistry::new();
        // 1. Primary: Uninitialized Tox DHT bridge (fails)
        let tox = ToxBridge::new();
        registry.register(ProtocolKind::Tox, Box::new(tox));

        // 2. Fallback: Initialized Cellular SMS bridge (succeeds)
        let mut sms = SmsBridge::new();
        let _ = <SmsBridge as ProtocolBridge>::initialize(&mut sms).await;
        registry.register(ProtocolKind::Sms, Box::new(sms));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "+15550789".to_string(),
            content: "Off-grid P2P failover to cellular SMS".to_string(),
            fallback_protocol: "SMS".to_string(),
        };

        let (chosen, state) = router
            .route_with_cascade(&payload, ProtocolKind::Tox, &[ProtocolKind::Sms])
            .await
            .expect("Tox to SMS fallback should succeed");

        assert_eq!(chosen, ProtocolKind::Sms);
        match state {
            DeliveryState::FallbackTriggered { from, to, .. } => {
                assert_eq!(from, ProtocolKind::Tox);
                assert_eq!(to, ProtocolKind::Sms);
            }
            _ => panic!("Expected FallbackTriggered delivery state"),
        }
    }

    #[tokio::test]
    async fn test_multipath_router_simplex_queue_to_matrix_fallback() {
        use crate::bridges::simplex::SimplexBridge;

        let mut registry = BridgeRegistry::new();
        // 1. Primary: Uninitialized SimpleX queue bridge (fails)
        let simplex = SimplexBridge::new();
        registry.register(ProtocolKind::Simplex, Box::new(simplex));

        // 2. Fallback: Initialized Matrix bridge (succeeds)
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "!matrix_room_xyz:matrix.org".to_string(),
            content: "SimpleX queue failover to Matrix homeserver".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        let (chosen, state) = router
            .route_with_cascade(&payload, ProtocolKind::Simplex, &[ProtocolKind::Matrix])
            .await
            .expect("SimpleX to Matrix fallback should succeed");

        assert_eq!(chosen, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { from, to, .. } => {
                assert_eq!(from, ProtocolKind::Simplex);
                assert_eq!(to, ProtocolKind::Matrix);
            }
            _ => panic!("Expected FallbackTriggered delivery state"),
        }
    }
}
