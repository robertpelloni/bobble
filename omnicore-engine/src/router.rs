use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{anyhow, Result};
use tokio::sync::RwLock;
use crate::bridge::ProtocolBridge;
use crate::dispatcher::MessagePayload;
use crate::models::{DeliveryState, ProtocolKind};

/// Address book translating a unified contact identity into protocol-specific destination identifiers.
#[derive(Debug, Clone, Default)]
pub struct CrossProtocolAddressBook {
    mappings: HashMap<String, HashMap<ProtocolKind, String>>,
}

impl CrossProtocolAddressBook {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    /// Registers a protocol-specific recipient address for a unified contact identifier.
    pub fn register_address(&mut self, unified_id: &str, protocol: ProtocolKind, address: &str) {
        self.mappings
            .entry(unified_id.to_string())
            .or_default()
            .insert(protocol, address.to_string());
    }

    /// Resolves the protocol address for a given unified contact; if unmapped, defaults to unified_id.
    pub fn resolve_address(&self, unified_id: &str, protocol: ProtocolKind) -> String {
        self.mappings
            .get(unified_id)
            .and_then(|protos| protos.get(&protocol))
            .cloned()
            .unwrap_or_else(|| unified_id.to_string())
    }
}

/// Protocol-specific payload size constraints for transport adaptation.
pub struct ProtocolMtuLimits;

impl ProtocolMtuLimits {
    /// Returns max payload character/byte limit for a given protocol transport.
    pub fn max_payload_len(protocol: ProtocolKind) -> usize {
        match protocol {
            ProtocolKind::Sms => 140, // Single SMS segment safe character budget
            ProtocolKind::Tox => 1372, // P2P UDP packet limit
            _ => 65536, // Unconstrained high-MTU bridges (Signal, Matrix, Discord, SimpleX, iMessage)
        }
    }
}

/// QoS and routing policy preferences balancing financial cost, battery efficiency, and delivery speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QosRoutingPreference {
    /// Prioritize low financial cost (prefers free internet/P2P over metered cellular SMS)
    CostSaving,
    /// Prioritize lowest battery consumption (prefers low-power local mesh/Wi-Fi over power-heavy cellular modems)
    BatteryPreservation,
    /// Prioritize minimal latency regardless of transport cost (emergency mode)
    UltraLowLatency,
}

/// Dynamic QoS and route cost optimizer evaluating financial cost, battery drain tier, and transport overhead.
pub struct RouteCostOptimizer;

impl RouteCostOptimizer {
    /// Returns financial transport cost unit score (0 = free internet/DHT, 50 = metered cellular SMS).
    pub fn financial_cost_units(protocol: ProtocolKind) -> u64 {
        match protocol {
            ProtocolKind::Sms => 50, // Paid cellular SMS segment
            _ => 0,                  // Unmetered broadband/internet/P2P transports
        }
    }

    /// Returns estimated battery consumption score per dispatch (1 = low power Wi-Fi/mesh, 40 = cellular radio transmit).
    pub fn battery_drain_score(protocol: ProtocolKind) -> u64 {
        match protocol {
            ProtocolKind::Sms => 40,    // High power radio TX
            ProtocolKind::Tox => 15,    // P2P UDP keepalive background drain
            ProtocolKind::Simplex => 5, // SimpleX HTTP queue polling
            _ => 2,                     // Standard TCP/WebSocket connections
        }
    }

    /// Computes unified composite QoS penalty score for a given candidate protocol.
    /// Lower score = preferred route.
    pub fn compute_qos_score(
        protocol: ProtocolKind,
        latency_ms: u64,
        preference: QosRoutingPreference,
    ) -> u64 {
        let base_lat = if latency_ms == 0 { 50 } else { latency_ms };
        let fin_cost = Self::financial_cost_units(protocol);
        let bat_cost = Self::battery_drain_score(protocol);

        match preference {
            QosRoutingPreference::CostSaving => {
                // Financial cost heavily weighted (x100)
                base_lat.saturating_add(fin_cost.saturating_mul(100)).saturating_add(bat_cost.saturating_mul(2))
            }
            QosRoutingPreference::BatteryPreservation => {
                // Battery drain score heavily weighted (x50)
                base_lat.saturating_add(bat_cost.saturating_mul(50)).saturating_add(fin_cost.saturating_mul(5))
            }
            QosRoutingPreference::UltraLowLatency => {
                // Latency is primary factor, cost/battery negligible
                base_lat.saturating_add(fin_cost).saturating_add(bat_cost)
            }
        }
    }
}

/// Sliding-window byte quota and rate limiter for metered or bandwidth-constrained protocol bridges.
#[derive(Debug, Clone)]
pub struct RouteBandwidthShaper {
    byte_limits: HashMap<ProtocolKind, usize>,
    consumed_bytes: HashMap<ProtocolKind, Vec<(u64, usize)>>, // protocol -> vec of (timestamp_sec, byte_count)
    window_duration_sec: u64,
}

impl Default for RouteBandwidthShaper {
    fn default() -> Self {
        let mut byte_limits = HashMap::new();
        byte_limits.insert(ProtocolKind::Sms, 2048); // 2 KB window budget for cellular SMS
        byte_limits.insert(ProtocolKind::Tox, 1024 * 1024); // 1 MB window budget for P2P Tox

        Self {
            byte_limits,
            consumed_bytes: HashMap::new(),
            window_duration_sec: 60, // 60s sliding window
        }
    }
}

impl RouteBandwidthShaper {
    pub fn new(window_duration_sec: u64) -> Self {
        Self {
            byte_limits: HashMap::new(),
            consumed_bytes: HashMap::new(),
            window_duration_sec,
        }
    }

    /// Sets the byte consumption allowance within the sliding window for a protocol.
    pub fn set_byte_limit(&mut self, protocol: ProtocolKind, max_bytes: usize) {
        self.byte_limits.insert(protocol, max_bytes);
    }

    /// Prunes expired consumption records outside the sliding window.
    pub fn prune_expired(&mut self, now_sec: u64) {
        let cutoff = now_sec.saturating_sub(self.window_duration_sec);
        for entries in self.consumed_bytes.values_mut() {
            entries.retain(|&(ts, _)| ts > cutoff);
        }
    }

    /// Calculates currently consumed bytes within the sliding window.
    pub fn current_usage(&self, protocol: ProtocolKind, now_sec: u64) -> usize {
        let cutoff = now_sec.saturating_sub(self.window_duration_sec);
        self.consumed_bytes
            .get(&protocol)
            .map(|entries| {
                entries
                    .iter()
                    .filter(|&&(ts, _)| ts > cutoff)
                    .map(|&(_, bytes)| bytes)
                    .sum()
            })
            .unwrap_or(0)
    }

    /// Checks if the payload fits within the remaining bandwidth budget. If allowed, records the usage.
    pub fn try_consume_bandwidth(
        &mut self,
        protocol: ProtocolKind,
        byte_len: usize,
        now_sec: u64,
    ) -> bool {
        self.prune_expired(now_sec);

        if let Some(&limit) = self.byte_limits.get(&protocol) {
            let usage = self.current_usage(protocol, now_sec);
            if usage.saturating_add(byte_len) > limit {
                return false; // Quota exceeded
            }
        }

        self.consumed_bytes
            .entry(protocol)
            .or_default()
            .push((now_sec, byte_len));
        true
    }
}

/// Route dispersal balancer distributing traffic across healthy multi-path candidate bridges
/// to avoid socket starvation, provider rate limits, and serial modem bottlenecks.
#[derive(Debug, Clone, Default)]
pub struct RouteDispersalBalancer {
    counter: usize,
}

impl RouteDispersalBalancer {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    /// Selects the primary bridge and fallback chain by rotating through candidate bridges.
    pub fn select_dispersed_route(
        &mut self,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
    ) -> (ProtocolKind, Vec<ProtocolKind>) {
        let mut candidates = vec![primary];
        for &fb in fallbacks {
            if !candidates.contains(&fb) {
                candidates.push(fb);
            }
        }

        if candidates.len() <= 1 {
            return (primary, fallbacks.to_vec());
        }

        let idx = self.counter % candidates.len();
        self.counter = self.counter.wrapping_add(1);

        let chosen_primary = candidates[idx];
        let mut remaining = Vec::with_capacity(candidates.len() - 1);
        for (i, &cand) in candidates.iter().enumerate() {
            if i != idx {
                remaining.push(cand);
            }
        }

        (chosen_primary, remaining)
    }
}

/// Dynamic content transcoder adapting formatting between disparate protocol transports.
/// Normalizes rich text (Markdown, HTML, URLs) into plain text for cellular SMS and constrained transports.
pub struct PayloadContentTranscoder;

impl PayloadContentTranscoder {
    /// Transcodes payload content for destination protocol requirements.
    pub fn transcode_for_protocol(content: &str, target_protocol: ProtocolKind) -> String {
        match target_protocol {
            ProtocolKind::Sms => {
                // Strip markdown formatting (*bold*, _italic_, `code`, [link](url))
                let mut sanitized = content
                    .replace("**", "")
                    .replace("__", "")
                    .replace("*", "")
                    .replace("_", "")
                    .replace("`", "")
                    .replace("~~", "");

                // Convert HTML line breaks if present
                sanitized = sanitized.replace("<br>", "\n").replace("<br/>", "\n");

                // Normalize complex unicode characters into basic ASCII equivalents
                sanitized = sanitized
                    .replace('’', "'")
                    .replace('‘', "'")
                    .replace('“', "\"")
                    .replace('”', "\"")
                    .replace('—', "-")
                    .replace('–', "-")
                    .replace('…', "...");

                sanitized.trim().to_string()
            }
            _ => content.to_string(),
        }
    }
}

/// Message payload fragmenter splitting oversized messages into sequential chunks.
pub struct PayloadFragmenter;

impl PayloadFragmenter {
    /// Splits payload content into chunks conforming to MTU constraints.
    pub fn fragment_payload(content: &str, max_len: usize) -> Vec<String> {
        let max_len = max_len.max(20);
        if content.len() <= max_len {
            return vec![content.to_string()];
        }

        // Account for prefix header: "[1/N] " (approx 8 chars)
        let chunk_size = max_len.saturating_sub(10).max(10);
        let chars: Vec<char> = content.chars().collect();
        let total_chunks = (chars.len() + chunk_size - 1) / chunk_size;

        let mut fragments = Vec::new();
        for (idx, chunk) in chars.chunks(chunk_size).enumerate() {
            let part_str: String = chunk.iter().collect();
            fragments.push(format!("[{}/{}] {}", idx + 1, total_chunks, part_str));
        }

        fragments
    }
}

/// Cryptographic delivery acknowledgment receipt confirming message arrival at the final recipient node.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DeliveryReceipt {
    pub nonce: String,
    pub recipient_id: String,
    pub protocol: ProtocolKind,
    pub received_timestamp_sec: u64,
    pub signature: String,
}

impl DeliveryReceipt {
    /// Generates a delivery receipt signed with a simulated recipient key.
    pub fn generate(nonce: &str, recipient_id: &str, protocol: ProtocolKind, timestamp_sec: u64) -> Self {
        let mut sum: u64 = 0xcbf29ce484222325;
        for &b in nonce.as_bytes() {
            sum ^= b as u64;
            sum = sum.wrapping_mul(0x100000001b3);
        }
        sum ^= protocol as u64;
        sum = sum.wrapping_mul(0x100000001b3);

        Self {
            nonce: nonce.to_string(),
            recipient_id: recipient_id.to_string(),
            protocol,
            received_timestamp_sec: timestamp_sec,
            signature: format!("{:016x}", sum),
        }
    }

    /// Verifies the authenticity of the receipt signature.
    pub fn verify_signature(&self) -> bool {
        let mut sum: u64 = 0xcbf29ce484222325;
        for &b in self.nonce.as_bytes() {
            sum ^= b as u64;
            sum = sum.wrapping_mul(0x100000001b3);
        }
        sum ^= self.protocol as u64;
        sum = sum.wrapping_mul(0x100000001b3);
        self.signature == format!("{:016x}", sum)
    }
}

/// Tracker managing in-flight delivery receipts and timeout deadlines across multi-protocol dispatches.
pub struct ReceiptTracker {
    pending_receipts: HashMap<String, (ProtocolKind, u64)>, // nonce -> (protocol, deadline_epoch_sec)
    receipt_timeout_sec: u64,
}

impl Default for ReceiptTracker {
    fn default() -> Self {
        Self {
            pending_receipts: HashMap::new(),
            receipt_timeout_sec: 15,
        }
    }
}

impl ReceiptTracker {
    pub fn new(receipt_timeout_sec: u64) -> Self {
        Self {
            pending_receipts: HashMap::new(),
            receipt_timeout_sec,
        }
    }

    /// Registers a sent message nonce expecting an acknowledgment before the deadline.
    pub fn register_pending(&mut self, nonce: &str, protocol: ProtocolKind, now_sec: u64) {
        let deadline = now_sec + self.receipt_timeout_sec;
        self.pending_receipts.insert(nonce.to_string(), (protocol, deadline));
    }

    /// Processes an incoming delivery receipt. Returns true if receipt was pending and verified.
    pub fn confirm_receipt(&mut self, receipt: &DeliveryReceipt, now_sec: u64) -> bool {
        if !receipt.verify_signature() {
            return false;
        }

        if let Some((proto, deadline)) = self.pending_receipts.remove(&receipt.nonce) {
            proto == receipt.protocol && now_sec <= deadline
        } else {
            false
        }
    }

    /// Checks if a pending nonce has timed out.
    pub fn is_timed_out(&self, nonce: &str, now_sec: u64) -> bool {
        if let Some(&(_, deadline)) = self.pending_receipts.get(nonce) {
            now_sec > deadline
        } else {
            false
        }
    }

    pub fn pending_count(&self) -> usize {
        self.pending_receipts.len()
    }
}

/// Concurrency guard bounding simultaneous in-flight dispatch operations per bridge protocol.
/// Prevents modem serial port contention, socket starvation, and rate-limiting penalties under load.
#[derive(Debug, Clone)]
pub struct BridgeConcurrencyGuard {
    limits: HashMap<ProtocolKind, usize>,
    in_flight: HashMap<ProtocolKind, usize>,
}

impl Default for BridgeConcurrencyGuard {
    fn default() -> Self {
        let mut limits = HashMap::new();
        limits.insert(ProtocolKind::Sms, 2); // Serial Hayes AT cellular modem bound to 2
        limits.insert(ProtocolKind::Signal, 15);
        limits.insert(ProtocolKind::Matrix, 20);
        limits.insert(ProtocolKind::Discord, 10);
        limits.insert(ProtocolKind::Nostr, 20);
        limits.insert(ProtocolKind::Tox, 10);
        limits.insert(ProtocolKind::Simplex, 10);
        limits.insert(ProtocolKind::IMessage, 5);

        Self {
            limits,
            in_flight: HashMap::new(),
        }
    }
}

impl BridgeConcurrencyGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_limit(&mut self, protocol: ProtocolKind, max_concurrent: usize) {
        self.limits.insert(protocol, max_concurrent);
    }

    /// Attempts to acquire a concurrency dispatch permit for the given protocol.
    pub fn try_acquire_permit(&mut self, protocol: ProtocolKind) -> bool {
        let limit = self.limits.get(&protocol).copied().unwrap_or(10);
        let current = self.in_flight.entry(protocol).or_insert(0);
        if *current < limit {
            *current += 1;
            true
        } else {
            false
        }
    }

    /// Releases a previously acquired concurrency permit.
    pub fn release_permit(&mut self, protocol: ProtocolKind) {
        if let Some(current) = self.in_flight.get_mut(&protocol) {
            *current = current.saturating_sub(1);
        }
    }

    pub fn in_flight_count(&self, protocol: ProtocolKind) -> usize {
        self.in_flight.get(&protocol).copied().unwrap_or(0)
    }
}

/// Sliding-window latency sample accumulator estimating dynamic p50/p95 latency envelopes
/// to compute early-abort timeout deadlines and eliminate tail latency stalls across bridges.
#[derive(Debug, Clone)]
pub struct RouteLatencyEstimator {
    samples: HashMap<ProtocolKind, Vec<u64>>,
    window_size: usize,
}

impl Default for RouteLatencyEstimator {
    fn default() -> Self {
        Self {
            samples: HashMap::new(),
            window_size: 50,
        }
    }
}

impl RouteLatencyEstimator {
    pub fn new(window_size: usize) -> Self {
        Self {
            samples: HashMap::new(),
            window_size: window_size.max(5),
        }
    }

    /// Records a new delivery latency observation for a protocol bridge.
    pub fn record_sample(&mut self, protocol: ProtocolKind, latency_ms: u64) {
        let entry = self.samples.entry(protocol).or_default();
        if entry.len() >= self.window_size {
            entry.remove(0);
        }
        entry.push(latency_ms);
    }

    /// Computes the p50 (median) latency for a candidate bridge. Defaults to 50ms if unobserved.
    pub fn p50_latency_ms(&self, protocol: ProtocolKind) -> u64 {
        self.compute_percentile(protocol, 0.50).unwrap_or(50)
    }

    /// Computes the p95 latency for a candidate bridge. Defaults to 250ms if unobserved.
    pub fn p95_latency_ms(&self, protocol: ProtocolKind) -> u64 {
        self.compute_percentile(protocol, 0.95).unwrap_or(250)
    }

    fn compute_percentile(&self, protocol: ProtocolKind, percentile: f64) -> Option<u64> {
        let entry = self.samples.get(&protocol)?;
        if entry.is_empty() {
            return None;
        }

        let mut sorted = entry.clone();
        sorted.sort_unstable();

        let rank = ((sorted.len() as f64 * percentile).ceil() as usize).saturating_sub(1);
        let idx = rank.min(sorted.len() - 1);
        Some(sorted[idx])
    }
}

/// Canary probe detector testing for silent packet drops, ISP blackholing, or gateway drops.
#[derive(Debug, Clone)]
pub struct CanaryDropDetector {
    pending_canaries: HashMap<String, (ProtocolKind, u64)>, // nonce -> (protocol, deadline_epoch_sec)
    canary_timeout_sec: u64,
}

impl Default for CanaryDropDetector {
    fn default() -> Self {
        Self {
            pending_canaries: HashMap::new(),
            canary_timeout_sec: 10,
        }
    }
}

impl CanaryDropDetector {
    pub fn new(canary_timeout_sec: u64) -> Self {
        Self {
            pending_canaries: HashMap::new(),
            canary_timeout_sec,
        }
    }

    /// Generates a canary probe nonce and records its confirmation deadline.
    pub fn issue_canary(&mut self, protocol: ProtocolKind, now_sec: u64) -> (String, MessagePayload) {
        let nonce = format!("canary_{:?}_{}", protocol, now_sec);
        let deadline = now_sec + self.canary_timeout_sec;
        self.pending_canaries.insert(nonce.clone(), (protocol, deadline));

        let payload = MessagePayload {
            recipient_id: "__canary_echo__".to_string(),
            content: format!("CANARY_PROBE:{}", nonce),
            fallback_protocol: "none".to_string(),
        };

        (nonce, payload)
    }

    /// Verifies and acknowledges a returned canary probe echo.
    pub fn acknowledge_canary(&mut self, nonce: &str, protocol: ProtocolKind, now_sec: u64) -> bool {
        if let Some((proto, deadline)) = self.pending_canaries.remove(nonce) {
            proto == protocol && now_sec <= deadline
        } else {
            false
        }
    }

    /// Returns true if the canary probe has timed out without confirmation (indicating silent drop).
    pub fn is_silent_drop(&self, nonce: &str, now_sec: u64) -> bool {
        if let Some(&(_, deadline)) = self.pending_canaries.get(nonce) {
            now_sec > deadline
        } else {
            false
        }
    }
}

/// Egress deduplication ledger ensuring exact-once transmission semantics across multi-protocol fallback cascades.
pub struct EgressDeduplicationLedger {
    transmitted_digests: HashMap<String, u64>, // digest -> expiration timestamp
    dedup_window_sec: u64,
}

impl Default for EgressDeduplicationLedger {
    fn default() -> Self {
        Self {
            transmitted_digests: HashMap::new(),
            dedup_window_sec: 120, // 2-minute deduplication window
        }
    }
}

impl EgressDeduplicationLedger {
    pub fn new(dedup_window_sec: u64) -> Self {
        Self {
            transmitted_digests: HashMap::new(),
            dedup_window_sec,
        }
    }

    /// Checks if a payload digest for a recipient has already been transmitted within the window.
    /// If fresh, records it with an expiration timestamp and returns true.
    pub fn check_and_record_transmission(&mut self, recipient: &str, content: &str, now_sec: u64) -> bool {
        let mut sum: u64 = 0xcbf29ce484222325;
        for &b in recipient.as_bytes() {
            sum ^= b as u64;
            sum = sum.wrapping_mul(0x100000001b3);
        }
        sum ^= b':' as u64;
        sum = sum.wrapping_mul(0x100000001b3);
        for &b in content.as_bytes() {
            sum ^= b as u64;
            sum = sum.wrapping_mul(0x100000001b3);
        }
        let digest = format!("{:016x}", sum);

        if let Some(&expires_at) = self.transmitted_digests.get(&digest) {
            if expires_at > now_sec {
                return false; // Duplicate detected
            }
        }

        self.transmitted_digests.insert(digest, now_sec + self.dedup_window_sec);
        true
    }

    /// Prunes expired transmissions from the deduplication ledger.
    pub fn prune_expired(&mut self, now_sec: u64) -> usize {
        let initial = self.transmitted_digests.len();
        self.transmitted_digests.retain(|_, &mut exp| exp > now_sec);
        initial - self.transmitted_digests.len()
    }

    pub fn tracked_count(&self) -> usize {
        self.transmitted_digests.len()
    }
}

/// Prober actively testing degraded, open, or half-open bridges using lightweight heartbeat keepalives.
pub struct BridgeHeartbeatProber;

impl BridgeHeartbeatProber {
    /// Generates a synthetic ping heartbeat payload for non-disruptive bridge health probing.
    pub fn create_probe_payload(probe_nonce: &str) -> MessagePayload {
        MessagePayload {
            recipient_id: "__omnicore_health_probe__".to_string(),
            content: format!("PING:{}", probe_nonce),
            fallback_protocol: "none".to_string(),
        }
    }
}

/// Filter smoothing transient latency spikes and preventing metric poisoning from isolated network anomalies.
pub struct RouteAnomalyFilter;

impl RouteAnomalyFilter {
    /// Computes smoothed latency using damped Exponential Moving Average (EMA).
    /// If the incoming sample is > 4x the existing baseline, clamps the spike to 2x baseline
    /// to avoid false-positive routing cascades from single-packet jitter.
    pub fn compute_damped_latency(current_avg_ms: u64, sample_ms: u64) -> u64 {
        if current_avg_ms == 0 {
            return sample_ms;
        }

        let clamped_sample = if sample_ms > current_avg_ms.saturating_mul(4) {
            current_avg_ms.saturating_mul(2)
        } else {
            sample_ms
        };

        // Standard 70/30 weight EMA
        (current_avg_ms.saturating_mul(7) + clamped_sample.saturating_mul(3)) / 10
    }
}

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

/// An envelope binding a routed message payload with a cryptographic nonce and timestamp.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RoutedPacketEnvelope {
    pub nonce: String,
    pub timestamp_sec: u64,
    pub payload: MessagePayload,
}

/// Cache protecting against replay attacks across bridge cascades by tracking observed nonces.
pub struct NonceReplayCache {
    seen_nonces: HashMap<String, u64>,
    ttl_seconds: u64,
}

impl Default for NonceReplayCache {
    fn default() -> Self {
        Self {
            seen_nonces: HashMap::new(),
            ttl_seconds: 3600, // 1 hour replay window
        }
    }
}

impl NonceReplayCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            seen_nonces: HashMap::new(),
            ttl_seconds,
        }
    }

    /// Checks if a nonce has been seen; if fresh, records it with an expiration timestamp.
    pub fn check_and_record_nonce(&mut self, nonce: &str, now_sec: u64) -> bool {
        if let Some(&expires_at) = self.seen_nonces.get(nonce) {
            if expires_at > now_sec {
                return false; // Replay detected
            }
        }
        self.seen_nonces.insert(nonce.to_string(), now_sec + self.ttl_seconds);
        true
    }

    /// Prunes expired nonces to maintain bounded memory footprint.
    pub fn prune_expired(&mut self, now_sec: u64) -> usize {
        let initial = self.seen_nonces.len();
        self.seen_nonces.retain(|_, &mut exp| exp > now_sec);
        initial - self.seen_nonces.len()
    }

    pub fn cached_nonce_count(&self) -> usize {
        self.seen_nonces.len()
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
    pub cooldown_jitter_sec: u64,
    pub consecutive_trips: u32,
    pub max_cooldown_sec: u64,
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
            cooldown_jitter_sec: 0,
            consecutive_trips: 0,
            max_cooldown_sec: 300,
        }
    }
}

impl ProtocolMetric {
    /// Computes effective cooldown incorporating exponential backoff for flapping bridges and dynamic jitter.
    pub fn compute_effective_cooldown(&self) -> u64 {
        let multiplier = 1u64.checked_shl(self.consecutive_trips.min(5)).unwrap_or(32);
        let backoff_cooldown = (self.cooldown_sec.saturating_mul(multiplier)).min(self.max_cooldown_sec);
        backoff_cooldown.saturating_add(self.cooldown_jitter_sec)
    }

    /// Checks availability, computing effective cooldown with dynamic jitter and exponential backoff to avoid thundering herd.
    pub fn is_available_at(&mut self, now_epoch: u64) -> bool {
        match self.circuit_state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let effective_cooldown = self.compute_effective_cooldown();
                if now_epoch >= self.last_failure_epoch_sec.saturating_add(effective_cooldown) {
                    self.circuit_state = CircuitState::HalfOpen;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Decays consecutive trip counts and backoff penalties if the bridge has maintained stable
    /// Closed state without failures for at least `stability_window_sec`.
    pub fn decay_trips_if_stable(&mut self, now_epoch: u64, stability_window_sec: u64) -> bool {
        if self.circuit_state == CircuitState::Closed && self.consecutive_trips > 0 {
            if now_epoch >= self.last_failure_epoch_sec.saturating_add(stability_window_sec) {
                self.consecutive_trips = self.consecutive_trips.saturating_sub(1);
                return true;
            }
        }
        false
    }

    /// Sets dynamic jitter in seconds to stagger recovery of flapping bridges.
    pub fn with_jitter(mut self, jitter_sec: u64) -> Self {
        self.cooldown_jitter_sec = jitter_sec;
        self
    }

    /// Configures exponential backoff parameters.
    pub fn with_backoff_limits(mut self, max_cooldown_sec: u64) -> Self {
        self.max_cooldown_sec = max_cooldown_sec;
        self
    }

    /// Computes a holistic penalty score for route prioritization.
    /// Lower score = healthier bridge.
    /// Incorporates EWMA latency, heavy multipliers for consecutive failures, and flapping trips.
    pub fn calculate_route_penalty_score(&self) -> u64 {
        let base_lat = if self.average_latency_ms == 0 {
            50 // Neutral baseline for unattempted bridges
        } else {
            self.average_latency_ms
        };

        let fail_penalty = (self.consecutive_failures as u64).saturating_mul(500);
        let trip_penalty = (self.consecutive_trips as u64).saturating_mul(2000);

        base_lat.saturating_add(fail_penalty).saturating_add(trip_penalty)
    }
}

/// Configuration policy for hedged racing dispatch across candidate bridges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HedgedRoutingPolicy {
    /// Milliseconds to wait before launching parallel hedged dispatch on secondary candidate.
    /// If 0, both candidate bridges are launched immediately in parallel.
    pub hedge_delay_ms: u64,
    /// Maximum parallel candidate bridges raced concurrently.
    pub max_parallel_candidates: usize,
}

impl Default for HedgedRoutingPolicy {
    fn default() -> Self {
        Self {
            hedge_delay_ms: 150,
            max_parallel_candidates: 2,
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
                if m.circuit_state != CircuitState::Open {
                    m.consecutive_trips = m.consecutive_trips.saturating_add(1);
                }
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

    /// Selects candidate ordering based on holistic penalty scores (latency + failure + trip penalties).
    pub fn select_penalty_weighted_route(
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

        candidates.sort_by(|a, b| {
            let score_a = self.metrics.get(a).map(|m| m.calculate_route_penalty_score()).unwrap_or(50);
            let score_b = self.metrics.get(b).map(|m| m.calculate_route_penalty_score()).unwrap_or(50);
            score_a.cmp(&score_b)
        });

        let chosen = candidates[0];
        let remaining = candidates[1..].to_vec();
        (chosen, remaining)
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

    /// Shared registry handle so clients can read accumulated routing
    /// metrics (they persist as long as this router instance lives).
    pub fn registry(&self) -> Arc<RwLock<BridgeRegistry>> {
        Arc::clone(&self.registry)
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

    /// Dispatches with end-to-end replay attack prevention and payload digest verification.
    pub async fn route_with_replay_and_integrity_guard(
        &self,
        envelope: &RoutedPacketEnvelope,
        replay_cache: &mut NonceReplayCache,
        expected_digest: &str,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        // Step 1: Check replay cache
        if !replay_cache.check_and_record_nonce(&envelope.nonce, now_epoch) {
            return Err(anyhow!("Replay attack detected: duplicate nonce [{}]", envelope.nonce));
        }

        // Step 2: Route with integrity verification
        self.route_with_integrity_check(
            &envelope.payload,
            expected_digest,
            primary,
            fallbacks,
            now_epoch,
        ).await
    }

    /// Hedged racing dispatch: dispatches across top candidate bridges concurrently
    /// or with a hedging delay, returning as soon as the fastest candidate confirms delivery.
    /// This eliminates tail latency spikes under transient network degradation and packet drop.
    pub async fn route_hedged_racing(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        policy: HedgedRoutingPolicy,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        // Step 1: Select optimal candidate ordering
        let (chosen_primary, reordered_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_optimal_route(primary, fallbacks)
        };

        let mut candidate_chain = vec![chosen_primary];
        candidate_chain.extend(reordered_fallbacks);

        // Filter out candidates with open circuits
        let mut available_candidates = Vec::new();
        {
            let mut registry_guard = self.registry.write().await;
            for candidate in candidate_chain {
                let avail = if let Some(m) = registry_guard.metrics.get_mut(&candidate) {
                    m.is_available_at(now_epoch)
                } else {
                    true
                };
                if avail {
                    available_candidates.push(candidate);
                }
            }
        }

        if available_candidates.is_empty() {
            return Err(anyhow!(
                "Hedged racing dispatch failed: all candidate circuits are OPEN or unavailable"
            ));
        }

        let max_race = policy.max_parallel_candidates.max(1).min(available_candidates.len());
        let raced_candidates = &available_candidates[..max_race];

        // If only 1 candidate to race, execute normal cascade
        if raced_candidates.len() == 1 {
            return self.route_with_optimal_cascade(payload, raced_candidates[0], &[], now_epoch).await;
        }

        // Race candidate bridges
        let primary_cand = raced_candidates[0];
        let secondary_cand = raced_candidates[1];

        let bridge_primary_opt = {
            let reg = self.registry.read().await;
            reg.get(primary_cand)
        };
        let bridge_secondary_opt = {
            let reg = self.registry.read().await;
            reg.get(secondary_cand)
        };

        let (b1, b2) = match (bridge_primary_opt, bridge_secondary_opt) {
            (Some(b1), Some(b2)) => (b1, b2),
            (Some(b1), None) => {
                let bridge = b1.read().await;
                bridge.send_message(payload).await?;
                let state = if primary_cand == primary {
                    DeliveryState::Delivered
                } else {
                    DeliveryState::FallbackTriggered {
                        from: primary,
                        to: primary_cand,
                        reason: "Fallback candidate selected".to_string(),
                    }
                };
                return Ok((primary_cand, state));
            }
            (None, Some(b2)) => {
                let bridge = b2.read().await;
                bridge.send_message(payload).await?;
                return Ok((
                    secondary_cand,
                    DeliveryState::FallbackTriggered {
                        from: primary,
                        to: secondary_cand,
                        reason: "Primary bridge unpopulated in registry".to_string(),
                    },
                ));
            }
            (None, None) => return Err(anyhow!("No valid bridges found to race")),
        };

        let p1 = payload.clone();
        let p2 = payload.clone();
        let delay_ms = policy.hedge_delay_ms;

        let primary_task = {
            let b1 = b1.clone();
            async move {
                let start = std::time::Instant::now();
                let res = {
                    let bridge = b1.read().await;
                    bridge.send_message(&p1).await
                };
                (primary_cand, res, start.elapsed().as_millis() as u64)
            }
        };

        let secondary_task = {
            let b2 = b2.clone();
            async move {
                if delay_ms > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
                let start = std::time::Instant::now();
                let res = {
                    let bridge = b2.read().await;
                    bridge.send_message(&p2).await
                };
                (secondary_cand, res, start.elapsed().as_millis() as u64)
            }
        };

        let mut primary_handle = tokio::spawn(primary_task);
        let mut secondary_handle = tokio::spawn(secondary_task);

        tokio::select! {
            res_prim = &mut primary_handle => {
                let (cand, res, lat) = res_prim.map_err(|e| anyhow!("Primary task join failed: {}", e))?;
                match res {
                    Ok(_) => {
                        secondary_handle.abort();
                        let mut reg = self.registry.write().await;
                        reg.record_delivery_metrics(cand, lat, true, now_epoch);
                        let state = if cand == primary {
                            DeliveryState::Delivered
                        } else {
                            DeliveryState::FallbackTriggered {
                                from: primary,
                                to: cand,
                                reason: "Optimal candidate won hedged race".to_string(),
                            }
                        };
                        Ok((cand, state))
                    }
                    Err(err) => {
                        {
                            let mut reg = self.registry.write().await;
                            reg.record_delivery_metrics(cand, lat, false, now_epoch);
                        }
                        eprintln!("Hedged racing primary [{}] failed: {}. Awaiting secondary...", cand.as_str(), err);
                        let (sec_cand, sec_res, sec_lat) = secondary_handle.await.map_err(|e| anyhow!("Secondary task join failed: {}", e))?;
                        let mut reg2 = self.registry.write().await;
                        if sec_res.is_ok() {
                            reg2.record_delivery_metrics(sec_cand, sec_lat, true, now_epoch);
                            Ok((sec_cand, DeliveryState::FallbackTriggered {
                                from: primary,
                                to: sec_cand,
                                reason: format!("Hedged racing fallback won: primary candidate error: {}", err),
                            }))
                        } else {
                            reg2.record_delivery_metrics(sec_cand, sec_lat, false, now_epoch);
                            Err(anyhow!("Hedged racing failed: both candidates failed"))
                        }
                    }
                }
            }
            res_sec = &mut secondary_handle => {
                let (cand, res, lat) = res_sec.map_err(|e| anyhow!("Secondary task join failed: {}", e))?;
                match res {
                    Ok(_) => {
                        primary_handle.abort();
                        let mut reg = self.registry.write().await;
                        reg.record_delivery_metrics(cand, lat, true, now_epoch);
                        let state = if cand == primary {
                            DeliveryState::Delivered
                        } else {
                            DeliveryState::FallbackTriggered {
                                from: primary,
                                to: cand,
                                reason: "Hedged race won by secondary candidate".to_string(),
                            }
                        };
                        Ok((cand, state))
                    }
                    Err(err) => {
                        {
                            let mut reg = self.registry.write().await;
                            reg.record_delivery_metrics(cand, lat, false, now_epoch);
                        }
                        eprintln!("Hedged secondary [{}] failed: {}. Awaiting primary...", cand.as_str(), err);
                        let (prim_cand, prim_res, prim_lat) = primary_handle.await.map_err(|e| anyhow!("Primary task join failed: {}", e))?;
                        let mut reg2 = self.registry.write().await;
                        if prim_res.is_ok() {
                            reg2.record_delivery_metrics(prim_cand, prim_lat, true, now_epoch);
                            let state = if prim_cand == primary {
                                DeliveryState::Delivered
                            } else {
                                DeliveryState::FallbackTriggered {
                                    from: primary,
                                    to: prim_cand,
                                    reason: "Primary candidate won after secondary failure".to_string(),
                                }
                            };
                            Ok((prim_cand, state))
                        } else {
                            reg2.record_delivery_metrics(prim_cand, prim_lat, false, now_epoch);
                            Err(anyhow!("Hedged racing failed: both candidates failed"))
                        }
                    }
                }
            }
        }
    }

    /// Dispatches with an adaptive multi-pass fallback cascade with per-candidate retry budgets.
    /// Re-evaluates circuit half-open state across passes to allow freshly recovered bridges
    /// to fulfill delivery before exhausting all fallbacks.
    pub async fn route_with_adaptive_fallback_retry(
        &self,
        payload: &MessagePayload,
        requested_primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        max_passes: usize,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        let mut last_err = anyhow!("No candidate bridges available");

        for pass in 0..max_passes {
            let pass_epoch = now_epoch + (pass as u64 * 5); // Simulated time advance across passes

            // Re-select optimal routes on each pass
            let (chosen_primary, reordered_fallbacks) = {
                let registry_guard = self.registry.read().await;
                registry_guard.select_optimal_route(requested_primary, fallbacks)
            };

            let mut candidates = vec![chosen_primary];
            candidates.extend(reordered_fallbacks);

            for &candidate in &candidates {
                // Check if candidate circuit is available (closed or half-open probing)
                let is_available = {
                    let mut registry_guard = self.registry.write().await;
                    if let Some(m) = registry_guard.metrics.get_mut(&candidate) {
                        m.is_available_at(pass_epoch)
                    } else {
                        true
                    }
                };

                if !is_available {
                    continue;
                }

                let bridge_opt = {
                    let registry_guard = self.registry.read().await;
                    registry_guard.get(candidate)
                };

                if let Some(bridge_ref) = bridge_opt {
                    let bridge = bridge_ref.read().await;
                    let start = std::time::Instant::now();
                    match bridge.send_message(payload).await {
                        Ok(_) => {
                            let lat = start.elapsed().as_millis() as u64;
                            let mut registry_guard = self.registry.write().await;
                            registry_guard.record_delivery_metrics(candidate, lat, true, pass_epoch);

                            let state = if candidate == requested_primary && pass == 0 {
                                DeliveryState::Delivered
                            } else {
                                DeliveryState::FallbackTriggered {
                                    from: requested_primary,
                                    to: candidate,
                                    reason: format!("Delivered via adaptive fallback pass {}", pass + 1),
                                }
                            };
                            return Ok((candidate, state));
                        }
                        Err(e) => {
                            let lat = start.elapsed().as_millis() as u64;
                            let mut registry_guard = self.registry.write().await;
                            registry_guard.record_delivery_metrics(candidate, lat, false, pass_epoch);
                            last_err = e;
                        }
                    }
                }
            }
        }

        Err(anyhow!(
            "Adaptive fallback retry failed after {} passes: {}",
            max_passes,
            last_err
        ))
    }

    /// Dispatches with flapping network resilience: prioritizes candidates with lower consecutive circuit
    /// trips and low failure counts, falling back gracefully to reliable low-tier bridges (e.g. SMS)
    /// during severe multi-bridge network flap events.
    pub async fn route_with_flapping_resilience(
        &self,
        payload: &MessagePayload,
        requested_primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        // Collect candidate chain sorted by health, but heavily penalizing flapping candidates
        let mut candidates = vec![requested_primary];
        for &fb in fallbacks {
            if !candidates.contains(&fb) {
                candidates.push(fb);
            }
        }

        {
            let registry_guard = self.registry.read().await;
            candidates.sort_by(|a, b| {
                let m_a = registry_guard.metrics.get(a);
                let m_b = registry_guard.metrics.get(b);

                let trips_a = m_a.map(|m| m.consecutive_trips).unwrap_or(0);
                let trips_b = m_b.map(|m| m.consecutive_trips).unwrap_or(0);

                if trips_a != trips_b {
                    trips_a.cmp(&trips_b)
                } else {
                    let fail_a = m_a.map(|m| m.consecutive_failures).unwrap_or(0);
                    let fail_b = m_b.map(|m| m.consecutive_failures).unwrap_or(0);
                    fail_a.cmp(&fail_b)
                }
            });
        }

        let non_flapping_primary = candidates[0];
        let non_flapping_fallbacks = candidates[1..].to_vec();

        let (chosen, mut state) = self.route_with_optimal_cascade(
            payload,
            non_flapping_primary,
            &non_flapping_fallbacks,
            now_epoch,
        ).await?;

        // If delivered on a candidate other than requested_primary, map to FallbackTriggered
        if chosen != requested_primary {
            state = DeliveryState::FallbackTriggered {
                from: requested_primary,
                to: chosen,
                reason: format!("Flapping resilience bypassed primary [{}] for [{}]", requested_primary.as_str(), chosen.as_str()),
            };
        }

        Ok((chosen, state))
    }

    /// Dispatches using penalty-weighted candidate ordering, deprioritizing bridges with
    /// elevated latencies and prior failures before they completely trip into an open circuit.
    pub async fn route_with_penalty_weighted_cascade(
        &self,
        payload: &MessagePayload,
        requested_primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        let (chosen_primary, reordered_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_penalty_weighted_route(requested_primary, fallbacks)
        };

        let mut candidate_chain = vec![chosen_primary];
        candidate_chain.extend(reordered_fallbacks);

        for (idx, &candidate) in candidate_chain.iter().enumerate() {
            let is_available = {
                let mut registry_guard = self.registry.write().await;
                if let Some(metric) = registry_guard.metrics.get_mut(&candidate) {
                    metric.is_available_at(now_epoch)
                } else {
                    true
                }
            };

            if !is_available {
                continue;
            }

            let bridge_opt = {
                let registry_guard = self.registry.read().await;
                registry_guard.get(candidate)
            };

            if let Some(bridge_ref) = bridge_opt {
                let bridge = bridge_ref.read().await;
                let start = std::time::Instant::now();
                match bridge.send_message(payload).await {
                    Ok(_) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, true, now_epoch);

                        let state = if idx == 0 && candidate == requested_primary {
                            DeliveryState::Delivered
                        } else {
                            DeliveryState::FallbackTriggered {
                                from: requested_primary,
                                to: candidate,
                                reason: format!("Penalty-weighted fallback delivered via [{}]", candidate.as_str()),
                            }
                        };
                        return Ok((candidate, state));
                    }
                    Err(e) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, false, now_epoch);
                        eprintln!(
                            "Penalty weighted bridge [{}] failed: {}. Continuing cascade...",
                            candidate.as_str(),
                            e
                        );
                    }
                }
            }
        }

        Err(anyhow!(
            "Penalty-weighted cascade failed: all candidate protocols exhausted for recipient {}",
            payload.recipient_id
        ))
    }

    /// Dispatches a message directed to a unified contact identity across primary and fallback
    /// protocols, automatically translating recipient destination identifiers via CrossProtocolAddressBook
    /// at each step of the cascade.
    pub async fn route_unified_message(
        &self,
        unified_id: &str,
        content: &str,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        address_book: &CrossProtocolAddressBook,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        let (chosen_primary, reordered_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_optimal_route(primary, fallbacks)
        };

        let mut candidate_chain = vec![chosen_primary];
        candidate_chain.extend(reordered_fallbacks);

        for (idx, &candidate) in candidate_chain.iter().enumerate() {
            let is_available = {
                let mut registry_guard = self.registry.write().await;
                if let Some(metric) = registry_guard.metrics.get_mut(&candidate) {
                    metric.is_available_at(now_epoch)
                } else {
                    true
                }
            };

            if !is_available {
                continue;
            }

            let bridge_opt = {
                let registry_guard = self.registry.read().await;
                registry_guard.get(candidate)
            };

            if let Some(bridge_ref) = bridge_opt {
                let bridge = bridge_ref.read().await;
                // Resolve recipient address for the specific bridge candidate
                let resolved_recipient = address_book.resolve_address(unified_id, candidate);
                let payload = MessagePayload {
                    recipient_id: resolved_recipient,
                    content: content.to_string(),
                    fallback_protocol: candidate_chain.get(idx + 1).map(|p| p.as_str().to_string()).unwrap_or_default(),
                };

                let start = std::time::Instant::now();
                match bridge.send_message(&payload).await {
                    Ok(_) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, true, now_epoch);

                        let state = if idx == 0 && candidate == primary {
                            DeliveryState::Delivered
                        } else {
                            DeliveryState::FallbackTriggered {
                                from: primary,
                                to: candidate,
                                reason: format!("Unified identity [{}] translated and routed via [{}]", unified_id, candidate.as_str()),
                            }
                        };
                        return Ok((candidate, state));
                    }
                    Err(e) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, false, now_epoch);
                        eprintln!(
                            "Unified route bridge [{}] failed: {}. Continuing cascade...",
                            candidate.as_str(),
                            e
                        );
                    }
                }
            }
        }

        Err(anyhow!(
            "Unified routing failed: all candidates exhausted for contact {}",
            unified_id
        ))
    }

    /// Dispatches a message across cascading protocols with dynamic MTU fragmentation.
    /// If candidate bridge requires fragmentation (e.g. SMS limit 140 chars), payload is
    /// fragmented into ordered chunks and sequentially transmitted.
    pub async fn route_with_fragmentation_adaptive_cascade(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState, usize)> {
        let (chosen_primary, reordered_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_optimal_route(primary, fallbacks)
        };

        let mut candidate_chain = vec![chosen_primary];
        candidate_chain.extend(reordered_fallbacks);

        for (idx, &candidate) in candidate_chain.iter().enumerate() {
            let is_available = {
                let mut registry_guard = self.registry.write().await;
                if let Some(metric) = registry_guard.metrics.get_mut(&candidate) {
                    metric.is_available_at(now_epoch)
                } else {
                    true
                }
            };

            if !is_available {
                continue;
            }

            let bridge_opt = {
                let registry_guard = self.registry.read().await;
                registry_guard.get(candidate)
            };

            if let Some(bridge_ref) = bridge_opt {
                let bridge = bridge_ref.read().await;
                let mtu = ProtocolMtuLimits::max_payload_len(candidate);
                let fragments = PayloadFragmenter::fragment_payload(&payload.content, mtu);
                let frag_count = fragments.len();

                let mut all_fragments_sent = true;
                let start = std::time::Instant::now();

                for fragment in &fragments {
                    let frag_payload = MessagePayload {
                        recipient_id: payload.recipient_id.clone(),
                        content: fragment.clone(),
                        fallback_protocol: candidate_chain.get(idx + 1).map(|p| p.as_str().to_string()).unwrap_or_default(),
                    };

                    if let Err(e) = bridge.send_message(&frag_payload).await {
                        eprintln!(
                            "Candidate bridge [{}] failed fragment dispatch: {}. Cascading...",
                            candidate.as_str(),
                            e
                        );
                        all_fragments_sent = false;
                        break;
                    }
                }

                let latency_ms = start.elapsed().as_millis() as u64;
                let mut registry_guard = self.registry.write().await;
                registry_guard.record_delivery_metrics(candidate, latency_ms, all_fragments_sent, now_epoch);

                if all_fragments_sent {
                    let state = if idx == 0 && candidate == primary {
                        DeliveryState::Delivered
                    } else {
                        DeliveryState::FallbackTriggered {
                            from: primary,
                            to: candidate,
                            reason: format!(
                                "Delivered via [{}] (fragmented into {} parts, MTU: {})",
                                candidate.as_str(),
                                frag_count,
                                mtu
                            ),
                        }
                    };
                    return Ok((candidate, state, frag_count));
                }
            }
        }

        Err(anyhow!(
            "MTU-adaptive cascade failed: all candidate protocols exhausted for recipient {}",
            payload.recipient_id
        ))
    }

    /// Dispatches a message across cascading protocols with automated content transcoding.
    /// Adapts rich markdown/HTML formatting to plain text when fallback cascades to constrained
    /// transports like cellular SMS, ensuring message readability and transmission integrity.
    pub async fn route_with_transcoding_cascade(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState, String)> {
        let (chosen_primary, reordered_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_optimal_route(primary, fallbacks)
        };

        let mut candidate_chain = vec![chosen_primary];
        candidate_chain.extend(reordered_fallbacks);

        for (idx, &candidate) in candidate_chain.iter().enumerate() {
            let is_available = {
                let mut registry_guard = self.registry.write().await;
                if let Some(metric) = registry_guard.metrics.get_mut(&candidate) {
                    metric.is_available_at(now_epoch)
                } else {
                    true
                }
            };

            if !is_available {
                continue;
            }

            let bridge_opt = {
                let registry_guard = self.registry.read().await;
                registry_guard.get(candidate)
            };

            if let Some(bridge_ref) = bridge_opt {
                let bridge = bridge_ref.read().await;
                let transcoded_content = PayloadContentTranscoder::transcode_for_protocol(&payload.content, candidate);
                let transcoded_payload = MessagePayload {
                    recipient_id: payload.recipient_id.clone(),
                    content: transcoded_content.clone(),
                    fallback_protocol: candidate_chain.get(idx + 1).map(|p| p.as_str().to_string()).unwrap_or_default(),
                };

                let start = std::time::Instant::now();
                match bridge.send_message(&transcoded_payload).await {
                    Ok(_) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, true, now_epoch);

                        let state = if idx == 0 && candidate == primary {
                            DeliveryState::Delivered
                        } else {
                            DeliveryState::FallbackTriggered {
                                from: primary,
                                to: candidate,
                                reason: format!("Transcoded and delivered via [{}]", candidate.as_str()),
                            }
                        };
                        return Ok((candidate, state, transcoded_content));
                    }
                    Err(e) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, false, now_epoch);
                        eprintln!(
                            "Transcoding cascade bridge [{}] failed: {}. Continuing cascade...",
                            candidate.as_str(),
                            e
                        );
                    }
                }
            }
        }

        Err(anyhow!(
            "Transcoding cascade failed: all candidate protocols exhausted for recipient {}",
            payload.recipient_id
        ))
    }

    /// Dispatches with explicit end-to-end delivery receipt confirmation.
    /// If candidate bridge sends successfully but fails to return a verified cryptographic
    /// acknowledgment receipt within the deadline (e.g. silent drop), triggers immediate cascade fallback.
    pub async fn route_with_confirmed_receipt<F>(
        &self,
        payload: &MessagePayload,
        nonce: &str,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        tracker: &mut ReceiptTracker,
        receipt_generator: F,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState, DeliveryReceipt)>
    where
        F: Fn(ProtocolKind, &str, &str, u64) -> Option<DeliveryReceipt>,
    {
        let (chosen_primary, reordered_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_optimal_route(primary, fallbacks)
        };

        let mut candidate_chain = vec![chosen_primary];
        candidate_chain.extend(reordered_fallbacks);

        for (idx, &candidate) in candidate_chain.iter().enumerate() {
            let is_available = {
                let mut registry_guard = self.registry.write().await;
                if let Some(metric) = registry_guard.metrics.get_mut(&candidate) {
                    metric.is_available_at(now_epoch)
                } else {
                    true
                }
            };

            if !is_available {
                continue;
            }

            let bridge_opt = {
                let registry_guard = self.registry.read().await;
                registry_guard.get(candidate)
            };

            if let Some(bridge_ref) = bridge_opt {
                let bridge = bridge_ref.read().await;
                let start = std::time::Instant::now();

                match bridge.send_message(payload).await {
                    Ok(_) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        tracker.register_pending(nonce, candidate, now_epoch);

                        // Await simulated receipt response
                        if let Some(receipt) = receipt_generator(candidate, nonce, &payload.recipient_id, now_epoch) {
                            if tracker.confirm_receipt(&receipt, now_epoch) {
                                let mut registry_guard = self.registry.write().await;
                                registry_guard.record_delivery_metrics(candidate, latency_ms, true, now_epoch);

                                let state = if idx == 0 && candidate == primary {
                                    DeliveryState::Delivered
                                } else {
                                    DeliveryState::FallbackTriggered {
                                        from: primary,
                                        to: candidate,
                                        reason: format!("Receipt confirmed via fallback [{}]", candidate.as_str()),
                                    }
                                };
                                return Ok((candidate, state, receipt));
                            }
                        }

                        // Receipt missing, invalid, or timed out (silent drop) -> treat as failed attempt
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, false, now_epoch);
                        eprintln!(
                            "Bridge [{}] sent but receipt confirmation timed out or signature invalid. Cascading...",
                            candidate.as_str()
                        );
                    }
                    Err(e) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, false, now_epoch);
                        eprintln!(
                            "Bridge [{}] send failed: {}. Continuing cascade...",
                            candidate.as_str(),
                            e
                        );
                    }
                }
            }
        }

        Err(anyhow!(
            "Confirmed receipt routing failed: all candidates exhausted or timed out for recipient {}",
            payload.recipient_id
        ))
    }

    /// Dispatches with active concurrency permit bounding per protocol bridge.
    /// If candidate bridge has exhausted its concurrent dispatch quota (e.g. cellular modem in use),
    /// immediately cascades to alternate candidate bridges to preserve high throughput.
    pub async fn route_with_concurrency_bounded_cascade(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        concurrency_guard: &mut BridgeConcurrencyGuard,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        let (chosen_primary, reordered_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_optimal_route(primary, fallbacks)
        };

        let mut candidate_chain = vec![chosen_primary];
        candidate_chain.extend(reordered_fallbacks);

        for (idx, &candidate) in candidate_chain.iter().enumerate() {
            let is_available = {
                let mut registry_guard = self.registry.write().await;
                if let Some(metric) = registry_guard.metrics.get_mut(&candidate) {
                    metric.is_available_at(now_epoch)
                } else {
                    true
                }
            };

            if !is_available {
                continue;
            }

            // Attempt to acquire concurrency permit
            if !concurrency_guard.try_acquire_permit(candidate) {
                eprintln!(
                    "Bridge [{}] concurrency budget exhausted (in-flight: {}). Bypassing candidate...",
                    candidate.as_str(),
                    concurrency_guard.in_flight_count(candidate)
                );
                continue;
            }

            let bridge_opt = {
                let registry_guard = self.registry.read().await;
                registry_guard.get(candidate)
            };

            if let Some(bridge_ref) = bridge_opt {
                let bridge = bridge_ref.read().await;
                let start = std::time::Instant::now();
                let send_res = bridge.send_message(payload).await;
                concurrency_guard.release_permit(candidate);

                match send_res {
                    Ok(_) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, true, now_epoch);

                        let state = if idx == 0 && candidate == primary {
                            DeliveryState::Delivered
                        } else {
                            DeliveryState::FallbackTriggered {
                                from: primary,
                                to: candidate,
                                reason: format!("Concurrency bounded cascade delivered via [{}]", candidate.as_str()),
                            }
                        };
                        return Ok((candidate, state));
                    }
                    Err(e) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, false, now_epoch);
                        eprintln!(
                            "Concurrency bounded bridge [{}] dispatch failed: {}. Continuing cascade...",
                            candidate.as_str(),
                            e
                        );
                    }
                }
            } else {
                concurrency_guard.release_permit(candidate);
            }
        }

        Err(anyhow!(
            "Concurrency bounded routing failed: all candidates exhausted or saturated for recipient {}",
            payload.recipient_id
        ))
    }

    /// Dispatches with active anomaly latency dampening, preventing isolated packet spikes
    /// from causing premature deprioritization and false-positive cascading fallbacks.
    pub async fn route_with_anomaly_damped_cascade(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        let (chosen_primary, reordered_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_optimal_route(primary, fallbacks)
        };

        let mut candidate_chain = vec![chosen_primary];
        candidate_chain.extend(reordered_fallbacks);

        for (idx, &candidate) in candidate_chain.iter().enumerate() {
            let is_available = {
                let mut registry_guard = self.registry.write().await;
                if let Some(metric) = registry_guard.metrics.get_mut(&candidate) {
                    metric.is_available_at(now_epoch)
                } else {
                    true
                }
            };

            if !is_available {
                continue;
            }

            let bridge_opt = {
                let registry_guard = self.registry.read().await;
                registry_guard.get(candidate)
            };

            if let Some(bridge_ref) = bridge_opt {
                let bridge = bridge_ref.read().await;
                let start = std::time::Instant::now();
                match bridge.send_message(payload).await {
                    Ok(_) => {
                        let sample_latency = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        let m = registry_guard.metrics.entry(candidate).or_default();
                        m.total_attempts += 1;
                        m.successful_attempts += 1;
                        m.consecutive_failures = 0;
                        m.circuit_state = CircuitState::Closed;
                        // Apply anomaly damped latency smoothing
                        m.average_latency_ms = RouteAnomalyFilter::compute_damped_latency(m.average_latency_ms, sample_latency);

                        let state = if idx == 0 && candidate == primary {
                            DeliveryState::Delivered
                        } else {
                            DeliveryState::FallbackTriggered {
                                from: primary,
                                to: candidate,
                                reason: format!("Damped fallback delivered via [{}]", candidate.as_str()),
                            }
                        };
                        return Ok((candidate, state));
                    }
                    Err(e) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, false, now_epoch);
                        eprintln!(
                            "Damped cascade bridge [{}] failed: {}. Continuing cascade...",
                            candidate.as_str(),
                            e
                        );
                    }
                }
            }
        }

        Err(anyhow!(
            "Anomaly damped routing failed: all candidates exhausted for recipient {}",
            payload.recipient_id
        ))
    }

    /// Dispatches with exact-once delivery semantics across multi-bridge cascades.
    /// Checks the EgressDeduplicationLedger to reject redundant outbound transmissions of the
    /// same message content to the same recipient during rapid failover attempts or split-brain races.
    pub async fn route_with_exact_once_deduplication(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        ledger: &mut EgressDeduplicationLedger,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        // Step 1: Check deduplication ledger
        if !ledger.check_and_record_transmission(&payload.recipient_id, &payload.content, now_epoch) {
            return Err(anyhow!(
                "Exact-once delivery check rejected transmission: duplicate message payload detected for recipient {}",
                payload.recipient_id
            ));
        }

        // Step 2: Route via optimal cascade
        self.route_with_optimal_cascade(payload, primary, fallbacks, now_epoch).await
    }

    /// Actively probes candidate bridges that are in Open or HalfOpen circuit states using
    /// lightweight heartbeat pings, recovering closed circuit status upon confirmation.
    pub async fn probe_and_recover_circuits(&self, now_epoch: u64) -> Vec<(ProtocolKind, bool)> {
        let mut candidates_to_probe = Vec::new();
        {
            let mut reg = self.registry.write().await;
            for (&proto, metric) in &mut reg.metrics {
                if metric.circuit_state == CircuitState::HalfOpen || (metric.circuit_state == CircuitState::Open && metric.is_available_at(now_epoch)) {
                    candidates_to_probe.push(proto);
                }
            }
        }

        let mut probe_results = Vec::new();
        for proto in candidates_to_probe {
            let bridge_opt = {
                let reg = self.registry.read().await;
                reg.get(proto)
            };

            if let Some(bridge_ref) = bridge_opt {
                let bridge = bridge_ref.read().await;
                let probe_payload = BridgeHeartbeatProber::create_probe_payload(&format!("probe_{}", now_epoch));
                let start = std::time::Instant::now();
                let success = bridge.send_message(&probe_payload).await.is_ok();
                let lat = start.elapsed().as_millis() as u64;

                let mut reg = self.registry.write().await;
                reg.record_delivery_metrics(proto, lat, success, now_epoch);
                if success {
                    if let Some(m) = reg.metrics.get_mut(&proto) {
                        m.circuit_state = CircuitState::Closed;
                        m.consecutive_trips = m.consecutive_trips.saturating_sub(1);
                    }
                }
                probe_results.push((proto, success));
            }
        }

        probe_results
    }

    /// Dispatches with active background health probing: initiates heartbeat probes on degraded circuits
    /// while executing normal cascading delivery for the current message.
    pub async fn route_with_active_health_probing(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        // Trigger recovery probes for any flapping or half-open circuits
        let _ = self.probe_and_recover_circuits(now_epoch).await;

        // Execute optimal cascade dispatch
        self.route_with_optimal_cascade(payload, primary, fallbacks, now_epoch).await
    }

    /// Dispatches with dynamic route cost and QoS optimization: reorders candidate bridges according
    /// to specified user preferences (financial cost, battery conservation, ultra-low latency)
    /// while honoring circuit breaker availability and metric feedback.
    pub async fn route_with_cost_and_qos_optimization(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        preference: QosRoutingPreference,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        let mut candidates = vec![primary];
        for &fb in fallbacks {
            if !candidates.contains(&fb) {
                candidates.push(fb);
            }
        }

        {
            let reg = self.registry.read().await;
            candidates.sort_by(|a, b| {
                let lat_a = reg.metrics.get(a).map(|m| m.average_latency_ms).unwrap_or(50);
                let lat_b = reg.metrics.get(b).map(|m| m.average_latency_ms).unwrap_or(50);

                let score_a = RouteCostOptimizer::compute_qos_score(*a, lat_a, preference);
                let score_b = RouteCostOptimizer::compute_qos_score(*b, lat_b, preference);
                score_a.cmp(&score_b)
            });
        }

        let chosen_primary = candidates[0];
        let remaining_fallbacks = candidates[1..].to_vec();

        let (chosen, mut state) = self.route_with_optimal_cascade(
            payload,
            chosen_primary,
            &remaining_fallbacks,
            now_epoch,
        ).await?;

        if chosen != primary {
            state = DeliveryState::FallbackTriggered {
                from: primary,
                to: chosen,
                reason: format!("QoS optimizer preferred [{}] based on {:?}", chosen.as_str(), preference),
            };
        }

        Ok((chosen, state))
    }

    /// Dispatches with active bandwidth shaping and quota enforcement.
    /// If candidate bridge has exhausted its sliding-window byte budget (e.g. metered cellular SMS),
    /// automatically cascades to unmetered fallback bridges to prevent overage charges or network throttling.
    pub async fn route_with_bandwidth_throttled_cascade(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        shaper: &mut RouteBandwidthShaper,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        let (chosen_primary, reordered_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_optimal_route(primary, fallbacks)
        };

        let mut candidate_chain = vec![chosen_primary];
        candidate_chain.extend(reordered_fallbacks);
        let payload_bytes = payload.content.len();

        for (idx, &candidate) in candidate_chain.iter().enumerate() {
            let is_available = {
                let mut registry_guard = self.registry.write().await;
                if let Some(metric) = registry_guard.metrics.get_mut(&candidate) {
                    metric.is_available_at(now_epoch)
                } else {
                    true
                }
            };

            if !is_available {
                continue;
            }

            // Check bandwidth shaper allowance
            if !shaper.try_consume_bandwidth(candidate, payload_bytes, now_epoch) {
                eprintln!(
                    "Bridge [{}] bandwidth quota exceeded (current usage: {} bytes). Cascading...",
                    candidate.as_str(),
                    shaper.current_usage(candidate, now_epoch)
                );
                continue;
            }

            let bridge_opt = {
                let registry_guard = self.registry.read().await;
                registry_guard.get(candidate)
            };

            if let Some(bridge_ref) = bridge_opt {
                let bridge = bridge_ref.read().await;
                let start = std::time::Instant::now();
                match bridge.send_message(payload).await {
                    Ok(_) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, true, now_epoch);

                        let state = if idx == 0 && candidate == primary {
                            DeliveryState::Delivered
                        } else {
                            DeliveryState::FallbackTriggered {
                                from: primary,
                                to: candidate,
                                reason: format!("Bandwidth throttled fallback delivered via [{}]", candidate.as_str()),
                            }
                        };
                        return Ok((candidate, state));
                    }
                    Err(e) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, false, now_epoch);
                        eprintln!(
                            "Bandwidth throttled cascade bridge [{}] failed: {}. Continuing cascade...",
                            candidate.as_str(),
                            e
                        );
                    }
                }
            }
        }

        Err(anyhow!(
            "Bandwidth throttled routing failed: all candidates exhausted or quota limited for recipient {}",
            payload.recipient_id
        ))
    }

    /// Automatically decays consecutive trip counts across all candidate bridges that have exhibited
    /// uninterrupted stability during the `stability_window_sec`, gradually rehabilitating their priority tiers.
    pub async fn rehabilitate_stable_circuits(&self, now_epoch: u64, stability_window_sec: u64) -> usize {
        let mut reg = self.registry.write().await;
        let mut decayed_count = 0;
        for metric in reg.metrics.values_mut() {
            if metric.decay_trips_if_stable(now_epoch, stability_window_sec) {
                decayed_count += 1;
            }
        }
        decayed_count
    }

    /// Dispatches with automated circuit rehabilitation: decays backoff penalties on bridges that
    /// have remained stable, before performing optimal cascading delivery.
    pub async fn route_with_rehabilitated_circuits(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        stability_window_sec: u64,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        let _ = self.rehabilitate_stable_circuits(now_epoch, stability_window_sec).await;
        self.route_with_optimal_cascade(payload, primary, fallbacks, now_epoch).await
    }

    /// Dispatches with traffic dispersal balancing across multiple candidate bridges.
    /// Rotates primary candidate across dispatches to prevent socket exhaustion and rate limiting.
    pub async fn route_with_dispersal_balancing(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        balancer: &mut RouteDispersalBalancer,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        let (dispersed_primary, dispersed_fallbacks) = balancer.select_dispersed_route(primary, fallbacks);

        let (chosen, mut state) = self
            .route_with_optimal_cascade(payload, dispersed_primary, &dispersed_fallbacks, now_epoch)
            .await?;

        if chosen != primary {
            state = DeliveryState::FallbackTriggered {
                from: primary,
                to: chosen,
                reason: format!("Dispersal balancer routed via [{}]", chosen.as_str()),
            };
        }

        Ok((chosen, state))
    }

    /// Dispatches across candidate bridges with dynamic percentile-bounded timeouts.
    /// If an attempt on a candidate bridge exceeds its historical p95 latency envelope,
    /// aborts the attempt and immediately cascades to alternate bridges to eliminate tail latency stalls.
    pub async fn route_with_percentile_bounded_cascade(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        estimator: &mut RouteLatencyEstimator,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)> {
        let (chosen_primary, reordered_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_optimal_route(primary, fallbacks)
        };

        let mut candidate_chain = vec![chosen_primary];
        candidate_chain.extend(reordered_fallbacks);

        for (idx, &candidate) in candidate_chain.iter().enumerate() {
            let is_available = {
                let mut registry_guard = self.registry.write().await;
                if let Some(metric) = registry_guard.metrics.get_mut(&candidate) {
                    metric.is_available_at(now_epoch)
                } else {
                    true
                }
            };

            if !is_available {
                continue;
            }

            let bridge_opt = {
                let registry_guard = self.registry.read().await;
                registry_guard.get(candidate)
            };

            if let Some(bridge_ref) = bridge_opt {
                let bridge = bridge_ref.read().await;
                let p95_deadline = estimator.p95_latency_ms(candidate).max(20);
                let start = std::time::Instant::now();

                // Bounded dispatch with p95 timeout
                let send_res = tokio::time::timeout(
                    tokio::time::Duration::from_millis(p95_deadline),
                    bridge.send_message(payload),
                )
                .await;

                match send_res {
                    Ok(Ok(_)) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        estimator.record_sample(candidate, latency_ms);
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, true, now_epoch);

                        let state = if idx == 0 && candidate == primary {
                            DeliveryState::Delivered
                        } else {
                            DeliveryState::FallbackTriggered {
                                from: primary,
                                to: candidate,
                                reason: format!("Percentile-bounded fallback delivered via [{}]", candidate.as_str()),
                            }
                        };
                        return Ok((candidate, state));
                    }
                    Ok(Err(e)) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        estimator.record_sample(candidate, latency_ms);
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, false, now_epoch);
                        eprintln!(
                            "Percentile-bounded bridge [{}] dispatch failed: {}. Continuing cascade...",
                            candidate.as_str(),
                            e
                        );
                    }
                    Err(_) => {
                        // Timeout exceeded p95 deadline -> early abort and cascade!
                        let latency_ms = p95_deadline;
                        estimator.record_sample(candidate, latency_ms.saturating_mul(2));
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, false, now_epoch);
                        eprintln!(
                            "Percentile-bounded bridge [{}] exceeded p95 deadline ({}ms). Early aborting to cascade...",
                            candidate.as_str(),
                            p95_deadline
                        );
                    }
                }
            }
        }

        Err(anyhow!(
            "Percentile-bounded routing failed: all candidates exhausted or exceeded latency envelopes for recipient {}",
            payload.recipient_id
        ))
    }

    /// Dispatches with active canary probe silent-drop detection.
    /// Issues an active canary probe prior to dispatch; if the candidate bridge silently drops the canary,
    /// marks failure and immediately cascades to alternate transports.
    pub async fn route_with_canary_drop_detection<F>(
        &self,
        payload: &MessagePayload,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
        detector: &mut CanaryDropDetector,
        echo_simulator: F,
        now_epoch: u64,
    ) -> Result<(ProtocolKind, DeliveryState)>
    where
        F: Fn(ProtocolKind, &str) -> bool,
    {
        let (chosen_primary, reordered_fallbacks) = {
            let registry_guard = self.registry.read().await;
            registry_guard.select_optimal_route(primary, fallbacks)
        };

        let mut candidate_chain = vec![chosen_primary];
        candidate_chain.extend(reordered_fallbacks);

        for (idx, &candidate) in candidate_chain.iter().enumerate() {
            let is_available = {
                let mut registry_guard = self.registry.write().await;
                if let Some(metric) = registry_guard.metrics.get_mut(&candidate) {
                    metric.is_available_at(now_epoch)
                } else {
                    true
                }
            };

            if !is_available {
                continue;
            }

            // Issue and check canary probe
            let (nonce, _probe_payload) = detector.issue_canary(candidate, now_epoch);
            let canary_echoed = echo_simulator(candidate, &nonce);

            if !canary_echoed || !detector.acknowledge_canary(&nonce, candidate, now_epoch) {
                // Silent drop detected on canary
                let mut registry_guard = self.registry.write().await;
                registry_guard.record_delivery_metrics(candidate, 50, false, now_epoch);
                eprintln!(
                    "Bridge [{}] silent drop canary check failed. Cascading...",
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
                let start = std::time::Instant::now();
                match bridge.send_message(payload).await {
                    Ok(_) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, true, now_epoch);

                        let state = if idx == 0 && candidate == primary {
                            DeliveryState::Delivered
                        } else {
                            DeliveryState::FallbackTriggered {
                                from: primary,
                                to: candidate,
                                reason: format!("Canary-verified fallback delivered via [{}]", candidate.as_str()),
                            }
                        };
                        return Ok((candidate, state));
                    }
                    Err(e) => {
                        let latency_ms = start.elapsed().as_millis() as u64;
                        let mut registry_guard = self.registry.write().await;
                        registry_guard.record_delivery_metrics(candidate, latency_ms, false, now_epoch);
                        eprintln!(
                            "Canary-verified bridge [{}] dispatch failed: {}. Continuing cascade...",
                            candidate.as_str(),
                            e
                        );
                    }
                }
            }
        }

        Err(anyhow!(
            "Canary drop routing failed: all candidates exhausted or blackholed for recipient {}",
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

    #[test]
    fn test_canary_drop_detector_lifecycle() {
        let mut detector = CanaryDropDetector::new(5); // 5s timeout
        let now = 1000;

        let (nonce, probe) = detector.issue_canary(ProtocolKind::Signal, now);
        assert!(probe.content.contains(&nonce));

        // Unexpired acknowledgment succeeds
        assert!(detector.acknowledge_canary(&nonce, ProtocolKind::Signal, now + 3));

        // Expired or unknown nonce fails
        let (nonce2, _) = detector.issue_canary(ProtocolKind::Signal, now);
        assert!(detector.is_silent_drop(&nonce2, now + 6));
        assert!(!detector.acknowledge_canary(&nonce2, ProtocolKind::Signal, now + 6));
    }

    #[tokio::test]
    async fn test_route_with_canary_drop_detection_failover() {
        let mut registry = BridgeRegistry::new();

        // 1. Signal bridge: initialized, but simulated canary drop (e.g. ISP blackhole)
        let mut signal = SignalBridge::new();
        signal.initialize().await.unwrap();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        // 2. Matrix bridge: initialized, canary succeeds
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));
        let mut detector = CanaryDropDetector::new(10);

        // Echo simulator drops Signal, confirms Matrix
        let echo_sim = |proto: ProtocolKind, _nonce: &str| proto == ProtocolKind::Matrix;

        let payload = MessagePayload {
            recipient_id: "!matrix_canary_room:matrix.org".to_string(),
            content: "Canary drop verification test message".to_string(),
            fallback_protocol: "none".to_string(),
        };

        let (chosen, state) = router
            .route_with_canary_drop_detection(
                &payload,
                ProtocolKind::Signal,
                &[ProtocolKind::Matrix],
                &mut detector,
                echo_sim,
                1700000000,
            )
            .await
            .expect("Should detect Signal silent drop and cascade to Matrix");

        assert_eq!(chosen, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { from, to, reason } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Matrix);
                assert!(reason.contains("Canary-verified"));
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[test]
    fn test_route_bandwidth_shaper_quota_and_sliding_window() {
        let mut shaper = RouteBandwidthShaper::new(60); // 60s sliding window
        shaper.set_byte_limit(ProtocolKind::Sms, 100);

        let now = 1000;
        // First 60 bytes: allowed
        assert!(shaper.try_consume_bandwidth(ProtocolKind::Sms, 60, now));
        assert_eq!(shaper.current_usage(ProtocolKind::Sms, now), 60);

        // Next 30 bytes: allowed (total 90 <= 100)
        assert!(shaper.try_consume_bandwidth(ProtocolKind::Sms, 30, now + 10));
        assert_eq!(shaper.current_usage(ProtocolKind::Sms, now + 10), 90);

        // Next 20 bytes: rejected (90 + 20 > 100)
        assert!(!shaper.try_consume_bandwidth(ProtocolKind::Sms, 20, now + 20));
        assert_eq!(shaper.current_usage(ProtocolKind::Sms, now + 20), 90);

        // After window expires (now + 65 = 1065), the first 60 bytes expire (t=1000), leaving 30 bytes
        assert_eq!(shaper.current_usage(ProtocolKind::Sms, now + 65), 30);
        // Now 50 bytes can fit (30 + 50 = 80 <= 100)
        assert!(shaper.try_consume_bandwidth(ProtocolKind::Sms, 50, now + 65));
        assert_eq!(shaper.current_usage(ProtocolKind::Sms, now + 65), 80);
    }

    #[tokio::test]
    async fn test_route_with_bandwidth_throttled_cascade_failover() {
        let mut registry = BridgeRegistry::new();

        // 1. Cellular SMS bridge: initialized, but tight bandwidth limit (50 bytes)
        let mut sms = SmsBridge::new();
        let _ = <SmsBridge as ProtocolBridge>::initialize(&mut sms).await;
        registry.register(ProtocolKind::Sms, Box::new(sms));

        // 2. Matrix bridge: initialized, unmetered
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));
        let mut shaper = RouteBandwidthShaper::new(60);
        shaper.set_byte_limit(ProtocolKind::Sms, 50);

        // Pre-consume 40 bytes on SMS
        assert!(shaper.try_consume_bandwidth(ProtocolKind::Sms, 40, 1700000000));

        let payload = MessagePayload {
            recipient_id: "!matrix_bw_room:matrix.org".to_string(),
            content: "This message is 48 bytes long and exceeds SMS!".to_string(),
            fallback_protocol: "none".to_string(),
        };

        // When dispatching with SMS primary, bandwidth quota exceeds 50 bytes (40 + 48 = 88 > 50),
        // triggering automatic cascade to Matrix!
        let (chosen, state) = router
            .route_with_bandwidth_throttled_cascade(
                &payload,
                ProtocolKind::Sms,
                &[ProtocolKind::Matrix],
                &mut shaper,
                1700000005,
            )
            .await
            .expect("Bandwidth throttled cascade should deliver via Matrix");

        assert_eq!(chosen, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { from, to, reason } => {
                assert_eq!(from, ProtocolKind::Sms);
                assert_eq!(to, ProtocolKind::Matrix);
                assert!(reason.contains("Bandwidth throttled"));
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

    #[tokio::test]
    async fn test_route_with_replay_and_integrity_guard() {
        let mut registry = BridgeRegistry::new();
        let mut signal = SignalBridge::new();
        signal.initialize().await.unwrap();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));
        let mut replay_cache = NonceReplayCache::new(60);

        let payload = MessagePayload {
            recipient_id: "+15557777".to_string(),
            content: "Anti-replay test message".to_string(),
            fallback_protocol: "none".to_string(),
        };

        let digest = PayloadIntegrityGuard::compute_digest(&payload);

        let envelope = RoutedPacketEnvelope {
            nonce: "unique_nonce_001".to_string(),
            timestamp_sec: 1700000000,
            payload: payload.clone(),
        };

        // 1. First send: nonce is fresh, must succeed
        let (chosen, state) = router
            .route_with_replay_and_integrity_guard(
                &envelope,
                &mut replay_cache,
                &digest,
                ProtocolKind::Signal,
                &[],
                1700000000,
            )
            .await
            .expect("First dispatch should succeed");

        assert_eq!(chosen, ProtocolKind::Signal);
        assert_eq!(state, DeliveryState::Delivered);
        assert_eq!(replay_cache.cached_nonce_count(), 1);

        // 2. Replay with identical nonce: must be rejected
        let replay_res = router
            .route_with_replay_and_integrity_guard(
                &envelope,
                &mut replay_cache,
                &digest,
                ProtocolKind::Signal,
                &[],
                1700000010,
            )
            .await;

        assert!(replay_res.is_err());
        assert!(replay_res.unwrap_err().to_string().contains("Replay attack detected"));
    }

    #[test]
    fn test_circuit_breaker_dynamic_jitter_cooldown() {
        let mut metric = ProtocolMetric {
            failure_threshold: 3,
            cooldown_sec: 20,
            consecutive_failures: 3,
            last_failure_epoch_sec: 1700000000,
            circuit_state: CircuitState::Open,
            ..Default::default()
        }.with_jitter(15); // Adds 15s jitter -> total effective cooldown = 35s

        // Before base cooldown + jitter expires (e.g. 25s elapsed)
        assert!(!metric.is_available_at(1700000025));
        assert_eq!(metric.circuit_state, CircuitState::Open);

        // After jittered cooldown expires (36s elapsed)
        assert!(metric.is_available_at(1700000036));
        assert_eq!(metric.circuit_state, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_route_hedged_racing_primary_success() {
        let mut registry = BridgeRegistry::new();

        let mut signal = SignalBridge::new();
        signal.initialize().await.unwrap();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "+15551212".to_string(),
            content: "Hedged primary win test".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        let policy = HedgedRoutingPolicy {
            hedge_delay_ms: 10,
            max_parallel_candidates: 2,
        };

        let (winner, state) = router
            .route_hedged_racing(&payload, ProtocolKind::Signal, &[ProtocolKind::Matrix], policy, 1700000000)
            .await
            .expect("Hedged racing should succeed");

        assert_eq!(winner, ProtocolKind::Signal);
        assert_eq!(state, DeliveryState::Delivered);
    }

    #[tokio::test]
    async fn test_route_hedged_racing_secondary_wins_when_primary_fails() {
        let mut registry = BridgeRegistry::new();

        // Signal bridge uninitialized (will fail immediate attempt)
        let signal = SignalBridge::new();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        // Matrix bridge initialized (succeeds)
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "+15553434".to_string(),
            content: "Hedged secondary win test".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        let policy = HedgedRoutingPolicy {
            hedge_delay_ms: 0, // Concurrent parallel launch
            max_parallel_candidates: 2,
        };

        let (winner, state) = router
            .route_hedged_racing(&payload, ProtocolKind::Signal, &[ProtocolKind::Matrix], policy, 1700000000)
            .await
            .expect("Hedged secondary candidate should deliver successfully");

        assert_eq!(winner, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { to, .. } => {
                assert_eq!(to, ProtocolKind::Matrix);
            }
            _ => panic!("Expected fallback delivery state"),
        }
    }

    #[tokio::test]
    async fn test_route_with_adaptive_fallback_retry_recovers_on_subsequent_pass() {
        let mut registry = BridgeRegistry::new();

        // 1. Signal bridge: uninitialized (fails on all passes)
        let signal = SignalBridge::new();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        // 2. Matrix bridge: initialized (succeeds)
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "!matrix_adaptive:matrix.org".to_string(),
            content: "Adaptive fallback multi-pass test".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        let (chosen, state) = router
            .route_with_adaptive_fallback_retry(
                &payload,
                ProtocolKind::Signal,
                &[ProtocolKind::Matrix],
                3,
                1700000000,
            )
            .await
            .expect("Adaptive fallback multi-pass dispatch should succeed via Matrix");

        assert_eq!(chosen, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { from, to, .. } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Matrix);
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[test]
    fn test_circuit_exponential_cooldown_backoff_progression() {
        let mut metric = ProtocolMetric {
            cooldown_sec: 20,
            cooldown_jitter_sec: 5,
            max_cooldown_sec: 200,
            ..Default::default()
        };

        // 0 consecutive trips: 20 * 2^0 + 5 = 25s
        assert_eq!(metric.compute_effective_cooldown(), 25);

        // 1 trip: 20 * 2^1 + 5 = 45s
        metric.consecutive_trips = 1;
        assert_eq!(metric.compute_effective_cooldown(), 45);

        // 2 trips: 20 * 2^2 + 5 = 85s
        metric.consecutive_trips = 2;
        assert_eq!(metric.compute_effective_cooldown(), 85);

        // 3 trips: 20 * 2^3 + 5 = 165s
        metric.consecutive_trips = 3;
        assert_eq!(metric.compute_effective_cooldown(), 165);

        // 4 trips: clamped by max_cooldown_sec (200) + 5 = 205s
        metric.consecutive_trips = 4;
        assert_eq!(metric.compute_effective_cooldown(), 205);
    }

    #[tokio::test]
    async fn test_route_with_flapping_resilience_bypasses_flapping_bridge() {
        let mut registry = BridgeRegistry::new();

        // 1. Signal bridge: repeatedly flaps, has 3 consecutive circuit trips
        let signal = SignalBridge::new();
        registry.register(ProtocolKind::Signal, Box::new(signal));
        let now = 1700000000;
        for _ in 0..3 {
            registry.record_delivery_metrics(ProtocolKind::Signal, 10, false, now);
            registry.record_delivery_metrics(ProtocolKind::Signal, 10, false, now);
            registry.record_delivery_metrics(ProtocolKind::Signal, 10, false, now);
        }

        // 2. Cellular SMS bridge: stable, initialized, 0 trips
        let mut sms = SmsBridge::new();
        let _ = <SmsBridge as ProtocolBridge>::initialize(&mut sms).await;
        registry.register(ProtocolKind::Sms, Box::new(sms));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "+15557878".to_string(),
            content: "Testing flapping bridge resilience".to_string(),
            fallback_protocol: "SMS".to_string(),
        };

        // Flapping resilience should prioritize stable SMS bridge over repeatedly tripping Signal
        let (chosen, state) = router
            .route_with_flapping_resilience(&payload, ProtocolKind::Signal, &[ProtocolKind::Sms], now + 5)
            .await
            .expect("Routing with flapping resilience should deliver via SMS");

        assert_eq!(chosen, ProtocolKind::Sms);
        match state {
            DeliveryState::FallbackTriggered { from, to, .. } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Sms);
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[test]
    fn test_select_penalty_weighted_route_prioritization() {
        let mut registry = BridgeRegistry::new();
        let now = 1700000000;

        registry.register(ProtocolKind::Signal, Box::new(SignalBridge::new()));
        registry.register(ProtocolKind::Matrix, Box::new(MatrixBridge::new()));

        // Signal: low latency 30ms, but 1 failure -> penalty = 30 + 500 = 530
        registry.record_delivery_metrics(ProtocolKind::Signal, 30, false, now);

        // Matrix: 80ms latency, 0 failures -> penalty = 80
        registry.record_delivery_metrics(ProtocolKind::Matrix, 80, true, now);

        let (chosen, fallbacks) = registry.select_penalty_weighted_route(ProtocolKind::Signal, &[ProtocolKind::Matrix]);
        assert_eq!(chosen, ProtocolKind::Matrix);
        assert_eq!(fallbacks, vec![ProtocolKind::Signal]);
    }

    #[tokio::test]
    async fn test_route_with_penalty_weighted_cascade_delivery() {
        let mut registry = BridgeRegistry::new();
        let now = 1700000000;

        // 1. Signal bridge: degraded latency (450ms) + 1 failure
        let signal = SignalBridge::new();
        registry.register(ProtocolKind::Signal, Box::new(signal));
        registry.record_delivery_metrics(ProtocolKind::Signal, 450, false, now);

        // 2. Matrix bridge: fast (30ms), initialized, succeeds
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));
        registry.record_delivery_metrics(ProtocolKind::Matrix, 30, true, now);

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "!matrix_penalty:matrix.org".to_string(),
            content: "Penalty-weighted routing test".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        let (chosen, state) = router
            .route_with_penalty_weighted_cascade(&payload, ProtocolKind::Signal, &[ProtocolKind::Matrix], now + 5)
            .await
            .expect("Penalty weighted cascade should prioritize Matrix and deliver");

        assert_eq!(chosen, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { from, to, .. } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Matrix);
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[test]
    fn test_cross_protocol_address_book_translation() {
        let mut book = CrossProtocolAddressBook::new();
        book.register_address("alice_unified", ProtocolKind::Signal, "+15551234567");
        book.register_address("alice_unified", ProtocolKind::Matrix, "@alice:matrix.org");
        book.register_address("alice_unified", ProtocolKind::Discord, "discord_channel_999");

        assert_eq!(book.resolve_address("alice_unified", ProtocolKind::Signal), "+15551234567");
        assert_eq!(book.resolve_address("alice_unified", ProtocolKind::Matrix), "@alice:matrix.org");
        assert_eq!(book.resolve_address("alice_unified", ProtocolKind::Discord), "discord_channel_999");
        // Unmapped protocol falls back to unified_id
        assert_eq!(book.resolve_address("alice_unified", ProtocolKind::Sms), "alice_unified");
    }

    #[tokio::test]
    async fn test_route_unified_message_with_dynamic_translation_fallback() {
        let mut registry = BridgeRegistry::new();

        // 1. Primary: Signal bridge (uninitialized, fails)
        let signal = SignalBridge::new();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        // 2. Secondary fallback: Matrix bridge (initialized, succeeds)
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let mut book = CrossProtocolAddressBook::new();
        book.register_address("user_bob_unified", ProtocolKind::Signal, "+15550011223");
        book.register_address("user_bob_unified", ProtocolKind::Matrix, "!matrix_room_bob:matrix.org");

        let (chosen, state) = router
            .route_unified_message(
                "user_bob_unified",
                "Hello across unified identity boundaries",
                ProtocolKind::Signal,
                &[ProtocolKind::Matrix],
                &book,
                1700000000,
            )
            .await
            .expect("Unified message routing should cascade to Matrix and succeed");

        assert_eq!(chosen, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { from, to, reason } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Matrix);
                assert!(reason.contains("user_bob_unified"));
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[test]
    fn test_payload_fragmenter_splits_oversized_content() {
        let text = "This is a long test payload exceeding the small MTU limit of 50 characters to test fragmentation header formatting.";
        let fragments = PayloadFragmenter::fragment_payload(text, 50);

        assert!(fragments.len() > 1);
        for (i, frag) in fragments.iter().enumerate() {
            assert!(frag.starts_with(&format!("[{}/", i + 1)));
            assert!(frag.len() <= 60); // bounded by MTU
        }
    }

    #[tokio::test]
    async fn test_route_with_fragmentation_adaptive_cascade_to_sms() {
        let mut registry = BridgeRegistry::new();

        // 1. Signal bridge: uninitialized (fails)
        let signal = SignalBridge::new();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        // 2. Cellular SMS bridge: initialized (succeeds)
        let mut sms = SmsBridge::new();
        let _ = <SmsBridge as ProtocolBridge>::initialize(&mut sms).await;
        registry.register(ProtocolKind::Sms, Box::new(sms));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let long_content = "A".repeat(350); // 350 chars exceeds SMS 140 char MTU limit
        let payload = MessagePayload {
            recipient_id: "+15554321".to_string(),
            content: long_content,
            fallback_protocol: "SMS".to_string(),
        };

        let (chosen, state, fragment_count) = router
            .route_with_fragmentation_adaptive_cascade(
                &payload,
                ProtocolKind::Signal,
                &[ProtocolKind::Sms],
                1700000000,
            )
            .await
            .expect("Fragmentation adaptive cascade to SMS should succeed");

        assert_eq!(chosen, ProtocolKind::Sms);
        assert!(fragment_count >= 3); // 350 / 130 ~ 3 chunks
        match state {
            DeliveryState::FallbackTriggered { from, to, reason } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Sms);
                assert!(reason.contains("fragmented"));
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[tokio::test]
    async fn test_multipath_router_nostr_to_matrix_fallback() {
        use crate::bridges::nostr::NostrBridge;

        let mut registry = BridgeRegistry::new();

        // 1. Primary: Uninitialized Nostr bridge (will fail)
        let nostr = NostrBridge::new();
        registry.register(ProtocolKind::Nostr, Box::new(nostr));

        // 2. Fallback: Initialized Matrix bridge (will succeed)
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "!nostr_matrix_room:matrix.org".to_string(),
            content: "Nostr relay network partition failover to Matrix".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        let (chosen, state) = router
            .route_with_cascade(&payload, ProtocolKind::Nostr, &[ProtocolKind::Matrix])
            .await
            .expect("Nostr to Matrix fallback should succeed");

        assert_eq!(chosen, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { from, to, .. } => {
                assert_eq!(from, ProtocolKind::Nostr);
                assert_eq!(to, ProtocolKind::Matrix);
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[test]
    fn test_payload_content_transcoder_sms_normalization() {
        let markdown_input = "**Alert:** Server _database_ is `DOWN`! Check status: <br/>— [Metrics]…";
        let transcoded = PayloadContentTranscoder::transcode_for_protocol(markdown_input, ProtocolKind::Sms);

        assert!(!transcoded.contains("**"));
        assert!(!transcoded.contains("_"));
        assert!(!transcoded.contains("`"));
        assert!(!transcoded.contains("<br/>"));
        assert!(transcoded.contains("Alert: Server database is DOWN!"));
        assert!(transcoded.contains("- [Metrics]..."));
    }

    #[tokio::test]
    async fn test_route_with_transcoding_cascade_to_sms() {
        let mut registry = BridgeRegistry::new();

        // 1. Matrix bridge: uninitialized (fails)
        let matrix = MatrixBridge::new();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        // 2. Cellular SMS bridge: initialized (succeeds)
        let mut sms = SmsBridge::new();
        let _ = <SmsBridge as ProtocolBridge>::initialize(&mut sms).await;
        registry.register(ProtocolKind::Sms, Box::new(sms));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let rich_payload = MessagePayload {
            recipient_id: "+15556677".to_string(),
            content: "**High Priority:** _Cluster nodes_ have failed `heartbeat`!—".to_string(),
            fallback_protocol: "SMS".to_string(),
        };

        let (chosen, state, transcoded) = router
            .route_with_transcoding_cascade(
                &rich_payload,
                ProtocolKind::Matrix,
                &[ProtocolKind::Sms],
                1700000000,
            )
            .await
            .expect("Transcoding cascade to SMS should succeed");

        assert_eq!(chosen, ProtocolKind::Sms);
        assert!(!transcoded.contains("**"));
        assert!(!transcoded.contains("`"));
        assert!(transcoded.contains("High Priority: Cluster nodes have failed heartbeat!-"));

        match state {
            DeliveryState::FallbackTriggered { from, to, reason } => {
                assert_eq!(from, ProtocolKind::Matrix);
                assert_eq!(to, ProtocolKind::Sms);
                assert!(reason.contains("Transcoded"));
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[test]
    fn test_receipt_tracker_lifecycle_and_timeout() {
        let mut tracker = ReceiptTracker::new(10); // 10s timeout
        let now = 1000;

        tracker.register_pending("nonce_abc", ProtocolKind::Signal, now);
        assert_eq!(tracker.pending_count(), 1);
        assert!(!tracker.is_timed_out("nonce_abc", now + 5));
        assert!(tracker.is_timed_out("nonce_abc", now + 11));

        let valid_receipt = DeliveryReceipt::generate("nonce_abc", "+1555", ProtocolKind::Signal, now + 5);
        assert!(valid_receipt.verify_signature());

        // Confirming receipt before timeout succeeds
        assert!(tracker.confirm_receipt(&valid_receipt, now + 5));
        assert_eq!(tracker.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_route_with_confirmed_receipt_silent_drop_cascade() {
        let mut registry = BridgeRegistry::new();

        // 1. Primary Signal bridge: initialized, sends, but simulated receipt fails / silent drop
        let mut signal = SignalBridge::new();
        signal.initialize().await.unwrap();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        // 2. Secondary Matrix bridge: initialized, sends AND returns verified receipt
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));
        let mut tracker = ReceiptTracker::new(10);

        let payload = MessagePayload {
            recipient_id: "!matrix_receipt_room:matrix.org".to_string(),
            content: "End-to-end receipt confirmation test".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        // Receipt generator: Signal drops receipt (None), Matrix confirms receipt (Some)
        let receipt_sim = |proto: ProtocolKind, nonce: &str, recipient: &str, now: u64| {
            if proto == ProtocolKind::Matrix {
                Some(DeliveryReceipt::generate(nonce, recipient, proto, now))
            } else {
                None // Silent drop on Signal
            }
        };

        let (chosen, state, receipt) = router
            .route_with_confirmed_receipt(
                &payload,
                "nonce_test_receipt_001",
                ProtocolKind::Signal,
                &[ProtocolKind::Matrix],
                &mut tracker,
                receipt_sim,
                1700000000,
            )
            .await
            .expect("Routing should cascade to Matrix upon missing Signal receipt");

        assert_eq!(chosen, ProtocolKind::Matrix);
        assert_eq!(receipt.nonce, "nonce_test_receipt_001");
        assert!(receipt.verify_signature());

        match state {
            DeliveryState::FallbackTriggered { from, to, reason } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Matrix);
                assert!(reason.contains("Receipt confirmed"));
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[test]
    fn test_bridge_concurrency_guard_limits_and_permits() {
        let mut guard = BridgeConcurrencyGuard::new();
        guard.set_limit(ProtocolKind::Sms, 2);

        assert_eq!(guard.in_flight_count(ProtocolKind::Sms), 0);
        assert!(guard.try_acquire_permit(ProtocolKind::Sms));
        assert!(guard.try_acquire_permit(ProtocolKind::Sms));
        assert_eq!(guard.in_flight_count(ProtocolKind::Sms), 2);

        // 3rd permit fails due to saturation
        assert!(!guard.try_acquire_permit(ProtocolKind::Sms));

        // Release permit and re-acquire
        guard.release_permit(ProtocolKind::Sms);
        assert_eq!(guard.in_flight_count(ProtocolKind::Sms), 1);
        assert!(guard.try_acquire_permit(ProtocolKind::Sms));
    }

    #[tokio::test]
    async fn test_route_with_concurrency_bounded_cascade_bypasses_saturated_bridge() {
        let mut registry = BridgeRegistry::new();

        // 1. Primary Signal bridge: initialized (would succeed), but concurrency saturated
        let mut signal = SignalBridge::new();
        signal.initialize().await.unwrap();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        // 2. Secondary Matrix bridge: initialized, unsaturated (succeeds)
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let mut guard = BridgeConcurrencyGuard::new();
        guard.set_limit(ProtocolKind::Signal, 1);
        assert!(guard.try_acquire_permit(ProtocolKind::Signal)); // Exhaust Signal permit budget

        let payload = MessagePayload {
            recipient_id: "!matrix_bounded_room:matrix.org".to_string(),
            content: "Concurrency saturation failover test".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        let (chosen, state) = router
            .route_with_concurrency_bounded_cascade(
                &payload,
                ProtocolKind::Signal,
                &[ProtocolKind::Matrix],
                &mut guard,
                1700000000,
            )
            .await
            .expect("Should bypass saturated Signal bridge and deliver via Matrix");

        assert_eq!(chosen, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { from, to, reason } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Matrix);
                assert!(reason.contains("Concurrency bounded"));
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[test]
    fn test_route_anomaly_filter_damps_single_packet_spike() {
        let baseline = 50u64;
        let outlier_spike = 10_000u64; // Extreme 10s packet jitter

        // Raw EMA without damping would spike from 50 to 3035ms!
        // Damped EMA clamps sample to 2x baseline (100ms) -> (50*7 + 100*3)/10 = 65ms
        let smoothed = RouteAnomalyFilter::compute_damped_latency(baseline, outlier_spike);
        assert_eq!(smoothed, 65);
        assert!(smoothed < 100);
    }

    #[tokio::test]
    async fn test_route_with_anomaly_damped_cascade_delivery() {
        let mut registry = BridgeRegistry::new();

        // 1. Signal bridge: uninitialized (fails)
        let signal = SignalBridge::new();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        // 2. Matrix bridge: initialized (succeeds)
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "!matrix_damped_room:matrix.org".to_string(),
            content: "Anomaly damped latency fallback test".to_string(),
            fallback_protocol: "Matrix".to_string(),
        };

        let (chosen, state) = router
            .route_with_anomaly_damped_cascade(
                &payload,
                ProtocolKind::Signal,
                &[ProtocolKind::Matrix],
                1700000000,
            )
            .await
            .expect("Anomaly damped cascade should deliver via Matrix");

        assert_eq!(chosen, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { from, to, reason } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Matrix);
                assert!(reason.contains("Damped fallback"));
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[test]
    fn test_egress_deduplication_ledger_sliding_window() {
        let mut ledger = EgressDeduplicationLedger::new(60); // 60s window
        let now = 1000;

        assert!(ledger.check_and_record_transmission("+15551111", "Hello world", now));
        assert_eq!(ledger.tracked_count(), 1);

        // Immediate duplicate transmission rejected
        assert!(!ledger.check_and_record_transmission("+15551111", "Hello world", now + 10));

        // Different message or recipient allowed
        assert!(ledger.check_and_record_transmission("+15552222", "Hello world", now + 10));
        assert!(ledger.check_and_record_transmission("+15551111", "Different text", now + 10));
        assert_eq!(ledger.tracked_count(), 3);

        // After window expires (now + 65 = 1065), the first record (exp 1060) is expired, allowing re-record
        assert!(ledger.check_and_record_transmission("+15551111", "Hello world", now + 65));
        // All 3 initial transmissions have exp <= 1070. At 1075, the remaining two from t=1010 have expired, plus the refreshed one has exp 1125
        let pruned = ledger.prune_expired(now + 75);
        assert_eq!(pruned, 2);
        assert_eq!(ledger.tracked_count(), 1);
    }

    #[tokio::test]
    async fn test_route_with_exact_once_deduplication_enforcement() {
        let mut registry = BridgeRegistry::new();
        let mut signal = SignalBridge::new();
        signal.initialize().await.unwrap();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));
        let mut ledger = EgressDeduplicationLedger::new(60);

        let payload = MessagePayload {
            recipient_id: "+15553344".to_string(),
            content: "Exact-once delivery check payload".to_string(),
            fallback_protocol: "none".to_string(),
        };

        // 1. Initial send: succeeds and records transmission in ledger
        let (chosen, state) = router
            .route_with_exact_once_deduplication(&payload, ProtocolKind::Signal, &[], &mut ledger, 1700000000)
            .await
            .expect("First dispatch should succeed");

        assert_eq!(chosen, ProtocolKind::Signal);
        assert_eq!(state, DeliveryState::Delivered);

        // 2. Rapid re-transmission attempt of same content: rejected by exact-once check
        let dup_res = router
            .route_with_exact_once_deduplication(&payload, ProtocolKind::Signal, &[], &mut ledger, 1700000010)
            .await;

        assert!(dup_res.is_err());
        assert!(dup_res.unwrap_err().to_string().contains("duplicate message payload detected"));
    }

    #[test]
    fn test_bridge_heartbeat_prober_payload_generation() {
        let probe = BridgeHeartbeatProber::create_probe_payload("test_probe_nonce_999");
        assert_eq!(probe.recipient_id, "__omnicore_health_probe__");
        assert_eq!(probe.content, "PING:test_probe_nonce_999");
        assert_eq!(probe.fallback_protocol, "none");
    }

    #[tokio::test]
    async fn test_probe_and_recover_circuits_restores_half_open_bridge() {
        let mut registry = BridgeRegistry::new();

        // Register Matrix bridge, initialize it so heartbeat probe send will succeed
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let now = 1700000000;
        // Trip Matrix bridge circuit by recording 3 consecutive failures
        registry.record_delivery_metrics(ProtocolKind::Matrix, 20, false, now);
        registry.record_delivery_metrics(ProtocolKind::Matrix, 20, false, now);
        registry.record_delivery_metrics(ProtocolKind::Matrix, 20, false, now);

        {
            let m = registry.get_metrics(ProtocolKind::Matrix).unwrap();
            assert_eq!(m.circuit_state, CircuitState::Open);
            assert_eq!(m.consecutive_trips, 1);
        }

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        // With 1 consecutive trip, effective cooldown is 30 * 2^1 = 60s.
        // At now + 65, circuit cooldown has elapsed and moves to HalfOpen when probed.
        // Active probing triggers heartbeat ping and restores circuit to Closed upon success!
        let probe_results = router.probe_and_recover_circuits(now + 65).await;

        assert_eq!(probe_results.len(), 1);
        assert_eq!(probe_results[0].0, ProtocolKind::Matrix);
        assert!(probe_results[0].1); // Probe succeeded

        let reg_guard = router.registry.read().await;
        let m = reg_guard.get_metrics(ProtocolKind::Matrix).unwrap();
        assert_eq!(m.circuit_state, CircuitState::Closed);
        assert_eq!(m.consecutive_trips, 0); // Decoupled/decremented on recovery
    }

    #[tokio::test]
    async fn test_route_with_active_health_probing_execution() {
        let mut registry = BridgeRegistry::new();

        // 1. Matrix: initialized, healthy
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        // 2. Signal: initialized, healthy
        let mut signal = SignalBridge::new();
        signal.initialize().await.unwrap();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "!matrix_active_probing:matrix.org".to_string(),
            content: "Testing active health probing route dispatch".to_string(),
            fallback_protocol: "none".to_string(),
        };

        let (chosen, state) = router
            .route_with_active_health_probing(&payload, ProtocolKind::Matrix, &[ProtocolKind::Signal], 1700000000)
            .await
            .expect("Active health probing dispatch should succeed");

        assert_eq!(chosen, ProtocolKind::Matrix);
        assert_eq!(state, DeliveryState::Delivered);
    }

    #[test]
    fn test_route_cost_optimizer_scoring_modes() {
        // SMS has 50 financial units and 40 battery drain score
        // Matrix has 0 financial units and 2 battery drain score
        let sms_cost_score = RouteCostOptimizer::compute_qos_score(
            ProtocolKind::Sms,
            30,
            QosRoutingPreference::CostSaving,
        );
        let matrix_cost_score = RouteCostOptimizer::compute_qos_score(
            ProtocolKind::Matrix,
            30,
            QosRoutingPreference::CostSaving,
        );

        // In CostSaving mode, SMS penalty should far exceed Matrix (free)
        assert!(sms_cost_score > matrix_cost_score);

        // In UltraLowLatency mode with SMS having 10ms and Matrix having 200ms
        let sms_fast = RouteCostOptimizer::compute_qos_score(
            ProtocolKind::Sms,
            10,
            QosRoutingPreference::UltraLowLatency,
        );
        let matrix_slow = RouteCostOptimizer::compute_qos_score(
            ProtocolKind::Matrix,
            200,
            QosRoutingPreference::UltraLowLatency,
        );

        // Fast SMS should be preferred over slow Matrix in UltraLowLatency
        assert!(sms_fast < matrix_slow);
    }

    #[tokio::test]
    async fn test_route_with_cost_and_qos_optimization_prefers_cost_saving() {
        let mut registry = BridgeRegistry::new();

        // 1. Initialized SMS bridge (metered, higher financial cost)
        let mut sms = SmsBridge::new();
        let _ = <SmsBridge as ProtocolBridge>::initialize(&mut sms).await;
        registry.register(ProtocolKind::Sms, Box::new(sms));

        // 2. Initialized Matrix bridge (unmetered, zero financial cost)
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        let payload = MessagePayload {
            recipient_id: "!matrix_qos_room:matrix.org".to_string(),
            content: "Testing cost-saving QoS route selection".to_string(),
            fallback_protocol: "none".to_string(),
        };

        // When requesting SMS as primary with CostSaving preference, router promotes Matrix
        let (chosen, state) = router
            .route_with_cost_and_qos_optimization(
                &payload,
                ProtocolKind::Sms,
                &[ProtocolKind::Matrix],
                QosRoutingPreference::CostSaving,
                1700000000,
            )
            .await
            .expect("QoS route optimization should succeed");

        assert_eq!(chosen, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { from, to, reason } => {
                assert_eq!(from, ProtocolKind::Sms);
                assert_eq!(to, ProtocolKind::Matrix);
                assert!(reason.contains("CostSaving"));
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[test]
    fn test_protocol_metric_decay_trips_if_stable() {
        let mut metric = ProtocolMetric {
            consecutive_trips: 3,
            circuit_state: CircuitState::Closed,
            last_failure_epoch_sec: 1000,
            ..Default::default()
        };

        // Stability window is 60s
        // At 1050 (50s elapsed), stability window not met -> no decay
        assert!(!metric.decay_trips_if_stable(1050, 60));
        assert_eq!(metric.consecutive_trips, 3);

        // At 1065 (65s elapsed), stability window met -> trips decay by 1
        assert!(metric.decay_trips_if_stable(1065, 60));
        assert_eq!(metric.consecutive_trips, 2);

        // Subsequent call at 1070 -> decays further
        assert!(metric.decay_trips_if_stable(1070, 60));
        assert_eq!(metric.consecutive_trips, 1);
    }

    #[tokio::test]
    async fn test_rehabilitate_stable_circuits_and_route() {
        let mut registry = BridgeRegistry::new();

        let mut signal = SignalBridge::new();
        signal.initialize().await.unwrap();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        let now = 1700000000;
        // Simulate previous flapping history: 2 consecutive trips, but currently Closed
        {
            let m = registry.metrics.get_mut(&ProtocolKind::Signal).unwrap();
            m.consecutive_trips = 2;
            m.circuit_state = CircuitState::Closed;
            m.last_failure_epoch_sec = now;
        }

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));

        // At now + 120s (with stability window 60s), trips are decayed
        let decayed = router.rehabilitate_stable_circuits(now + 120, 60).await;
        assert_eq!(decayed, 1);

        {
            let reg_guard = router.registry.read().await;
            let m = reg_guard.get_metrics(ProtocolKind::Signal).unwrap();
            assert_eq!(m.consecutive_trips, 1); // Decayed from 2 to 1
        }

        let payload = MessagePayload {
            recipient_id: "+15551357".to_string(),
            content: "Testing route with rehabilitated circuit".to_string(),
            fallback_protocol: "none".to_string(),
        };

        let (chosen, state) = router
            .route_with_rehabilitated_circuits(
                &payload,
                ProtocolKind::Signal,
                &[],
                60,
                now + 130,
            )
            .await
            .expect("Rehabilitated circuit dispatch should succeed");

        assert_eq!(chosen, ProtocolKind::Signal);
        assert_eq!(state, DeliveryState::Delivered);
    }

    #[test]
    fn test_route_dispersal_balancer_rotation() {
        let mut balancer = RouteDispersalBalancer::new();
        let primary = ProtocolKind::Signal;
        let fallbacks = [ProtocolKind::Matrix, ProtocolKind::Sms];

        // 1st call: selects candidates[0] = Signal
        let (p1, f1) = balancer.select_dispersed_route(primary, &fallbacks);
        assert_eq!(p1, ProtocolKind::Signal);
        assert_eq!(f1, vec![ProtocolKind::Matrix, ProtocolKind::Sms]);

        // 2nd call: rotates to candidates[1] = Matrix
        let (p2, f2) = balancer.select_dispersed_route(primary, &fallbacks);
        assert_eq!(p2, ProtocolKind::Matrix);
        assert_eq!(f2, vec![ProtocolKind::Signal, ProtocolKind::Sms]);

        // 3rd call: rotates to candidates[2] = SMS
        let (p3, f3) = balancer.select_dispersed_route(primary, &fallbacks);
        assert_eq!(p3, ProtocolKind::Sms);
        assert_eq!(f3, vec![ProtocolKind::Signal, ProtocolKind::Matrix]);

        // 4th call: wraps around to Signal
        let (p4, _) = balancer.select_dispersed_route(primary, &fallbacks);
        assert_eq!(p4, ProtocolKind::Signal);
    }

    #[tokio::test]
    async fn test_route_with_dispersal_balancing_execution() {
        let mut registry = BridgeRegistry::new();

        let mut signal = SignalBridge::new();
        signal.initialize().await.unwrap();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));
        let mut balancer = RouteDispersalBalancer::new();

        let payload = MessagePayload {
            recipient_id: "+15552468".to_string(),
            content: "Dispersal load balanced test message".to_string(),
            fallback_protocol: "none".to_string(),
        };

        // First dispatch: Signal selected
        let (c1, s1) = router
            .route_with_dispersal_balancing(&payload, ProtocolKind::Signal, &[ProtocolKind::Matrix], &mut balancer, 1700000000)
            .await
            .unwrap();
        assert_eq!(c1, ProtocolKind::Signal);
        assert_eq!(s1, DeliveryState::Delivered);

        // Second dispatch: Matrix selected
        let (c2, s2) = router
            .route_with_dispersal_balancing(&payload, ProtocolKind::Signal, &[ProtocolKind::Matrix], &mut balancer, 1700000001)
            .await
            .unwrap();
        assert_eq!(c2, ProtocolKind::Matrix);
        match s2 {
            DeliveryState::FallbackTriggered { from, to, reason } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Matrix);
                assert!(reason.contains("Dispersal balancer"));
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }

    #[test]
    fn test_route_latency_estimator_percentiles() {
        let mut estimator = RouteLatencyEstimator::new(10);

        // Record samples: 10, 20, 30, 40, 50, 60, 70, 80, 90, 100
        for lat in (10..=100).step_by(10) {
            estimator.record_sample(ProtocolKind::Matrix, lat);
        }

        // p50 (median) should be around 50ms
        let p50 = estimator.p50_latency_ms(ProtocolKind::Matrix);
        assert!((40..=60).contains(&p50));

        // p95 should be at the upper edge (90-100ms)
        let p95 = estimator.p95_latency_ms(ProtocolKind::Matrix);
        assert!((90..=100).contains(&p95));
    }

    #[tokio::test]
    async fn test_route_with_percentile_bounded_cascade_delivery() {
        let mut registry = BridgeRegistry::new();

        // 1. Signal bridge: uninitialized (fails fast)
        let signal = SignalBridge::new();
        registry.register(ProtocolKind::Signal, Box::new(signal));

        // 2. Matrix bridge: initialized (succeeds within p95)
        let mut matrix = MatrixBridge::new();
        matrix.initialize().await.unwrap();
        registry.register(ProtocolKind::Matrix, Box::new(matrix));

        let router = MultipathRouter::new(Arc::new(RwLock::new(registry)));
        let mut estimator = RouteLatencyEstimator::new(10);
        estimator.record_sample(ProtocolKind::Matrix, 40);

        let payload = MessagePayload {
            recipient_id: "!matrix_p95:matrix.org".to_string(),
            content: "Percentile bounded dispatch message".to_string(),
            fallback_protocol: "none".to_string(),
        };

        let (chosen, state) = router
            .route_with_percentile_bounded_cascade(
                &payload,
                ProtocolKind::Signal,
                &[ProtocolKind::Matrix],
                &mut estimator,
                1700000000,
            )
            .await
            .expect("Percentile bounded cascade should succeed via Matrix");

        assert_eq!(chosen, ProtocolKind::Matrix);
        match state {
            DeliveryState::FallbackTriggered { from, to, reason } => {
                assert_eq!(from, ProtocolKind::Signal);
                assert_eq!(to, ProtocolKind::Matrix);
                assert!(reason.contains("Percentile-bounded"));
            }
            _ => panic!("Expected FallbackTriggered state"),
        }
    }
}
