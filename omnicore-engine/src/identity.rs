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
}
