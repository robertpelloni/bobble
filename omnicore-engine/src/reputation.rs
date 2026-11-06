//! Decentralized Reputation & Anti-Spam Heuristics
//!
//! Evaluates incoming cross-protocol peer messages to prevent flooding,
//! sybil spam, and untrusted relayed injection in decentralized P2P environments.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Qualitative trust tier based on accumulated score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustTier {
    Untrusted,
    Neutral,
    Vetted,
    Trusted,
    Blacklisted,
}

/// Peer reputation profile tracking behavioral history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerReputation {
    pub peer_id: String,
    pub score: i32,
    pub verified_signatures: u32,
    pub successful_deliveries: u32,
    pub spam_violations: u32,
    pub last_seen_epoch_sec: u64,
}

impl PeerReputation {
    pub fn new(peer_id: &str, current_epoch: u64) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            score: 0,
            verified_signatures: 0,
            successful_deliveries: 0,
            spam_violations: 0,
            last_seen_epoch_sec: current_epoch,
        }
    }

    pub fn trust_tier(&self) -> TrustTier {
        if self.score <= -50 || self.spam_violations >= 5 {
            TrustTier::Blacklisted
        } else if self.score < 0 {
            TrustTier::Untrusted
        } else if self.score < 25 {
            TrustTier::Neutral
        } else if self.score < 75 {
            TrustTier::Vetted
        } else {
            TrustTier::Trusted
        }
    }
}

/// Anti-spam heuristic rules and thresholds.
#[derive(Debug, Clone)]
pub struct SpamHeuristicFilter {
    pub max_messages_per_window: u32,
    pub window_seconds: u64,
    pub penalty_per_violation: i32,
    pub reward_verified_signature: i32,
    pub reward_delivery_ack: i32,
}

impl Default for SpamHeuristicFilter {
    fn default() -> Self {
        Self {
            max_messages_per_window: 10,
            window_seconds: 60,
            penalty_per_violation: 20,
            reward_verified_signature: 5,
            reward_delivery_ack: 2,
        }
    }
}

/// Engine managing peer reputations and real-time rate limiting.
#[derive(Debug)]
pub struct ReputationEngine {
    filter: SpamHeuristicFilter,
    reputations: HashMap<String, PeerReputation>,
    message_timestamps: HashMap<String, Vec<u64>>,
}

impl ReputationEngine {
    pub fn new(filter: SpamHeuristicFilter) -> Self {
        Self {
            filter,
            reputations: HashMap::new(),
            message_timestamps: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, peer_id: &str, current_epoch: u64) -> &mut PeerReputation {
        self.reputations
            .entry(peer_id.to_string())
            .or_insert_with(|| PeerReputation::new(peer_id, current_epoch))
    }

    /// Evaluates if an incoming message from a peer is accepted or marked as spam.
    pub fn evaluate_message(&mut self, peer_id: &str, current_epoch: u64, has_valid_sig: bool) -> bool {
        let max_msgs = self.filter.max_messages_per_window;
        let window = self.filter.window_seconds;
        let penalty = self.filter.penalty_per_violation;
        let reward_sig = self.filter.reward_verified_signature;

        // 1. Evaluate rate limit timestamps first
        let count = {
            let timestamps = self
                .message_timestamps
                .entry(peer_id.to_string())
                .or_default();
            let cutoff = current_epoch.saturating_sub(window);
            timestamps.retain(|&ts| ts >= cutoff);
            timestamps.len() as u32
        };

        // 2. Fetch and evaluate reputation
        let rep = self.get_or_create(peer_id, current_epoch);
        rep.last_seen_epoch_sec = current_epoch;
        if rep.trust_tier() == TrustTier::Blacklisted {
            return false;
        }

        if count >= max_msgs {
            rep.spam_violations += 1;
            rep.score -= penalty;
            return false;
        }

        // 3. Record valid message timestamp and rewards
        self.message_timestamps
            .entry(peer_id.to_string())
            .or_default()
            .push(current_epoch);

        let rep = self.get_or_create(peer_id, current_epoch);
        if has_valid_sig {
            rep.verified_signatures += 1;
            rep.score += reward_sig;
        }

        true
    }

    /// Records a successful delivery acknowledgment from this peer.
    pub fn record_delivery_ack(&mut self, peer_id: &str, current_epoch: u64) {
        let reward = self.filter.reward_delivery_ack;
        let rep = self.get_or_create(peer_id, current_epoch);
        rep.successful_deliveries += 1;
        rep.score += reward;
    }

    pub fn get_reputation(&self, peer_id: &str) -> Option<&PeerReputation> {
        self.reputations.get(peer_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_reputation_tiers() {
        let mut rep = PeerReputation::new("peer_alice", 1700000000);
        assert_eq!(rep.trust_tier(), TrustTier::Neutral);

        rep.score = 30;
        assert_eq!(rep.trust_tier(), TrustTier::Vetted);

        rep.score = 80;
        assert_eq!(rep.trust_tier(), TrustTier::Trusted);

        rep.score = -10;
        assert_eq!(rep.trust_tier(), TrustTier::Untrusted);

        rep.score = -60;
        assert_eq!(rep.trust_tier(), TrustTier::Blacklisted);

        rep.score = 100;
        rep.spam_violations = 5;
        assert_eq!(rep.trust_tier(), TrustTier::Blacklisted);
    }

    #[test]
    fn test_reputation_engine_flow_and_spam_mitigation() {
        let filter = SpamHeuristicFilter {
            max_messages_per_window: 3,
            window_seconds: 10,
            penalty_per_violation: 15,
            reward_verified_signature: 5,
            reward_delivery_ack: 2,
        };
        let mut engine = ReputationEngine::new(filter);
        let now = 1700000000;

        // Message 1 (valid sig)
        assert!(engine.evaluate_message("peer_bob", now, true));
        // Message 2 (valid sig)
        assert!(engine.evaluate_message("peer_bob", now + 1, true));
        // Message 3 (valid sig)
        assert!(engine.evaluate_message("peer_bob", now + 2, true));

        let rep = engine.get_reputation("peer_bob").unwrap();
        assert_eq!(rep.verified_signatures, 3);
        assert_eq!(rep.score, 15);

        // Message 4 within window triggers rate limit violation
        assert!(!engine.evaluate_message("peer_bob", now + 3, true));
        let rep_after = engine.get_reputation("peer_bob").unwrap();
        assert_eq!(rep_after.spam_violations, 1);
        assert_eq!(rep_after.score, 0); // 15 - 15 = 0

        // Delivery ack rewards
        engine.record_delivery_ack("peer_bob", now + 4);
        assert_eq!(engine.get_reputation("peer_bob").unwrap().score, 2);
    }
}
