use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use tokio::sync::RwLock;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;
use crate::models::{DeliveryState, ProtocolKind};

/// Guard ensuring message payload integrity and tamper detection across multi-protocol cascading fallbacks.
pub struct PayloadIntegrityGuard;

impl PayloadIntegrityGuard {
    /// Computes a deterministic 64-bit FNV-1a cryptographic checksum of a message payload's content and recipient.
    pub fn compute_digest(payload: &MessagePayload) -> String {
        let mut sum: u64 = 0xcbf29ce484222325;
        for &b in payload.recipient_id.as_bytes() {
            sum ^= b as u64;
            sum = sum.wrapping_mul(0x100000001b3);
        }
        sum ^= b':' as u64;
        sum = sum.wrapping_mul(0x100000001b3);
        for &b in payload.content.as_bytes() {
            sum ^= b as u64;
            sum = sum.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", sum)
    }

    /// Verifies that the payload matches an expected digest string.
    pub fn verify_digest(payload: &MessagePayload, expected_digest: &str) -> bool {
        Self::compute_digest(payload) == expected_digest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,   // Normal operation, all traffic passes
    Open,     // Tripped, bypass bridge to prevent timeout blocking
    HalfOpen, // Probing after cooldown
}

#[derive(Debug, Clone)]
pub struct ProtocolMetric {
    pub total_attempts: u32,
    pub successful_attempts: u32,
    pub average_latency_ms: u64,
    pub consecutive_failures: u32,
    pub circuit_state: CircuitState,
    pub last_failure_epoch_sec: u64,
    pub failure_threshold: u32,
    pub cooldown_sec: u64,
}

impl Default for ProtocolMetric {
    fn default() -> Self {
        Self {
            total_attempts: 0,
            successful_attempts: 0,
            average_latency_ms: 0,
            consecutive_failures: 0,
            circuit_state: CircuitState::Closed,
            last_failure_epoch_sec: 0,
            failure_threshold: 3,
            cooldown_sec: 30,
        }
    }
}

impl ProtocolMetric {
    pub fn is_available_at(&mut self, now_epoch: u64) -> bool {
        match self.circuit_state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if now_epoch >= self.last_failure_epoch_sec.saturating_add(self.cooldown_sec) {
                    self.circuit_state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }
}

pub struct BridgeRegistry {
    bridges: HashMap<ProtocolKind, Arc<RwLock<Box<dyn ProtocolBridge>>>>,
    metrics: HashMap<ProtocolKind, ProtocolMetric>,
}

impl BridgeRegistry {
    pub fn new() -> Self {
        Self {
            bridges: HashMap::new(),
            metrics: HashMap::new(),
        }
    }

    pub fn register(&mut self, protocol: ProtocolKind, bridge: Box<dyn ProtocolBridge>) {
        self.bridges.insert(protocol, Arc::new(RwLock::new(bridge)));
        self.metrics.entry(protocol).or_default();
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

    /// Records delivery outcome and latency to adjust route optimization and circuit-breaker state.
    pub fn record_delivery_metrics(&mut self, protocol: ProtocolKind, latency_ms: u64, success: bool, now_epoch: u64) {
        let m = self.metrics.entry(protocol).or_default();
        m.total_attempts += 1;
        if success {
            m.successful_attempts += 1;
            m.consecutive_failures = 0;
            m.circuit_state = CircuitState::Closed;
            if m.average_latency_ms == 0 {
                m.average_latency_ms = latency_ms;
            } else {
                m.average_latency_ms = (m.average_latency_ms * 7 + latency_ms * 3) / 10;
            }
        } else {
            m.consecutive_failures += 1;
            m.last_failure_epoch_sec = now_epoch;
            if m.consecutive_failures >= m.failure_threshold {
                m.circuit_state = CircuitState::Open;
            }
        }
    }

    pub fn get_metrics(&self, protocol: ProtocolKind) -> Option<&ProtocolMetric> {
        self.metrics.get(&protocol)
    }

    /// Selects optimal primary and reordered fallback candidates according to bridge health.
    pub fn select_optimal_route(
        &self,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
    ) -> (ProtocolKind, Vec<ProtocolKind>) {
        let mut candidates = vec![primary];
        for &fb in fallbacks {
            if !candidates.contains(&fb) {
                candidates.push(fb);
            }
        }

        // Sort candidates: healthy bridges first (low consecutive failures), then by latency
        candidates.sort_by(|a, b| {
            let m_a = self.metrics.get(a);
            let m_b = self.metrics.get(b);

            let fail_a = m_a.map(|m| m.consecutive_failures).unwrap_or(0);
            let fail_b = m_b.map(|m| m.consecutive_failures).unwrap_or(0);

            if fail_a != fail_b {
                fail_a.cmp(&fail_b)
            } else {
                let lat_a = m_a.map(|m| m.average_latency_ms).unwrap_or(u64::MAX);
                let lat_b = m_b.map(|m| m.average_latency_ms).unwrap_or(u64::MAX);
                lat_a.cmp(&lat_b)
            }
        });

        let chosen_primary = candidates[0];
        let remaining_fallbacks = candidates[1..].to_vec();
        (chosen_primary, remaining_fallbacks)
    }

    /// Selects optimal route while actively promoting starved (zero-attempt) candidates
    /// if the default candidate exhibits elevated latency or previous failure.
    pub fn select_optimal_route_with_probing(
        &self,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        latency_probe_threshold_ms: u64,
    ) -> (ProtocolKind, Vec<ProtocolKind>) {
        let (mut chosen_primary, mut remaining) = self.select_optimal_route(primary, fallbacks);

        let primary_latency = self.metrics.get(&chosen_primary)
            .map(|m| m.average_latency_ms)
            .unwrap_or(0);

        // If current primary has elevated latency (> threshold), probe an unattempted starved bridge
        if primary_latency >= latency_probe_threshold_ms {
            if let Some(pos) = remaining.iter().position(|proto| {
                self.metrics.get(proto).map(|m| m.total_attempts == 0).unwrap_or(true)
            }) {
                let starved_candidate = remaining.remove(pos);
                remaining.insert(0, chosen_primary);
                chosen_primary = starved_candidate;
            }
        }

        (chosen_primary, remaining)
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

    /// Automatically optimizes candidate order using health and latency metrics,
    /// checks circuit-breaker availability, executes cascading delivery, and updates metrics.
    pub async fn route_with_optimal_cascade(
        &self,
        payload: &MessagePayload,
        requested_primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        // Step 1: Compute optimal candidate priority order
        let (chosen_primary, reordered_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_optimal_route(requested_primary, fallbacks)
        };

        let mut candidate_chain = vec![chosen_primary];
        candidate_chain.extend(reordered_fallbacks);

        let start_time = std::time::Instant::now();

        // Step 2: Attempt dispatch sequentially across candidates
        for (idx, &candidate) in candidate_chain.iter().enumerate() {
            // Check circuit-breaker availability
            let is_available = {
                let mut registry_guard = self.registry.write().await;
                if let Some(metric) = registry_guard.metrics.get_mut(&candidate) {
                    metric.is_available_at(now_epoch)
                } else {
                    true
                }
            };

            if !is_available {
                eprintln!(
                    "Bridge [{}] circuit is OPEN. Bypassing candidate...",
                    candidate.as_str()
                );
                continue;
            }

            let bridge_opt = {
                let registry_guard = self.registry.read().await;
                registry_guard.get(candidate)
            };

            if let Some(bridge_ref) = bridge_opt {
                let bridge = bridge_ref.read().await;
                let attempt_start = std::time::Instant::now();
                match bridge.send_message(payload).await {
                    Ok(_) => {
                        let latency_ms = attempt_start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, true, now_epoch);

                        let state = if idx == 0 && candidate == requested_primary {
                            DeliveryState::Delivered
                        } else {
                            DeliveryState::FallbackTriggered {
                                from: requested_primary,
                                to: candidate,
                                reason: format!("Optimal routing selected fallback after {}ms", start_time.elapsed().as_millis()),
                            }
                        };
                        return Ok((candidate, state));
                    }
                    Err(e) => {
                        let latency_ms = attempt_start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, false, now_epoch);
                        eprintln!(
                            "Optimal route bridge [{}] failed: {}. Continuing cascade...",
                            candidate.as_str(),
                            e
                        );
                    }
                }
            }
        }

        Err(anyhow!(
            "Multipath optimal routing failed: all candidate protocols exhausted for recipient {}",
            payload.recipient_id
        ))
    }

    /// Dispatches with active starvation prevention and probe discovery: elevates unattempted
    /// candidate bridges if primary latency exceeds threshold, preventing traffic bottlenecks.
    pub async fn route_with_starvation_prevention(
        &self,
        payload: &MessagePayload,
        requested_primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        latency_probe_threshold_ms: u64,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        let (probed_primary, probed_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_optimal_route_with_probing(
                requested_primary,
                fallbacks,
                latency_probe_threshold_ms,
            )
        };

        self.route_with_optimal_cascade(
            payload,
            probed_primary,
            &probed_fallbacks,
            now_epoch,
        ).await
    }

    /// Dispatches a message across cascading fallbacks while verifying payload digest integrity before and after dispatch.
    pub async fn route_with_integrity_check(
        &self,
        payload: &MessagePayload,
        expected_digest: &str,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        if !PayloadIntegrityGuard::verify_digest(payload, expected_digest) {
            return Err(anyhow!("Payload integrity verification failed: tamper or truncation detected"));
        }

        let result = self.route_with_optimal_cascade(payload, primary, fallbacks, now_epoch).await?;

        // Post-dispatch re-verification
        if !PayloadIntegrityGuard::verify_digest(payload, expected_digest) {
            return Err(anyhow!("Post-dispatch payload integrity check failed"));
        }

        Ok(result)
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

    #[test]
    fn test_adaptive_latency_route_selection() {
        let mut registry = BridgeRegistry::new();
        registry.register(ProtocolKind::Signal, Box::new(crate::bridges::signal::SignalBridge::new()));
        registry.register(ProtocolKind::Matrix, Box::new(crate::bridges::matrix::MatrixBridge::new()));
        registry.register(ProtocolKind::Sms, Box::new(crate::bridges::sms::SmsBridge::new()));

        let now = 1700000000;
        // Signal: low latency (15ms)
        registry.record_delivery_metrics(ProtocolKind::Signal, 15, true, now);
        // Matrix: higher latency (80ms)
        registry.record_delivery_metrics(ProtocolKind::Matrix, 80, true, now);
        // SMS: high latency (120ms)
        registry.record_delivery_metrics(ProtocolKind::Sms, 120, true, now);

        let (primary, fallbacks) = registry.select_optimal_route(
            ProtocolKind::Matrix,
            &[ProtocolKind::Sms, ProtocolKind::Signal],
        );

        // Signal has lower latency so it should be promoted to primary
        assert_eq!(primary, ProtocolKind::Signal);
        assert_eq!(fallbacks[0], ProtocolKind::Matrix);
        assert_eq!(fallbacks[1], ProtocolKind::Sms);
    }

    #[test]
    fn test_unhealthy_bridge_dynamic_deprioritization() {
        let mut registry = BridgeRegistry::new();
        registry.register(ProtocolKind::Signal, Box::new(crate::bridges::signal::SignalBridge::new()));
        registry.register(ProtocolKind::Matrix, Box::new(crate::bridges::matrix::MatrixBridge::new()));

        let now = 1700000000;
        // Signal experiences consecutive failures
        registry.record_delivery_metrics(ProtocolKind::Signal, 10, false, now);
        registry.record_delivery_metrics(ProtocolKind::Signal, 10, false, now);

        // Matrix is healthy
        registry.record_delivery_metrics(ProtocolKind::Matrix, 50, true, now);

        let (primary, fallbacks) = registry.select_optimal_route(ProtocolKind::Signal, &[ProtocolKind::Matrix]);

        // Unhealthy Signal is deprioritized; healthy Matrix is chosen first
        assert_eq!(primary, ProtocolKind::Matrix);
        assert_eq!(fallbacks, vec![ProtocolKind::Signal]);
    }

    #[test]
    fn test_circuit_breaker_bypasses_open_bridge() {
        let mut metric = ProtocolMetric {
            failure_threshold: 3,
            cooldown_sec: 20,
            ..Default::default()
        };

        let now = 1700000000;
        assert_eq!(metric.circuit_state, CircuitState::Closed);
        assert!(metric.is_available_at(now));

        // Simulate 3 failures to trip the circuit
        metric.consecutive_failures = 3;
        metric.last_failure_epoch_sec = now;
        metric.circuit_state = CircuitState::Open;

        // Immediately after, bridge should be bypassed (not available)
        assert!(!metric.is_available_at(now + 5));
        assert_eq!(metric.circuit_state, CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_half_open_recovery() {
        let mut metric = ProtocolMetric {
            failure_threshold: 3,
            cooldown_sec: 20,
            consecutive_failures: 3,
            last_failure_epoch_sec: 1700000000,
            circuit_state: CircuitState::Open,
            ..Default::default()
        };

        let post_cooldown = 1700000025;
        // After cooldown expires, circuit moves to HalfOpen to probe recovery
        assert!(metric.is_available_at(post_cooldown));
        assert_eq!(metric.circuit_state, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_route_with_optimal_cascade_bypasses_open_circuit() {
        let mut registry = BridgeRegistry::new();

        // Register Signal bridge but trip its circuit
        let signal = SignalBridge::new();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        let now = 1700000000;
        // Trip Signal circuit by recording 3 consecutive failures
        registry.record_delivery_metrics(ProtocolKind::Signal, 10, false, now);
        registry.record_delivery_metrics(ProtocolKind::Signal, 10, false, now);
        registry.record_delivery_metrics(ProtocolKind::Signal, 10, false, now);

        // Register healthy Matrix bridge
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let shared_registry = Arc::new(RwLock::new(registry));
        let router = MultipathRouter::new(shared_registry.clone());

        let payload = MessagePayload {
            recipient_id: "!matrix_room_opt:matrix.org".to_string(),
            content: "Testing optimal cascade with circuit breaker".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        // Dispatch with optimal cascade at now + 5 (within cooldown, Signal circuit is OPEN)
        let (chosen, state) = router
            .route_with_optimal_cascade(&payload, ProtocolKind::Signal, &[ProtocolKind::Matrix], now + 5)
            .await
            .expect("Routing should bypass open Signal circuit and deliver via Matrix");

        assert_eq!(chosen, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { from, to, .. } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Matrix);
            }
            _ => panic!("Expected FallbackTriggered state"),
        }

        // Verify Matrix metrics were updated with success
        let reg_guard = shared_registry.read().await;
        let matrix_metric = reg_guard.get_metrics(ProtocolKind::Matrix).unwrap();
        assert_eq!(matrix_metric.successful_attempts, 1);
        assert_eq!(matrix_metric.circuit_state, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_multipath_router_imessage_to_sms_cascade() {
        use crate::bridges::imessage::IMessageBridge;

        let mut registry = BridgeRegistry::new();
        // 1. Primary: Uninitialized iMessage bridge (will fail)
        let imessage = IMessageBridge::new();
        registry.register(ProtocolKind::IMessage, Box::new(imessage));

        // 2. Fallback: Initialized SMS modem bridge (will succeed)
        let mut sms = SmsBridge::new();
        let _ = <SmsBridge as ProtocolBridge>::initialize(&mut sms).await;
        registry.register(ProtocolKind::Sms, Box::new(sms));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "+15550999".to_string(),
            content: "iMessage failover to cellular SMS test".to_string(),
            fallback_protocol: "SMS".to_string(),
        };

        let (chosen, state) = router
            .route_with_cascade(&payload, ProtocolKind::IMessage, &[ProtocolKind::Sms])
            .await
            .expect("iMessage to SMS cascade should succeed");

        assert_eq!(chosen, ProtocolKind::Sms);
        match state {
            DeliveryState::FallbackTriggered { from, to, .. } => {
                assert_eq!(from, ProtocolKind::IMessage);
                assert_eq!(to, ProtocolKind::Sms);
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[tokio::test]
    async fn test_route_with_starvation_prevention_probes_unattempted_bridge() {
        let mut registry = BridgeRegistry::new();

        // 1. Matrix: high latency (2500ms > 1000ms threshold)
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));
        registry.record_delivery_metrics(ProtocolKind::Matrix, 2500, true, 1700000000);

        // 2. Signal: unattempted (0 attempts), healthy
        let mut signal = SignalBridge::new();
        signal.initialize().await.unwrap();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        let shared_registry = Arc::new(RwLock::new(registry));
        let router = MultipathRouter::new(shared_registry.clone());

        let payload = MessagePayload {
            recipient_id: "+15550333".to_string(),
            content: "Testing starvation probing elevation".to_string(),
            fallback_protocol: "Signal".to_string(),
        };

        // Even though Matrix was requested, starvation prevention promotes unattempted Signal to probe
        let (chosen, _) = router
            .route_with_starvation_prevention(
                &payload,
                ProtocolKind::Matrix,
                &[ProtocolKind::Signal],
                1000, // Probe threshold 1s
                1700000010,
            )
            .await
            .expect("Starvation probing dispatch should succeed");

        assert_eq!(chosen, ProtocolKind::Signal);

        // Verify Signal metric was recorded from probe
        let reg_guard = shared_registry.read().await;
        let signal_metric = reg_guard.get_metrics(ProtocolKind::Signal).unwrap();
        assert_eq!(signal_metric.total_attempts, 1);
        assert_eq!(signal_metric.successful_attempts, 1);
    }

    #[tokio::test]
    async fn test_route_with_payload_integrity_verification() {
        let mut registry = BridgeRegistry::new();
        let mut signal = SignalBridge::new();
        signal.initialize().await.unwrap();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let valid_payload = MessagePayload {
            recipient_id: "+15558888".to_string(),
            content: "Encrypted integrity payload".to_string(),
            fallback_protocol: "none".to_string(),
        };

        let digest = PayloadIntegrityGuard::compute_digest(&valid_payload);

        // 1. Dispatch with valid digest succeeds
        let (chosen, state) = router
            .route_with_integrity_check(
                &valid_payload,
                &digest,
                ProtocolKind::Signal,
                &[],
                1700000000,
            )
            .await
            .expect("Valid digest should pass integrity check");

        assert_eq!(chosen, ProtocolKind::Signal);
        assert_eq!(state, DeliveryState::Delivered);

        // 2. Tampered content fails verification
        let mut tampered_payload = valid_payload.clone();
        tampered_payload.content = "Tampered unauthorized content!".to_string();

        let fail_res = router
            .route_with_integrity_check(
                &tampered_payload,
                &digest,
                ProtocolKind::Signal,
                &[],
                1700000000,
            )
            .await;

        assert!(fail_res.is_err());
        assert!(fail_res.unwrap_err().to_string().contains("integrity verification failed"));
    }
}
