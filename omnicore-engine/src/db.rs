use rusqlite::{Connection, Result};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn init<P: AsRef<Path>>(path: P, key: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Use SQLCipher to encrypt the database
        conn.pragma_update(None, "key", key)?;

        // Initialize schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS contacts (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                unified_id TEXT UNIQUE NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS mappings (
                id INTEGER PRIMARY KEY,
                contact_id INTEGER NOT NULL,
                platform TEXT NOT NULL,
                platform_account_id TEXT NOT NULL,
                FOREIGN KEY(contact_id) REFERENCES contacts(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY,
                contact_id INTEGER NOT NULL,
                platform TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(contact_id) REFERENCES contacts(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_tokens (
                platform TEXT PRIMARY KEY,
                token TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn insert_contact(&self, name: &str, unified_id: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO contacts (name, unified_id) VALUES (?1, ?2)",
            [name, unified_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_database_initialization() {
        let db_path = "test_init.db";
        // Clean up from previous run if any
        let _ = fs::remove_file(db_path);

        let db = Database::init(db_path, "test_key").expect("Failed to init db");

        // Test basic insert
        let id = db.insert_contact("Alice", "alice_unified_1").expect("Failed to insert contact");
        assert_eq!(id, 1);

        // Clean up
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn test_database_encryption() {
        let db_path = "test_enc.db";
        let _ = fs::remove_file(db_path);

        // Init with key
        let db = Database::init(db_path, "secret_key").expect("Failed to init db");
        db.insert_contact("Bob", "bob_unified_1").expect("Failed to insert contact");

        // Try opening with wrong key, assuming it works but queries fail or we just test it opens
        let wrong_db_result = Connection::open(db_path);
        assert!(wrong_db_result.is_ok()); // File opens, but data is unreadable

        let wrong_conn = wrong_db_result.unwrap();
        wrong_conn.pragma_update(None, "key", "wrong_key").unwrap();

        // This query should fail because the key is wrong
        let stmt = wrong_conn.prepare("SELECT name FROM contacts");
        assert!(stmt.is_err() || stmt.unwrap().query([]).is_err());

        // Clean up
        let _ = fs::remove_file(db_path);
    }
}
