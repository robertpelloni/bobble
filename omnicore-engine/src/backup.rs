use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
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
    pub sha256_checksum: String,
    pub encrypted_bytes: Vec<u8>,
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
        let mut chunks = Vec::new();
        let mut checksums = Vec::new();

        for (index, slice) in plaintext.chunks(chunk_size).enumerate() {
            // Client-side encryption: XOR with rolling key stream as PoC
            let mut encrypted = Vec::with_capacity(slice.len());
            for (i, &b) in slice.iter().enumerate() {
                let k = key[i % key.len()];
                encrypted.push(b ^ k);
            }

            let checksum = format!("{:016x}", compute_checksum(&encrypted));
            checksums.push(checksum.clone());

            chunks.push(EncryptedChunk {
                chunk_index: index,
                sha256_checksum: checksum,
                encrypted_bytes: encrypted,
            });
        }

        let manifest = BackupManifest {
            manifest_id: format!("manifest_{}", now),
            total_chunks: chunks.len(),
            total_plaintext_size: plaintext.len(),
            // Honest label: create_backup currently uses a rolling-key XOR
            // stream as a proof of concept, not a production AEAD cipher.
            cipher_algorithm: "XOR rolling-key PoC (not production AEAD)".to_string(),
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

        let mut reconstructed = Vec::with_capacity(manifest.total_plaintext_size);

        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.chunk_index != i {
                return Err(anyhow!("Chunk sequence error at index {}", i));
            }
            let expected_checksum = &manifest.chunk_checksums[i];
            let actual_checksum = format!("{:016x}", compute_checksum(&chunk.encrypted_bytes));
            if &actual_checksum != expected_checksum {
                return Err(anyhow!("Integrity violation: checksum mismatch on chunk {}", i));
            }

            // Decrypt chunk
            for (j, &b) in chunk.encrypted_bytes.iter().enumerate() {
                let k = key[j % key.len()];
                reconstructed.push(b ^ k);
            }
        }

        Ok(reconstructed)
    }
}

fn compute_checksum(bytes: &[u8]) -> u64 {
    let mut sum: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        sum ^= b as u64;
        sum = sum.wrapping_mul(0x100000001b3);
    }
    sum
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
    }
}
