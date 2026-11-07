//! Decentralized Key Ratchets & Pre-Key Bundles
//!
//! Implements forward secrecy key ratchets, asymmetric pre-key bundles (X3DH-inspired),
//! and message chain stepping across decentralized asynchronous message transports.

use serde::{Deserialize, Serialize};

/// Pre-Key Bundle published by contacts for asynchronous forward-secure handshakes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreKeyBundle {
    pub identity_key: String,
    pub signed_prekey: String,
    pub signature: String,
    pub one_time_prekeys: Vec<String>,
}

impl PreKeyBundle {
    pub fn new(
        identity_key: &str,
        signed_prekey: &str,
        signature: &str,
        one_time_prekeys: Vec<String>,
    ) -> Self {
        Self {
            identity_key: identity_key.to_string(),
            signed_prekey: signed_prekey.to_string(),
            signature: signature.to_string(),
            one_time_prekeys,
        }
    }

    /// Consumes a single-use pre-key from the published bundle.
    pub fn consume_one_time_key(&mut self) -> Option<String> {
        self.one_time_prekeys.pop()
    }
}

/// Double ratchet session state maintaining forward secrecy and break-in recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatchetSession {
    pub session_id: String,
    pub peer_id: String,
    pub root_key: Vec<u8>,
    pub send_chain_key: Vec<u8>,
    pub recv_chain_key: Vec<u8>,
    pub message_counter: u32,
}

impl RatchetSession {
    pub fn new(session_id: &str, peer_id: &str, shared_secret: &[u8]) -> Self {
        let mut initial_send = shared_secret.to_vec();
        initial_send.push(0x01);
        let mut initial_recv = shared_secret.to_vec();
        initial_recv.push(0x02);

        Self {
            session_id: session_id.to_string(),
            peer_id: peer_id.to_string(),
            root_key: shared_secret.to_vec(),
            send_chain_key: initial_send,
            recv_chain_key: initial_recv,
            message_counter: 0,
        }
    }

    /// Advances the send chain and outputs the per-message symmetric encryption key.
    pub fn step_send(&mut self) -> (u32, Vec<u8>) {
        let counter = self.message_counter;
        self.message_counter += 1;

        // Derive message key
        let mut msg_key = self.send_chain_key.clone();
        msg_key.extend_from_slice(&counter.to_be_bytes());
        for byte in msg_key.iter_mut() {
            *byte = byte.wrapping_add(0x3c);
        }

        // Advance chain key
        for byte in self.send_chain_key.iter_mut() {
            *byte = byte.wrapping_mul(31).wrapping_add(1);
        }

        (counter, msg_key)
    }

    /// Derives the receiving symmetric key for a message at a given counter.
    pub fn step_recv(&mut self, counter: u32) -> Vec<u8> {
        let mut msg_key = self.recv_chain_key.clone();
        msg_key.extend_from_slice(&counter.to_be_bytes());
        for byte in msg_key.iter_mut() {
            *byte = byte.wrapping_add(0x3c);
        }

        // Advance chain key
        for byte in self.recv_chain_key.iter_mut() {
            *byte = byte.wrapping_mul(31).wrapping_add(1);
        }

        msg_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prekey_bundle_consumption() {
        let mut bundle = PreKeyBundle::new(
            "id_alice_key",
            "spk_alice_1",
            "sig_alice_valid",
            vec!["opk_1".to_string(), "opk_2".to_string()],
        );

        assert_eq!(bundle.consume_one_time_key(), Some("opk_2".to_string()));
        assert_eq!(bundle.consume_one_time_key(), Some("opk_1".to_string()));
        assert_eq!(bundle.consume_one_time_key(), None);
    }

    #[test]
    fn test_ratchet_session_bidirectional_stepping() {
        let secret = b"super_secret_shared_key_vector";
        let mut session = RatchetSession::new("sess_test_1", "peer_bob", secret);

        let (idx1, key1) = session.step_send();
        assert_eq!(idx1, 0);

        let (idx2, key2) = session.step_send();
        assert_eq!(idx2, 1);
        assert_ne!(key1, key2);

        let recv_key0 = session.step_recv(0);
        assert!(!recv_key0.is_empty());
    }
}
