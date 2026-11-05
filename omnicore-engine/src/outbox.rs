use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::models::ProtocolKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay_secs: u64,
    pub backoff_multiplier: f32,
    pub max_delay_secs: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay_secs: 2,
            backoff_multiplier: 2.0,
            max_delay_secs: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatchetState {
    pub root_key: Vec<u8>,
    pub chain_key: Vec<u8>,
    pub message_index: u32,
}

impl RatchetState {
    pub fn new(root_key: Vec<u8>) -> Self {
        let chain_key = root_key.clone();
        Self {
            root_key,
            chain_key,
            message_index: 0,
        }
    }

    /// Advances forward secrecy chain key and increments index
    pub fn advance(&mut self) -> Vec<u8> {
        let mut next_key = self.chain_key.clone();
        for byte in next_key.iter_mut() {
            *byte = byte.wrapping_add(1);
        }
        self.chain_key = next_key.clone();
        self.message_index += 1;
        next_key
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedMessage {
    pub id: String,
    pub contact_id: i64,
    pub payload_bytes: Vec<u8>,
    pub candidate_protocols: Vec<ProtocolKind>,
    pub attempts: u32,
    pub next_attempt_at: u64,
    pub enqueued_at: u64,
}

pub struct OutboxManager {
    queue: VecDeque<QueuedMessage>,
    policy: RetryPolicy,
}

impl OutboxManager {
    pub fn new(policy: RetryPolicy) -> Self {
        Self {
            queue: VecDeque::new(),
            policy,
        }
    }

    pub fn enqueue(
        &mut self,
        id: &str,
        contact_id: i64,
        payload_bytes: Vec<u8>,
        candidate_protocols: Vec<ProtocolKind>,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.enqueue_at(id, contact_id, payload_bytes, candidate_protocols, now);
    }

    pub fn enqueue_at(
        &mut self,
        id: &str,
        contact_id: i64,
        payload_bytes: Vec<u8>,
        candidate_protocols: Vec<ProtocolKind>,
        timestamp: u64,
    ) {
        self.queue.push_back(QueuedMessage {
            id: id.to_string(),
            contact_id,
            payload_bytes,
            candidate_protocols,
            attempts: 0,
            next_attempt_at: timestamp,
            enqueued_at: timestamp,
        });
    }

    pub fn pop_ready_messages(&mut self, current_time: u64) -> Vec<QueuedMessage> {
        let mut ready = Vec::new();
        let mut remaining = VecDeque::new();

        while let Some(msg) = self.queue.pop_front() {
            if msg.next_attempt_at <= current_time {
                ready.push(msg);
            } else {
                remaining.push_back(msg);
            }
        }

        self.queue = remaining;
        ready
    }

    pub fn record_failure(&mut self, mut msg: QueuedMessage, current_time: u64) -> bool {
        msg.attempts += 1;
        if msg.attempts >= self.policy.max_attempts {
            // Exceeded retry attempts
            return false;
        }

        let delay = ((self.policy.initial_delay_secs as f32)
            * self.policy.backoff_multiplier.powi((msg.attempts - 1) as i32))
            as u64;
        let clamped_delay = delay.min(self.policy.max_delay_secs);

        msg.next_attempt_at = current_time + clamped_delay;
        self.queue.push_back(msg);
        true
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ratchet_state_advancement() {
        let mut ratchet = RatchetState::new(vec![0x10, 0x20, 0x30]);
        assert_eq!(ratchet.message_index, 0);

        let k1 = ratchet.advance();
        assert_eq!(ratchet.message_index, 1);
        assert_eq!(k1, vec![0x11, 0x21, 0x31]);

        let k2 = ratchet.advance();
        assert_eq!(ratchet.message_index, 2);
        assert_eq!(k2, vec![0x12, 0x22, 0x32]);
    }

    #[test]
    fn test_outbox_retry_backoff() {
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay_secs: 2,
            backoff_multiplier: 2.0,
            max_delay_secs: 60,
        };

        let mut outbox = OutboxManager::new(policy);
        outbox.enqueue_at("msg_1", 100, vec![1, 2, 3], vec![ProtocolKind::Signal], 1000);
        assert_eq!(outbox.len(), 1);

        let ready = outbox.pop_ready_messages(1000);
        assert_eq!(ready.len(), 1);
        assert_eq!(outbox.len(), 0);

        let mut msg = ready[0].clone();
        // First failure: 2 * 2^0 = 2s delay -> next attempt at 1002
        let retry_queued = outbox.record_failure(msg.clone(), 1000);
        assert!(retry_queued);
        assert_eq!(outbox.len(), 1);

        // Not ready at 1001
        assert_eq!(outbox.pop_ready_messages(1001).len(), 0);

        // Ready at 1002
        let ready2 = outbox.pop_ready_messages(1002);
        assert_eq!(ready2.len(), 1);
        msg = ready2[0].clone();

        // Second failure: 2 * 2^1 = 4s delay -> next attempt at 1006
        assert!(outbox.record_failure(msg.clone(), 1002));
        let ready3 = outbox.pop_ready_messages(1006);
        assert_eq!(ready3.len(), 1);
        msg = ready3[0].clone();

        // Third failure: reaches max_attempts (3) -> discarded
        let retry_queued_again = outbox.record_failure(msg, 1006);
        assert!(!retry_queued_again);
        assert!(outbox.is_empty());
    }
}
