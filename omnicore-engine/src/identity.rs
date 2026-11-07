use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::models::ProtocolKind;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityFingerprint {
    pub algorithm: String, // e.g. "ed25519"
    pub public_key_hex: String,
    pub formatted_fingerprint: String,
}

impl IdentityFingerprint {
    pub fn new(public_key_hex: &str) -> Self {
        // Formats fingerprint in groups of 4 characters: XXXX-XXXX-...
        let formatted = public_key_hex
            .chars()
            .collect::<Vec<char>>()
            .chunks(4)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join("-");

        Self {
            algorithm: "ed25519".to_string(),
            public_key_hex: public_key_hex.to_string(),
            formatted_fingerprint: formatted,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityCard {
    pub version: u32,
    pub unified_id: String,
    pub display_name: String,
    pub fingerprint: IdentityFingerprint,
    pub platform_mappings: HashMap<String, String>, // protocol -> platform_account_id
    pub signature_hex: String,
    pub issued_at: u64,
}

impl IdentityCard {
    pub fn new(
        unified_id: &str,
        display_name: &str,
        public_key_hex: &str,
        mappings: HashMap<ProtocolKind, String>,
    ) -> Self {
        let issued_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut str_mappings = HashMap::new();
        for (k, v) in mappings {
            str_mappings.insert(k.as_str().to_string(), v);
        }

        let fingerprint = IdentityFingerprint::new(public_key_hex);

        // Deterministic mock signature for decentralized card verification
        let signature_payload = format!("{}:{}:{}", unified_id, public_key_hex, issued_at);
        let signature_hex = format!("{:x}", md5_digest_approx(signature_payload.as_bytes()));

        Self {
            version: 1,
            unified_id: unified_id.to_string(),
            display_name: display_name.to_string(),
            fingerprint,
            platform_mappings: str_mappings,
            signature_hex,
            issued_at,
        }
    }

    pub fn export_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to export identity card JSON: {}", e))
    }

    pub fn import_json(json_str: &str) -> Result<Self> {
        serde_json::from_str(json_str)
            .map_err(|e| anyhow!("Failed to parse identity card JSON: {}", e))
    }

    pub fn verify_signature(&self) -> bool {
        let expected_payload = format!("{}:{}:{}", self.unified_id, self.fingerprint.public_key_hex, self.issued_at);
        let expected_sig = format!("{:x}", md5_digest_approx(expected_payload.as_bytes()));
        self.signature_hex == expected_sig
    }

    /// Encodes a compact QR payload string for mobile peer scanning.
    pub fn to_qr_payload(&self) -> Result<String> {
        let json = serde_json::to_string(self)
            .map_err(|e| anyhow!("Failed to serialize identity card: {}", e))?;
        let encoded = hex_encode_bytes(json.as_bytes());
        Ok(format!("omnicore://identity?data={}", encoded))
    }

    /// Reconstructs an IdentityCard from a scanned QR payload string.
    pub fn from_qr_payload(uri: &str) -> Result<Self> {
        let prefix = "omnicore://identity?data=";
        if !uri.starts_with(prefix) {
            return Err(anyhow!("Invalid QR identity URI scheme"));
        }
        let hex_data = &uri[prefix.len()..];
        let bytes = hex_decode_bytes(hex_data)?;
        let json = String::from_utf8(bytes)
            .map_err(|e| anyhow!("Corrupted identity card JSON encoding: {}", e))?;
        let card: Self = serde_json::from_str(&json)
            .map_err(|e| anyhow!("Failed to deserialize identity card from QR: {}", e))?;
        if !card.verify_signature() {
            return Err(anyhow!("QR identity card failed cryptographic verification"));
        }
        Ok(card)
    }

    /// Validates an interactive mutual challenge response from a scanned peer.
    pub fn verify_peer_handshake(&self, challenge: &[u8], signature_hex: &str) -> bool {
        let mut payload = challenge.to_vec();
        payload.extend_from_slice(self.fingerprint.public_key_hex.as_bytes());
        let expected = format!("{:x}", md5_digest_approx(&payload));
        signature_hex == expected
    }
}

fn hex_encode_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode_bytes(hex_str: &str) -> Result<Vec<u8>> {
    if hex_str.len() % 2 != 0 {
        return Err(anyhow!("Odd length hex string"));
    }
    (0..hex_str.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex_str[i..i + 2], 16)
                .map_err(|e| anyhow!("Invalid hex byte: {}", e))
        })
        .collect()
}

fn md5_digest_approx(bytes: &[u8]) -> u128 {
    let mut hash: u128 = 0x67452301efcdab8998badcfe10325476;
    for &b in bytes {
        hash = hash.rotate_left(5) ^ (b as u128);
        hash = hash.wrapping_mul(0x5bd1e995);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_card_export_and_verify() {
        let mut mappings = HashMap::new();
        mappings.insert(ProtocolKind::Signal, "+15550199".to_string());
        mappings.insert(ProtocolKind::Matrix, "@alice:matrix.org".to_string());

        let pk = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let card = IdentityCard::new("alice_unified", "Alice Montgomery", pk, mappings);

        assert!(card.verify_signature());
        assert!(card.fingerprint.formatted_fingerprint.contains("-"));

        // JSON Export & Import
        let json = card.export_json().unwrap();
        let imported = IdentityCard::import_json(&json).unwrap();

        assert_eq!(card.unified_id, imported.unified_id);
        assert_eq!(card.display_name, imported.display_name);
        assert!(imported.verify_signature());
    }

    #[test]
    fn test_identity_card_qr_payload_roundtrip() {
        let mut mappings = HashMap::new();
        mappings.insert(ProtocolKind::Signal, "+15550300".to_string());
        mappings.insert(ProtocolKind::Simplex, "simplex_contact_abc".to_string());

        let pk = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let card = IdentityCard::new("bob_unified", "Bob Smith", pk, mappings);

        let qr_uri = card.to_qr_payload().unwrap();
        assert!(qr_uri.starts_with("omnicore://identity?data="));

        let decoded = IdentityCard::from_qr_payload(&qr_uri).unwrap();
        assert_eq!(decoded.unified_id, "bob_unified");
        assert_eq!(decoded.display_name, "Bob Smith");
        assert!(decoded.verify_signature());
    }

    #[test]
    fn test_identity_card_peer_handshake_verification() {
        let card = IdentityCard::new(
            "alice_auth",
            "Alice Montgomery",
            "fedcba0987654321fedcba0987654321fedcba0987654321fedcba0987654321",
            HashMap::new(),
        );

        let challenge = b"random_mutual_session_nonce_1234";
        let mut payload = challenge.to_vec();
        payload.extend_from_slice(card.fingerprint.public_key_hex.as_bytes());
        let valid_sig = format!("{:x}", md5_digest_approx(&payload));

        assert!(card.verify_peer_handshake(challenge, &valid_sig));
        assert!(!card.verify_peer_handshake(challenge, "invalid_signature"));
    }
}
