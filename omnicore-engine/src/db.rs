use anyhow::{anyhow, Result};
use rusqlite::{params, Connection};
use std::path::Path;
use crate::models::{ConversationMessage, DeliveryState, ProtocolKind, UnifiedContact};

pub struct Database {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct ThreadRecord {
    pub id: i64,
    pub title: String,
    pub contact_id: i64,
    pub is_group: bool,
    pub last_message_at: i64,
}

#[derive(Debug, Clone)]
pub struct AttachmentRecord {
    pub id: i64,
    pub message_id: i64,
    pub file_name: String,
    pub mime_type: String,
    pub file_size_bytes: i64,
    pub storage_path: String,
    pub sha256_hash: String,
}

impl Database {
    pub fn init<P: AsRef<Path>>(path: P, key: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Apply SQLCipher encryption key and security configuration
        conn.pragma_update(None, "key", key)?;
        let _ = conn.pragma_update(None, "cipher_page_size", 4096);
        let _ = conn.pragma_update(None, "kdf_iter", 256000);
        let _ = conn.pragma_update(None, "foreign_keys", "ON");

        // 1. Unified Contacts table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS contacts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                unified_id TEXT UNIQUE NOT NULL,
                primary_protocol TEXT NOT NULL,
                fallback_protocols TEXT NOT NULL DEFAULT '[]',
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
            [],
        )?;

        // 2. Cross-Protocol Account Mappings
        conn.execute(
            "CREATE TABLE IF NOT EXISTS mappings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                contact_id INTEGER NOT NULL,
                protocol TEXT NOT NULL,
                platform_account_id TEXT NOT NULL,
                verified INTEGER NOT NULL DEFAULT 1,
                FOREIGN KEY(contact_id) REFERENCES contacts(id) ON DELETE CASCADE,
                UNIQUE(protocol, platform_account_id)
            )",
            [],
        )?;

        // 3. Conversation Threads (direct 1:1 or multi-party channels)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS threads (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                contact_id INTEGER NOT NULL,
                is_group INTEGER NOT NULL DEFAULT 0,
                last_message_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                FOREIGN KEY(contact_id) REFERENCES contacts(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // 4. Unified Messages
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                contact_id INTEGER NOT NULL,
                thread_id INTEGER,
                protocol TEXT NOT NULL,
                sender TEXT NOT NULL,
                recipient TEXT NOT NULL,
                content TEXT NOT NULL,
                delivery_state TEXT NOT NULL,
                timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                is_outgoing INTEGER NOT NULL DEFAULT 1,
                FOREIGN KEY(contact_id) REFERENCES contacts(id) ON DELETE CASCADE,
                FOREIGN KEY(thread_id) REFERENCES threads(id) ON DELETE SET NULL
            )",
            [],
        )?;

        // 5. Encrypted Media Attachments
        conn.execute(
            "CREATE TABLE IF NOT EXISTS attachments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id INTEGER NOT NULL,
                file_name TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                file_size_bytes INTEGER NOT NULL,
                storage_path TEXT NOT NULL,
                sha256_hash TEXT NOT NULL,
                FOREIGN KEY(message_id) REFERENCES messages(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // 6. Platform Cryptographic Key & Ratchet Store
        conn.execute(
            "CREATE TABLE IF NOT EXISTS key_store (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                protocol TEXT NOT NULL,
                account_id TEXT NOT NULL,
                key_type TEXT NOT NULL,
                encrypted_key_material BLOB NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                UNIQUE(protocol, account_id, key_type)
            )",
            [],
        )?;

        // 7. Sync Tokens
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_tokens (
                platform TEXT PRIMARY KEY,
                token TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
            [],
        )?;

        // Indices for high-throughput queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_contact ON messages(contact_id, timestamp)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(thread_id, timestamp)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_attachments_msg ON attachments(message_id)",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn insert_contact(
        &self,
        name: &str,
        unified_id: &str,
        primary: ProtocolKind,
        fallbacks: &[ProtocolKind],
    ) -> Result<i64> {
        let fallbacks_json = serde_json::to_string(fallbacks)?;
        self.conn.execute(
            "INSERT INTO contacts (name, unified_id, primary_protocol, fallback_protocols) VALUES (?1, ?2, ?3, ?4)",
            params![name, unified_id, primary.as_str(), fallbacks_json],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_contact_by_unified_id(&self, unified_id: &str) -> Result<Option<UnifiedContact>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, unified_id, primary_protocol, fallback_protocols, created_at FROM contacts WHERE unified_id = ?1",
        )?;
        let mut rows = stmt.query(params![unified_id])?;

        if let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let uid: String = row.get(2)?;
            let primary_str: String = row.get(3)?;
            let fallbacks_json: String = row.get(4)?;
            let created_at: i64 = row.get(5)?;

            let primary = ProtocolKind::from_str_name(&primary_str)
                .ok_or_else(|| anyhow!("Invalid protocol: {}", primary_str))?;
            let fallbacks: Vec<ProtocolKind> = serde_json::from_str(&fallbacks_json)?;

            Ok(Some(UnifiedContact {
                id,
                name,
                unified_id: uid,
                primary_protocol: primary,
                fallback_protocols: fallbacks,
                created_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn insert_thread(&self, title: &str, contact_id: i64, is_group: bool) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO threads (title, contact_id, is_group) VALUES (?1, ?2, ?3)",
            params![title, contact_id, if is_group { 1 } else { 0 }],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn record_message(
        &self,
        contact_id: i64,
        protocol: ProtocolKind,
        sender: &str,
        recipient: &str,
        content: &str,
        is_outgoing: bool,
    ) -> Result<i64> {
        let state_str = DeliveryState::Sent.as_str();
        self.conn.execute(
            "INSERT INTO messages (contact_id, protocol, sender, recipient, content, delivery_state, is_outgoing) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![contact_id, protocol.as_str(), sender, recipient, content, state_str, if is_outgoing { 1 } else { 0 }],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn record_message_with_thread(
        &self,
        contact_id: i64,
        thread_id: i64,
        protocol: ProtocolKind,
        sender: &str,
        recipient: &str,
        content: &str,
        is_outgoing: bool,
    ) -> Result<i64> {
        let state_str = DeliveryState::Sent.as_str();
        self.conn.execute(
            "INSERT INTO messages (contact_id, thread_id, protocol, sender, recipient, content, delivery_state, is_outgoing) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![contact_id, thread_id, protocol.as_str(), sender, recipient, content, state_str, if is_outgoing { 1 } else { 0 }],
        )?;
        let msg_id = self.conn.last_insert_rowid();

        // Update thread timestamp
        let _ = self.conn.execute(
            "UPDATE threads SET last_message_at = (strftime('%s', 'now')) WHERE id = ?1",
            params![thread_id],
        );

        Ok(msg_id)
    }

    pub fn insert_attachment(
        &self,
        message_id: i64,
        file_name: &str,
        mime_type: &str,
        file_size_bytes: i64,
        storage_path: &str,
        sha256_hash: &str,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO attachments (message_id, file_name, mime_type, file_size_bytes, storage_path, sha256_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![message_id, file_name, mime_type, file_size_bytes, storage_path, sha256_hash],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn save_key_material(
        &self,
        protocol: &str,
        account_id: &str,
        key_type: &str,
        key_bytes: &[u8],
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO key_store (protocol, account_id, key_type, encrypted_key_material) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(protocol, account_id, key_type) DO UPDATE SET encrypted_key_material = excluded.encrypted_key_material, updated_at = (strftime('%s', 'now'))",
            params![protocol, account_id, key_type, key_bytes],
        )?;
        Ok(())
    }

    pub fn get_key_material(
        &self,
        protocol: &str,
        account_id: &str,
        key_type: &str,
    ) -> Result<Option<Vec<u8>>> {
        let mut stmt = self.conn.prepare(
            "SELECT encrypted_key_material FROM key_store WHERE protocol = ?1 AND account_id = ?2 AND key_type = ?3",
        )?;
        let mut rows = stmt.query(params![protocol, account_id, key_type])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn update_delivery_state(&self, message_id: i64, state: &DeliveryState) -> Result<()> {
        let state_str = match state {
            DeliveryState::Queued => "Queued".to_string(),
            DeliveryState::Sent => "Sent".to_string(),
            DeliveryState::Delivered => "Delivered".to_string(),
            DeliveryState::Read => "Read".to_string(),
            DeliveryState::Failed(reason) => format!("Failed: {}", reason),
            DeliveryState::FallbackTriggered { from, to, reason } => {
                format!("Fallback: {} -> {} ({})", from.as_str(), to.as_str(), reason)
            }
        };

        self.conn.execute(
            "UPDATE messages SET delivery_state = ?1 WHERE id = ?2",
            params![state_str, message_id],
        )?;
        Ok(())
    }

    pub fn get_conversation_history(&self, contact_id: i64, limit: usize) -> Result<Vec<ConversationMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, contact_id, protocol, sender, recipient, content, delivery_state, timestamp, is_outgoing
             FROM messages WHERE contact_id = ?1 ORDER BY timestamp ASC, id ASC LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![contact_id, limit as i64], |row| {
            let id: i64 = row.get(0)?;
            let contact_id: i64 = row.get(1)?;
            let proto_str: String = row.get(2)?;
            let sender: String = row.get(3)?;
            let recipient: String = row.get(4)?;
            let content: String = row.get(5)?;
            let state_str: String = row.get(6)?;
            let timestamp: i64 = row.get(7)?;
            let is_outgoing: i64 = row.get(8)?;

            let protocol = ProtocolKind::from_str_name(&proto_str).unwrap_or(ProtocolKind::Signal);
            let state = match state_str.as_str() {
                "Queued" => DeliveryState::Queued,
                "Sent" => DeliveryState::Sent,
                "Delivered" => DeliveryState::Delivered,
                "Read" => DeliveryState::Read,
                s if s.starts_with("Failed:") => DeliveryState::Failed(s.trim_start_matches("Failed: ").to_string()),
                _ => DeliveryState::Delivered,
            };

            Ok(ConversationMessage {
                id,
                contact_id,
                protocol,
                sender,
                recipient,
                content,
                state,
                timestamp,
                is_outgoing: is_outgoing == 1,
            })
        })?;

        let mut msgs = Vec::new();
        for r in rows {
            msgs.push(r?);
        }
        Ok(msgs)
    }

    pub fn set_sync_token(&self, platform: &str, token: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sync_tokens (platform, token) VALUES (?1, ?2)
             ON CONFLICT(platform) DO UPDATE SET token = excluded.token, updated_at = strftime('%s', 'now')",
            params![platform, token],
        )?;
        Ok(())
    }

    pub fn get_sync_token(&self, platform: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT token FROM sync_tokens WHERE platform = ?1")?;
        let mut rows = stmt.query(params![platform])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_database_initialization() {
        let db_path = "test_init.db";
        let _ = fs::remove_file(db_path);

        let db = Database::init(db_path, "test_key").expect("Failed to init db");
        let id = db.insert_contact("Alice", "alice_unified_1", ProtocolKind::Signal, &[ProtocolKind::Matrix])
            .expect("Failed to insert contact");
        assert_eq!(id, 1);

        let c = db.get_contact_by_unified_id("alice_unified_1").unwrap().unwrap();
        assert_eq!(c.name, "Alice");
        assert_eq!(c.primary_protocol, ProtocolKind::Signal);
        assert_eq!(c.fallback_protocols, vec![ProtocolKind::Matrix]);

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn test_threads_and_attachments() {
        let db_path = "test_threads.db";
        let _ = fs::remove_file(db_path);

        let db = Database::init(db_path, "secret_key").expect("Failed to init db");
        let contact_id = db.insert_contact("Charlie", "charlie_1", ProtocolKind::Discord, &[]).unwrap();
        let thread_id = db.insert_thread("Direct Channel", contact_id, false).unwrap();
        assert_eq!(thread_id, 1);

        let msg_id = db.record_message_with_thread(
            contact_id,
            thread_id,
            ProtocolKind::Discord,
            "Me",
            "Charlie",
            "See attached encrypted report.",
            true,
        ).unwrap();
        assert_eq!(msg_id, 1);

        let attach_id = db.insert_attachment(
            msg_id,
            "report.pdf",
            "application/pdf",
            1048576,
            "/vault/encrypted/report.pdf.enc",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ).unwrap();
        assert_eq!(attach_id, 1);

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn test_key_store_operations() {
        let db_path = "test_keys.db";
        let _ = fs::remove_file(db_path);

        let db = Database::init(db_path, "secret_key").expect("Failed to init db");
        let key_data = vec![1, 2, 3, 4, 5, 42];
        db.save_key_material("Signal", "+15550199", "identity_ratchet", &key_data).unwrap();

        let retrieved = db.get_key_material("Signal", "+15550199", "identity_ratchet").unwrap();
        assert_eq!(retrieved, Some(key_data.clone()));

        // Update key material
        let new_key = vec![9, 8, 7];
        db.save_key_material("Signal", "+15550199", "identity_ratchet", &new_key).unwrap();
        let updated = db.get_key_material("Signal", "+15550199", "identity_ratchet").unwrap();
        assert_eq!(updated, Some(new_key));

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn test_message_history_and_state_updates() {
        let db_path = "test_msgs.db";
        let _ = fs::remove_file(db_path);

        let db = Database::init(db_path, "test_key").expect("Failed to init db");
        let contact_id = db.insert_contact("Bob", "bob_unified_1", ProtocolKind::Matrix, &[])
            .unwrap();

        let msg_id = db.record_message(contact_id, ProtocolKind::Matrix, "Bob", "Me", "Hello world!", false)
            .unwrap();
        assert_eq!(msg_id, 1);

        db.update_delivery_state(msg_id, &DeliveryState::Read).unwrap();

        let history = db.get_conversation_history(contact_id, 10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, "Hello world!");
        assert!(!history[0].is_outgoing);

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn test_sync_token_upsert() {
        let db_path = "test_tokens.db";
        let _ = fs::remove_file(db_path);

        let db = Database::init(db_path, "test_key").expect("Failed to init db");
        db.set_sync_token("Matrix", "token_v1").unwrap();
        assert_eq!(db.get_sync_token("Matrix").unwrap(), Some("token_v1".to_string()));

        db.set_sync_token("Matrix", "token_v2").unwrap();
        assert_eq!(db.get_sync_token("Matrix").unwrap(), Some("token_v2".to_string()));

        let _ = fs::remove_file(db_path);
    }
}
