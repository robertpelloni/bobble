# OmniCore Database Architecture & SQLCipher Schema Specification

## 1. Cryptographic Security Pragmas (SQLCipher)

OmniCore uses SQLCipher with AES-256-GCM / PBKDF2-HMAC-SHA512:
- `cipher_page_size = 4096`: Optimal disk sector aligned I/O.
- `kdf_iter = 256000`: High work-factor key derivation resisting brute-force attacks.
- `foreign_keys = ON`: Strict relational integrity and cascading deletes.

---

## 2. Entity Relational Architecture

```text
+-------------------+       +--------------------+
|     contacts      | 1---* |      mappings      |
+-------------------+       +--------------------+
| id (PK)           |       | id (PK)            |
| name              |       | contact_id (FK)    |
| unified_id (UQ)   |       | protocol           |
| primary_protocol  |       | platform_account_id|
| fallback_protocols|       | verified           |
+-------------------+       +--------------------+
        | 1
        |
        *
+-------------------+
|      threads      |
+-------------------+
| id (PK)           |
| title             |
| contact_id (FK)   |
| is_group          |
| last_message_at   |
+-------------------+
        | 1
        |
        *
+-------------------+       +--------------------+
|     messages      | 1---* |    attachments     |
+-------------------+       +--------------------+
| id (PK)           |       | id (PK)            |
| contact_id (FK)   |       | message_id (FK)    |
| thread_id (FK)    |       | file_name          |
| protocol          |       | mime_type          |
| sender / recipient|       | file_size_bytes    |
| content           |       | storage_path       |
| delivery_state    |       | sha256_hash        |
| timestamp         |       +--------------------+
| is_outgoing       |
+-------------------+

+---------------------------+       +--------------------+
|         key_store         |       |    sync_tokens     |
+---------------------------+       +--------------------+
| id (PK)                   |       | platform (PK)      |
| protocol                  |       | token              |
| account_id                |       | updated_at         |
| key_type                  |       +--------------------+
| encrypted_key_material    |
| updated_at                |
+---------------------------+
```

---

## 3. Schema Definitions

### `contacts`
Stores unified human identities aggregating accounts across multiple networks.
- `id` INTEGER PRIMARY KEY AUTOINCREMENT
- `name` TEXT NOT NULL
- `unified_id` TEXT UNIQUE NOT NULL
- `primary_protocol` TEXT NOT NULL
- `fallback_protocols` TEXT NOT NULL DEFAULT '[]'
- `created_at` INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))

### `mappings`
Maps external protocol usernames/phone numbers/handles to a unified contact.
- `id` INTEGER PRIMARY KEY AUTOINCREMENT
- `contact_id` INTEGER NOT NULL (FK -> contacts.id ON DELETE CASCADE)
- `protocol` TEXT NOT NULL
- `platform_account_id` TEXT NOT NULL
- `verified` INTEGER NOT NULL DEFAULT 1
- `UNIQUE(protocol, platform_account_id)`

### `threads`
Represents direct 1:1 or multi-party chat sessions.
- `id` INTEGER PRIMARY KEY AUTOINCREMENT
- `title` TEXT NOT NULL
- `contact_id` INTEGER NOT NULL (FK -> contacts.id ON DELETE CASCADE)
- `is_group` INTEGER NOT NULL DEFAULT 0
- `last_message_at` INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))

### `messages`
Unified ledger of incoming and outgoing chat payloads across all transports.
- `id` INTEGER PRIMARY KEY AUTOINCREMENT
- `contact_id` INTEGER NOT NULL (FK -> contacts.id ON DELETE CASCADE)
- `thread_id` INTEGER (FK -> threads.id ON DELETE SET NULL)
- `protocol` TEXT NOT NULL
- `sender` TEXT NOT NULL
- `recipient` TEXT NOT NULL
- `content` TEXT NOT NULL
- `delivery_state` TEXT NOT NULL
- `timestamp` INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
- `is_outgoing` INTEGER NOT NULL DEFAULT 1

### `attachments`
Encrypted metadata for media or binary attachments tied to messages.
- `id` INTEGER PRIMARY KEY AUTOINCREMENT
- `message_id` INTEGER NOT NULL (FK -> messages.id ON DELETE CASCADE)
- `file_name` TEXT NOT NULL
- `mime_type` TEXT NOT NULL
- `file_size_bytes` INTEGER NOT NULL
- `storage_path` TEXT NOT NULL
- `sha256_hash` TEXT NOT NULL

### `key_store`
Encrypted vault for platform-specific session tokens, Signal Double Ratchet states, and Matrix Megolm keys.
- `id` INTEGER PRIMARY KEY AUTOINCREMENT
- `protocol` TEXT NOT NULL
- `account_id` TEXT NOT NULL
- `key_type` TEXT NOT NULL
- `encrypted_key_material` BLOB NOT NULL
- `updated_at` INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
- `UNIQUE(protocol, account_id, key_type)`

### `sync_tokens`
Stores latest synchronization cursors per platform (e.g. Matrix sync batch tokens).
- `platform` TEXT PRIMARY KEY
- `token` TEXT NOT NULL
- `updated_at` INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))

---

## 4. High-Throughput Indexes
- `idx_messages_contact` on `messages(contact_id, timestamp)`
- `idx_messages_thread` on `messages(thread_id, timestamp)`
- `idx_attachments_msg` on `attachments(message_id)`
