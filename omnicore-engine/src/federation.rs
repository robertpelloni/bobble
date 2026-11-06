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
}

impl ChannelEvent {
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
}
