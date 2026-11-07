# bobble (OmniCore)

**Universal next-generation messaging client** — a modern aggregator that unifies every messaging platform into a single cohesive desktop and mobile experience: Signal, Simplex, Matrix, Tox, Discord, iMessage, and SMS — with no platform treated as second-class, featuring intelligent multi-path fallback routing across networks.

---

## Architecture Overview

OmniCore uses a **Headless Rust Core + Multi-Target UI** architecture:

```text
               +--------------------------------------------+
               |        Desktop & Mobile Client Shells      |
               |  - omnicore-ui: Tauri v2 + React 19 + Tray |
               |  - omnicore-mobile: React Native + Expo    |
               +--------------------------------------------+
                                      |
                     JSON-RPC IPC / Local WebSockets
                                      |
               +--------------------------------------------+
               |               omnicore-engine              |
               |                                            |
               |  +-------------------+  +---------------+  |
               |  |  SQLCipher Vault  |  | Multipath     |  |
               |  |  AES-256-GCM / PBK|  | Router        |  |
               |  +-------------------+  +---------------+  |
               |  |  Federation &     |  | Ratchets &    |  |
               |  |  Vector Clocks    |  | Pre-keys      |  |
               |  +-------------------+  +---------------+  |
               |  |  Tor SOCKS5 &     |  | Hardware      |  |
               |  |  Reputation Engine|  | Telemetry     |  |
               |  +-------------------+  +---------------+  |
               +--------------------------------------------+
                                      |
         +----------+----------+------+-----+----------+----------+
         |          |          |            |          |          |
      Signal     Matrix     Discord      SimpleX      Tox        SMS /
      (IPC)      (SDK)      (REST/GW)    (Queue)     (DHT)      iMessage
```

---

## Core Engine & Database Design

### 1. SQLCipher Encrypted Storage
- **Cryptographic Pragmas**: AES-256-GCM with PBKDF2-HMAC-SHA512 (`kdf_iter = 256000`, `cipher_page_size = 4096`, `foreign_keys = ON`).
- **Relational Tables**:
  - `contacts`: Unified human identities aggregating multiple accounts.
  - `mappings`: External protocol IDs and handles mapped to unified contacts.
  - `threads`: Direct 1:1 and multi-party chat sessions.
  - `messages`: Universal message ledger tracking multi-protocol delivery states.
  - `attachments`: Encrypted metadata for media blobs and files.
  - `key_store`: Encrypted vault for Double Ratchet states and session keys.
  - `sync_tokens`: Platform synchronization cursors for delta replication.
- **Specification**: Complete database design is documented in [`omnicore-engine/SCHEMA.md`](omnicore-engine/SCHEMA.md).

### 2. Multi-Path Protocol Fallback Router
- Transparently routes messages across primary and fallback bridges (e.g. Signal -> Matrix -> SMS) with candidate exhaustion handling.
- Integrated bridges for Signal, Matrix, Discord, SimpleX Chat, Tox DHT, Cellular SMS modem, and macOS iMessage relay.

### 3. Decentralized Federation & Security
- Multi-party channel state synchronization with causal vector clocks (`VectorClock`) and deterministic conflict resolution.
- Cryptographic event signing and verification (`digest`, `sign`, `verify_signature`).
- Double Ratchet forward secrecy session management and pre-key bundles (`ratchet.rs`).
- Anonymity routing via Tor SOCKS5 proxy circuits and v3 .onion hidden service endpoints (`tor.rs`).
- Zero-knowledge encrypted cloud backup provider with SHA-256 manifests (`backup.rs`).
- Decentralized QR identity card pairing and mutual handshake challenge verification (`identity.rs`).

---

## User Interface & System Tray

- **Consolidated Dashboard UI (`omnicore-ui`)**: Single-page multi-pane view featuring Unified Chat Hub, Unified Identity Manager, Protocol Bridge Deck, and Routing Monitor with tooltips and accessible guidance.
- **Native System Tray**: Tauri v2 tray icon with window show/hide toggles, bridge telemetry, and background server termination (`quit_app`).
- **Mobile Shell (`omnicore-mobile`)**: React Native / Expo application with conversation feeds, bridge status cards, and settings view.

---

## Development & Build Automation

Helper scripts in `scripts/`:

```bash
# Build React frontend and compile Cargo workspace
bash scripts/build.sh

# Run full test suite (TypeScript build + Cargo workspace tests)
bash scripts/test.sh

# Start development preview server
bash scripts/dev.sh

# Run desktop Tauri application
bash scripts/run.sh

# Package multi-target bundles (desktop, android, ios, or all)
bash scripts/bundle.sh all
```

---

## Status

Active implementation (v0.5.2). Part of Robert Pelloni's bob* ecosystem.
