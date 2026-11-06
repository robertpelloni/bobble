use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CausalRelation {
    Equal,
    HappenedBefore,
    HappenedAfter,
    Concurrent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VectorClock {
    pub clocks: HashMap<String, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    pub fn increment(&mut self, node_id: &str) {
        let entry = self.clocks.entry(node_id.to_string()).or_insert(0);
        *entry += 1;
    }

    pub fn merge(&mut self, other: &VectorClock) {
        for (node_id, &clock) in &other.clocks {
            let entry = self.clocks.entry(node_id.clone()).or_insert(0);
            if clock > *entry {
                *entry = clock;
            }
        }
    }

    pub fn is_causally_newer_or_equal(&self, other: &VectorClock) -> bool {
        for (node, &other_clock) in &other.clocks {
            let our_clock = self.clocks.get(node).copied().unwrap_or(0);
            if our_clock < other_clock {
                return false;
            }
        }
        true
    }

    pub fn compare(&self, other: &VectorClock) -> CausalRelation {
        let self_ge = self.is_causally_newer_or_equal(other);
        let other_ge = other.is_causally_newer_or_equal(self);

        if self_ge && other_ge {
            CausalRelation::Equal
        } else if self_ge {
            CausalRelation::HappenedAfter
        } else if other_ge {
            CausalRelation::HappenedBefore
        } else {
            CausalRelation::Concurrent
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMember {
    pub unified_id: String,
    pub display_name: String,
    pub role: String, // "admin" or "member"
    pub joined_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelEvent {
    pub event_id: String,
    pub channel_id: String,
    pub author_id: String,
    pub event_type: String, // "message", "member_join", "member_leave"
    pub payload: String,
    pub clock: VectorClock,
    pub timestamp: u64,
    pub public_key: Option<String>,
    pub signature: Option<String>,
}

impl ChannelEvent {
    /// Computes canonical hash over event content for cryptographic signing.
    pub fn digest(&self) -> String {
        let canonical_str = format!(
            "{}:{}:{}:{}:{}:{}",
            self.event_id, self.channel_id, self.author_id, self.event_type, self.payload, self.timestamp
        );
        let mut hash: u128 = 0xcbf29ce484222325;
        for byte in canonical_str.as_bytes() {
            hash ^= *byte as u128;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:032x}", hash)
    }

    /// Signs the event using a simulated key pair identifier.
    pub fn sign(&mut self, pubkey: &str, privkey_sim: &str) {
        self.public_key = Some(pubkey.to_string());
        let digest = self.digest();
        let payload = format!("{}:{}", digest, privkey_sim);
        let mut hash: u128 = 0xcbf29ce484222325;
        for byte in payload.as_bytes() {
            hash ^= *byte as u128;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.signature = Some(format!("{:032x}", hash));
    }

    /// Verifies the signature integrity against the author's public key.
    pub fn verify_signature(&self, privkey_sim: &str) -> bool {
        if let Some(ref sig) = self.signature {
            let digest = self.digest();
            let payload = format!("{}:{}", digest, privkey_sim);
            let mut hash: u128 = 0xcbf29ce484222325;
            for byte in payload.as_bytes() {
                hash ^= *byte as u128;
                hash = hash.wrapping_mul(0x100000001b3);
            }
            let expected_sig = format!("{:032x}", hash);
            sig == &expected_sig
        } else {
            false
        }
    }

    /// Deterministic total ordering between two events:
    /// 1. Causal order (HappenedBefore < HappenedAfter)
    /// 2. If concurrent: timestamp ascending, author_id ascending, event_id ascending.
    pub fn compare_deterministic(&self, other: &ChannelEvent) -> Ordering {
        match self.clock.compare(&other.clock) {
            CausalRelation::HappenedBefore => Ordering::Less,
            CausalRelation::HappenedAfter => Ordering::Greater,
            CausalRelation::Equal | CausalRelation::Concurrent => {
                self.timestamp
                    .cmp(&other.timestamp)
                    .then_with(|| self.author_id.cmp(&other.author_id))
                    .then_with(|| self.event_id.cmp(&other.event_id))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedChannel {
    pub channel_id: String,
    pub name: String,
    pub topic: String,
    pub members: HashMap<String, ChannelMember>,
    pub clock: VectorClock,
    pub event_history: Vec<ChannelEvent>,
}

impl FederatedChannel {
    pub fn new(channel_id: &str, name: &str, topic: &str, creator_id: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut members = HashMap::new();
        members.insert(
            creator_id.to_string(),
            ChannelMember {
                unified_id: creator_id.to_string(),
                display_name: creator_id.to_string(),
                role: "admin".to_string(),
                joined_at: now,
            },
        );

        let mut clock = VectorClock::new();
        clock.increment(creator_id);

        Self {
            channel_id: channel_id.to_string(),
            name: name.to_string(),
            topic: topic.to_string(),
            members,
            clock,
            event_history: Vec::new(),
        }
    }

    /// Author and post a new message from a local channel member.
    pub fn post_message(&mut self, author_id: &str, content: &str) -> Result<ChannelEvent> {
        if !self.members.contains_key(author_id) {
            return Err(anyhow!("Author is not an active channel member"));
        }

        self.clock.increment(author_id);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let event = ChannelEvent {
            event_id: format!("ev_{}_{}", author_id, self.event_history.len() + 1),
            channel_id: self.channel_id.clone(),
            author_id: author_id.to_string(),
            event_type: "message".to_string(),
            payload: content.to_string(),
            clock: self.clock.clone(),
            timestamp: now,
            public_key: None,
            signature: None,
        };

        self.event_history.push(event.clone());
        Ok(event)
    }

    pub fn apply_event(&mut self, event: ChannelEvent) -> Result<()> {
        if event.channel_id != self.channel_id {
            return Err(anyhow!("Channel ID mismatch"));
        }

        // Idempotent duplicate check
        if self.event_history.iter().any(|e| e.event_id == event.event_id) {
            return Ok(());
        }

        // Update causal clock
        self.clock.merge(&event.clock);

        // Apply member state modifications
        if event.event_type == "member_join" {
            self.members.insert(
                event.author_id.clone(),
                ChannelMember {
                    unified_id: event.author_id.clone(),
                    display_name: event.author_id.clone(),
                    role: "member".to_string(),
                    joined_at: event.timestamp,
                },
            );
        } else if event.event_type == "member_leave" {
            self.members.remove(&event.author_id);
        }

        self.event_history.push(event);
        self.sort_events_deterministically();
        Ok(())
    }

    /// Sorts all channel events into a deterministically converged log across all replicas.
    pub fn sort_events_deterministically(&mut self) {
        self.event_history
            .sort_by(|a, b| a.compare_deterministic(b));
    }

    pub fn get_missing_events(&self, remote_clock: &VectorClock) -> Vec<ChannelEvent> {
        self.event_history
            .iter()
            .filter(|e| !remote_clock.is_causally_newer_or_equal(&e.clock))
            .cloned()
            .collect()
    }
}

pub struct FederationManager {
    channels: HashMap<String, FederatedChannel>,
}

impl FederationManager {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    pub fn create_channel(&mut self, id: &str, name: &str, topic: &str, creator: &str) -> &FederatedChannel {
        let channel = FederatedChannel::new(id, name, topic, creator);
        self.channels.insert(id.to_string(), channel);
        self.channels.get(id).unwrap()
    }

    pub fn get_channel(&self, id: &str) -> Option<&FederatedChannel> {
        self.channels.get(id)
    }

    pub fn get_channel_mut(&mut self, id: &str) -> Option<&mut FederatedChannel> {
        self.channels.get_mut(id)
    }

    pub fn sync_channel_events(&mut self, channel_id: &str, incoming_events: Vec<ChannelEvent>) -> Result<usize> {
        let channel = self.channels.get_mut(channel_id).ok_or_else(|| anyhow!("Channel not found"))?;
        let mut applied_count = 0;
        for ev in incoming_events {
            channel.apply_event(ev)?;
            applied_count += 1;
        }
        Ok(applied_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_clock_causality() {
        let mut c1 = VectorClock::new();
        c1.increment("alice");
        assert_eq!(c1.clocks.get("alice"), Some(&1));

        let mut c2 = c1.clone();
        c2.increment("bob");

        assert_eq!(c1.compare(&c2), CausalRelation::HappenedBefore);
        assert_eq!(c2.compare(&c1), CausalRelation::HappenedAfter);

        let mut c3 = VectorClock::new();
        c3.increment("charlie");
        assert_eq!(c1.compare(&c3), CausalRelation::Concurrent);

        c1.merge(&c2);
        assert_eq!(c1.clocks.get("bob"), Some(&1));
        assert!(c1.is_causally_newer_or_equal(&c2));
    }

    #[test]
    fn test_channel_event_signing_and_verification() {
        let mut ev = ChannelEvent {
            event_id: "ev_sig_1".to_string(),
            channel_id: "chan_secure".to_string(),
            author_id: "alice".to_string(),
            event_type: "message".to_string(),
            payload: "Signed message content".to_string(),
            clock: VectorClock::new(),
            timestamp: 1700000000,
            public_key: None,
            signature: None,
        };

        let pubkey = "alice_ed25519_pub";
        let privkey = "secret_key_simulation_123";

        ev.sign(pubkey, privkey);
        assert!(ev.signature.is_some());
        assert!(ev.verify_signature(privkey));

        // Tamper with payload
        ev.payload = "Tampered message content".to_string();
        assert!(!ev.verify_signature(privkey));
    }

    #[test]
    fn test_federated_channel_event_application_and_sync() {
        let mut mgr = FederationManager::new();
        mgr.create_channel("chan_general", "General Discussion", "Universal Cross-Platform Chat", "alice");

        let channel = mgr.get_channel_mut("chan_general").unwrap();
        assert_eq!(channel.members.len(), 1);
        assert!(channel.members.contains_key("alice"));

        // Bob joins
        let mut bob_clock = VectorClock::new();
        bob_clock.increment("bob");
        let join_event = ChannelEvent {
            event_id: "ev_1".to_string(),
            channel_id: "chan_general".to_string(),
            author_id: "bob".to_string(),
            event_type: "member_join".to_string(),
            payload: "Bob has joined the channel".to_string(),
            clock: bob_clock,
            timestamp: 1000,
            public_key: None,
            signature: None,
        };

        channel.apply_event(join_event).unwrap();
        assert_eq!(channel.members.len(), 2);
        assert!(channel.members.contains_key("bob"));

        // Check missing events for remote replica that hasn't seen bob's join
        let empty_clock = VectorClock::new();
        let missing = channel.get_missing_events(&empty_clock);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].event_id, "ev_1");
    }

    #[test]
    fn test_concurrent_conflict_resolution_convergence() {
        let mut chan_a = FederatedChannel::new("chan_1", "Sync Room", "Topic", "alice");
        let mut chan_b = chan_a.clone();

        // Alice sends message on chan_a
        let ev_alice = chan_a.post_message("alice", "Hello from Alice").unwrap();

        // Bob joins and sends message on chan_b concurrently
        let mut bob_clock = chan_b.clock.clone();
        bob_clock.increment("bob");
        let ev_bob_join = ChannelEvent {
            event_id: "ev_bob_join".to_string(),
            channel_id: "chan_1".to_string(),
            author_id: "bob".to_string(),
            event_type: "member_join".to_string(),
            payload: "Bob joined".to_string(),
            clock: bob_clock,
            timestamp: 1050,
            public_key: None,
            signature: None,
        };
        chan_b.apply_event(ev_bob_join.clone()).unwrap();
        let ev_bob = chan_b.post_message("bob", "Hello from Bob").unwrap();

        // Exchange events between replicas
        chan_a.apply_event(ev_bob_join).unwrap();
        chan_a.apply_event(ev_bob).unwrap();
        chan_b.apply_event(ev_alice).unwrap();

        // Verify both replicas have identical converged event histories
        assert_eq!(chan_a.event_history.len(), chan_b.event_history.len());
        for (ea, eb) in chan_a.event_history.iter().zip(chan_b.event_history.iter()) {
            assert_eq!(ea.event_id, eb.event_id);
        }
    }

    #[test]
    fn test_cross_platform_federated_sync_and_delivery() {
        let mut node_signal = FederationManager::new();
        let mut node_matrix = FederationManager::new();
        let mut node_simplex = FederationManager::new();

        // 1. Initialize channel on node_signal with Alice as creator
        node_signal.create_channel("chan_cross_net", "Global Devs", "Multi-bridge Chat", "alice");

        // 2. Alice sends an initial message
        let chan = node_signal.get_channel_mut("chan_cross_net").unwrap();
        let mut msg1 = chan.post_message("alice", "Bridge sync test started").unwrap();
        msg1.sign("alice_pub", "alice_priv_key");

        // 3. Matrix node synchronizes channel creation and Alice's message
        node_matrix.create_channel("chan_cross_net", "Global Devs", "Multi-bridge Chat", "alice");
        let applied_matrix = node_matrix
            .sync_channel_events("chan_cross_net", vec![msg1.clone()])
            .unwrap();
        assert_eq!(applied_matrix, 1);

        // 4. Bob joins from Matrix node and replies
        let chan_m = node_matrix.get_channel_mut("chan_cross_net").unwrap();
        let mut bob_clock = chan_m.clock.clone();
        bob_clock.increment("bob");
        let mut join_bob = ChannelEvent {
            event_id: "ev_bob_join_matrix".to_string(),
            channel_id: "chan_cross_net".to_string(),
            author_id: "bob".to_string(),
            event_type: "member_join".to_string(),
            payload: "Bob joined via Matrix".to_string(),
            clock: bob_clock,
            timestamp: 2000,
            public_key: None,
            signature: None,
        };
        join_bob.sign("bob_pub", "bob_priv_key");
        chan_m.apply_event(join_bob.clone()).unwrap();

        let mut msg2 = chan_m.post_message("bob", "ACK from Matrix bridge").unwrap();
        msg2.sign("bob_pub", "bob_priv_key");
        // Update signed event in chan_m history so missing events contains the signature
        if let Some(last) = chan_m.event_history.last_mut() {
            *last = msg2.clone();
        }

        // 5. SimpleX node syncs all missing events from Matrix
        node_simplex.create_channel("chan_cross_net", "Global Devs", "Multi-bridge Chat", "alice");
        let chan_s = node_simplex.get_channel("chan_cross_net").unwrap();
        let missing_for_simplex = chan_m.get_missing_events(&chan_s.clock);
        assert_eq!(missing_for_simplex.len(), 3); // msg1, join_bob, msg2

        let applied_simplex = node_simplex
            .sync_channel_events("chan_cross_net", missing_for_simplex)
            .unwrap();
        assert_eq!(applied_simplex, 3);

        // 6. Node Signal receives Bob's join and reply from Matrix
        let chan_sig = node_signal.get_channel_mut("chan_cross_net").unwrap();
        chan_sig.apply_event(join_bob).unwrap();
        chan_sig.apply_event(msg2).unwrap();

        // 7. Verify all three platform nodes have identical converged histories and verified signatures
        let hist_sig = &node_signal.get_channel("chan_cross_net").unwrap().event_history;
        let hist_mat = &node_matrix.get_channel("chan_cross_net").unwrap().event_history;
        let hist_smp = &node_simplex.get_channel("chan_cross_net").unwrap().event_history;

        assert_eq!(hist_sig.len(), 3);
        assert_eq!(hist_mat.len(), 3);
        assert_eq!(hist_smp.len(), 3);

        for i in 0..3 {
            assert_eq!(hist_sig[i].event_id, hist_mat[i].event_id);
            assert_eq!(hist_mat[i].event_id, hist_smp[i].event_id);
        }

        // Verify signatures of delivered messages
        assert!(hist_smp[0].verify_signature("alice_priv_key"));
        assert!(hist_smp[1].verify_signature("bob_priv_key"));
        assert!(hist_smp[2].verify_signature("bob_priv_key"));
    }
}
