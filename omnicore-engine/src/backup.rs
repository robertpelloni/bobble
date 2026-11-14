use anyhow::{anyhow, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub provider_name: String,
    pub remote_endpoint: String,
    pub chunk_size_bytes: usize,
    pub auto_backup_interval_secs: u64,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            provider_name: "WebDAV / S3 Zero-Knowledge Vault".to_string(),
            remote_endpoint: "https://vault.omnicore.local/backup".to_string(),
            chunk_size_bytes: 65536, // 64 KiB
            auto_backup_interval_secs: 86400, // 24 hours
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedChunk {
    pub chunk_index: usize,
    /// Hex SHA-256 of encrypted_bytes (integrity check before decryption).
    pub sha256_checksum: String,
    /// 12-byte ChaCha20-Poly1305 nonce, unique per chunk in a backup.
    pub nonce: Vec<u8>,
    pub encrypted_bytes: Vec<u8>,
}

/// Derives the 32-byte AEAD key from the user key via SHA-256.
fn derive_key(key: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"omnicore-backup-v2:");
    hasher.update(key);
    hasher.finalize().into()
}

/// Deterministic per-chunk nonce: SHA-256(derived_key || index || created_at).
/// Unique for every chunk of a backup under a given key; a backup recreated
/// at a different second gets fresh nonces.
fn chunk_nonce(derived_key: &[u8; 32], index: usize, created_at: u64) -> [u8; 12] {
    let mut hasher = Sha256::new();
    hasher.update(derived_key);
    hasher.update((index as u64).to_be_bytes());
    hasher.update(created_at.to_be_bytes());
    let digest = hasher.finalize();
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&digest[..12]);
    nonce
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub manifest_id: String,
    pub total_chunks: usize,
    pub total_plaintext_size: usize,
    pub cipher_algorithm: String, // e.g. "ChaCha20-Poly1305"
    pub chunk_checksums: Vec<String>,
    pub created_at: u64,
}

pub struct ZeroKnowledgeBackup {
    config: BackupConfig,
}

impl ZeroKnowledgeBackup {
    pub fn new(config: BackupConfig) -> Self {
        Self { config }
    }

    /// Creates an encrypted chunked backup from plaintext database bytes.
    pub fn create_backup(&self, plaintext: &[u8], key: &[u8]) -> Result<(BackupManifest, Vec<EncryptedChunk>)> {
        if key.is_empty() {
            return Err(anyhow!("Backup encryption key cannot be empty"));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let chunk_size = self.config.chunk_size_bytes;
        let derived = derive_key(key);
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&derived));
        let mut chunks = Vec::new();
        let mut checksums = Vec::new();

        // Empty plaintext still produces one (empty) chunk so restores can
        // distinguish "empty backup" from "missing backup".
        let slices: Vec<&[u8]> = if plaintext.is_empty() { vec![&[]] } else { plaintext.chunks(chunk_size).collect() };
        for (index, slice) in slices.iter().enumerate() {
            let nonce_bytes = chunk_nonce(&derived, index, now);
            let encrypted = cipher
                .encrypt(Nonce::from_slice(&nonce_bytes), *slice)
                .map_err(|_| anyhow!("Backup chunk encryption failed at index {}", index))?;
            let checksum = sha256_hex(&encrypted);
            checksums.push(checksum.clone());
            chunks.push(EncryptedChunk {
                chunk_index: index,
                sha256_checksum: checksum,
                nonce: nonce_bytes.to_vec(),
                encrypted_bytes: encrypted,
            });
        }

        let manifest = BackupManifest {
            manifest_id: format!("manifest_{}", now),
            total_chunks: chunks.len(),
            total_plaintext_size: plaintext.len(),
            cipher_algorithm: "ChaCha20-Poly1305 AEAD (SHA-256 key derivation, per-chunk nonce)".to_string(),
            chunk_checksums: checksums,
            created_at: now,
        };

        Ok((manifest, chunks))
    }

    /// Restores plaintext bytes from verified encrypted chunks using the secret key.
    pub fn restore_backup(&self, manifest: &BackupManifest, chunks: &[EncryptedChunk], key: &[u8]) -> Result<Vec<u8>> {
        if chunks.len() != manifest.total_chunks {
            return Err(anyhow!(
                "Chunk count mismatch: expected {}, got {}",
                manifest.total_chunks,
                chunks.len()
            ));
        }

        let derived = derive_key(key);
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&derived));
        let mut reconstructed = Vec::with_capacity(manifest.total_plaintext_size);

        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.chunk_index != i {
                return Err(anyhow!("Chunk sequence error at index {}", i));
            }
            let expected_checksum = &manifest.chunk_checksums[i];
            if &sha256_hex(&chunk.encrypted_bytes) != expected_checksum {
                return Err(anyhow!("Integrity violation: checksum mismatch on chunk {}", i));
            }
            if chunk.nonce.len() != 12 {
                return Err(anyhow!("Invalid nonce length on chunk {}", i));
            }
            // AEAD decryption fails on tampered ciphertext or a wrong key.
            let decrypted = cipher
                .decrypt(Nonce::from_slice(&chunk.nonce), chunk.encrypted_bytes.as_ref())
                .map_err(|_| anyhow!("Decryption failed on chunk {} (tampered data or wrong key)", i))?;
            reconstructed.extend_from_slice(&decrypted);
        }

        Ok(reconstructed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_knowledge_backup_lifecycle() {
        let config = BackupConfig {
            provider_name: "Test Vault".to_string(),
            remote_endpoint: "https://test.local".to_string(),
            chunk_size_bytes: 16,
            auto_backup_interval_secs: 3600,
        };

        let backup = ZeroKnowledgeBackup::new(config);
        let secret_key = b"super_secret_master_key_123456";
        let original_data = b"OmniCore SQLCipher Encrypted Database Snapshot Payload Across Multi-Chunks!";

        // 1. Create backup
        let (manifest, chunks) = backup
            .create_backup(original_data, secret_key)
            .expect("Backup creation should succeed");

        assert!(manifest.total_chunks > 1);
        assert_eq!(chunks.len(), manifest.total_chunks);
        assert_eq!(manifest.total_plaintext_size, original_data.len());

        // 2. Restore backup
        let restored = backup
            .restore_backup(&manifest, &chunks, secret_key)
            .expect("Backup restoration should succeed");

        assert_eq!(restored, original_data);

        // 3. Tamper detection
        let mut tampered_chunks = chunks.clone();
        tampered_chunks[0].encrypted_bytes[0] ^= 0xFF;
        let tamper_res = backup.restore_backup(&manifest, &tampered_chunks, secret_key);
        assert!(tamper_res.is_err());

        // 4. Real AEAD: ciphertext must not contain the plaintext, the
        // manifest must name the real cipher, and a wrong key must fail.
        assert!(manifest.cipher_algorithm.contains("ChaCha20-Poly1305"));
        assert!(!chunks[0].encrypted_bytes.windows(b"OmniCore".len()).any(|w| w == b"OmniCore"));
        assert!(backup.restore_backup(&manifest, &chunks, b"wrong_key_entirely").is_err());
        // Empty plaintext round-trips as one chunk.
        let (empty_manifest, empty_chunks) = backup.create_backup(&[], secret_key).unwrap();
        assert_eq!(backup.restore_backup(&empty_manifest, &empty_chunks, secret_key).unwrap(), Vec::<u8>::new());
    }
}
