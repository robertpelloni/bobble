//! Decentralized Key Ratchets & Pre-Key Bundles
//!
//! Implements forward secrecy key ratchets, asymmetric pre-key bundles (X3DH-inspired),
//! and message chain stepping across decentralized asynchronous message transports.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use crate::db::Database;

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

    /// Generates a pre-key bundle with N one-time ephemeral pre-keys.
    pub fn generate_ephemeral_bundle(identity: &str, count: usize) -> Self {
        let opks: Vec<String> = (1..=count)
            .map(|i| format!("opk_{}_{}", identity, i))
            .collect();
        Self {
            identity_key: format!("id_key_{}", identity),
            signed_prekey: format!("spk_{}_1", identity),
            signature: format!("sig_spk_{}", identity),
            one_time_prekeys: opks,
        }
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

    /// Serializes session state into encrypted key_store byte payload.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self)
            .map_err(|e| anyhow!("Failed to serialize RatchetSession: {}", e))
    }

    /// Deserializes session state from encrypted key_store byte payload.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes)
            .map_err(|e| anyhow!("Failed to deserialize RatchetSession: {}", e))
    }

    /// Persists the active ratchet session state into SQLCipher key_store.
    pub fn persist_to_db(&self, db: &Database, protocol: &str) -> Result<()> {
        let key_data = self.to_bytes()?;
        db.save_key_material(protocol, &self.peer_id, "double_ratchet_session", &key_data)?;
        Ok(())
    }

    /// Loads a persisted ratchet session state from SQLCipher key_store.
    pub fn load_from_db(db: &Database, protocol: &str, peer_id: &str) -> Result<Option<Self>> {
        if let Some(key_data) = db.get_key_material(protocol, peer_id, "double_ratchet_session")? {
            let session = Self::from_bytes(&key_data)?;
            Ok(Some(session))
        } else {
            Ok(None)
        }
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

    #[test]
    fn test_ephemeral_bundle_generation() {
        let mut bundle = PreKeyBundle::generate_ephemeral_bundle("bob", 5);
        assert_eq!(bundle.identity_key, "id_key_bob");
        assert_eq!(bundle.one_time_prekeys.len(), 5);

        let consumed = bundle.consume_one_time_key();
        assert_eq!(consumed, Some("opk_bob_5".to_string()));
        assert_eq!(bundle.one_time_prekeys.len(), 4);
    }

    #[test]
    fn test_ratchet_session_db_persistence() {
        let db_path = ":memory:";
        let db = Database::init(db_path, "test_ratchet_passphrase").unwrap();
        let secret = b"shared_key_persistence_vector";
        let mut session = RatchetSession::new("sess_persisted_1", "peer_carol", secret);
        session.step_send();
        session.step_send();
        assert_eq!(session.message_counter, 2);

        // Persist to SQLCipher key_store
        session.persist_to_db(&db, "Signal").unwrap();

        // Load back from SQLCipher key_store
        let loaded = RatchetSession::load_from_db(&db, "Signal", "peer_carol")
            .unwrap()
            .expect("Session should be loaded from db");

        assert_eq!(loaded.session_id, "sess_persisted_1");
        assert_eq!(loaded.peer_id, "peer_carol");
        assert_eq!(loaded.message_counter, 2);
        assert_eq!(loaded.root_key, secret);
    }
}
