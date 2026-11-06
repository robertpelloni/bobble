# OmniCore Memory

Ongoing internal architectural observations, codebase traits, and design preferences.

- Core Engine: Rust workspace containing `omnicore-engine` (headless daemon & protocol bridges) and `omnicore-ui` (Tauri desktop app).
- Mobile Shell: React Native + Expo app in `omnicore-mobile/` for iOS and Android, interfacing with the engine daemon over local IPC / WebSocket.
- Protocol Bridges (`omnicore-engine/src/bridges/`):
  - Matrix: `matrix-sdk` integration with homeserver session and sync token storage.
  - Signal: Unix socket IPC to `signald` daemon.
  - Discord: `serenity` REST / gateway client.
  - SimpleX: Unidirectional isolated cryptographic queue broker routing (`simplex.rs`).
  - Tox: DHT bootstrapping and peer-to-peer friend dispatch (`tox.rs`).
  - SMS: Serial AT cellular modem transport for offline fallback (`sms.rs`).
  - iMessage: BlueBubbles REST API and AppleScript relay client (`imessage.rs`).
- Bridge Integration: `BridgeRegistry` and `MultipathRouter` in `omnicore-engine/src/router.rs` managing dynamic multi-bridge dispatch with automated cascade fallback.
- Federation & Multi-Party: `FederationManager` and `VectorClock` in `omnicore-engine/src/federation.rs` enabling decentralized group channel replication.
- Cloud Backup: `ZeroKnowledgeBackup` in `omnicore-engine/src/backup.rs` managing client-side encrypted chunking and manifest generation.
- Decentralized Identity: `IdentityCard` and `IdentityFingerprint` in `omnicore-engine/src/identity.rs` providing exportable, verifiable cryptographic identity cards.
- Privacy & Anonymity: `TorManager` in `omnicore-engine/src/tor.rs` managing SOCKS5 proxy circuits and v3 .onion hidden service endpoints.
- Outbox & Ratchets: `OutboxManager` and `RatchetState` in `omnicore-engine/src/outbox.rs` ensuring offline message queue persistence and forward secrecy key progression.
- Hardware Appliance: `TelemetryCollector` in `omnicore-engine/src/telemetry.rs` monitoring battery status, thermal throttling, and power source failovers for standalone operation.
- VoIP & Calling: `CallSession` in `omnicore-engine/src/call.rs` implementing cross-protocol WebRTC signaling and SDP negotiation.
- Local Discovery: `PeerDiscovery` in `omnicore-engine/src/discovery.rs` generating and parsing UDP beacons for zero-server LAN synchronization.
- Domain Models: Explicit types in `omnicore-engine/src/models.rs` for `ProtocolKind`, `UnifiedContact`, `ProtocolMapping`, `ConversationMessage`, `DeliveryState`, and `RoutingStrategy`.
- Encrypted Storage: SQLCipher with AES-256-GCM encryption (`cipher_page_size=4096`, `kdf_iter=256000`, `foreign_keys=ON`). Schema includes `contacts`, `mappings`, `threads`, `messages`, `attachments`, `key_store`, and `sync_tokens`. Complete design documented in `omnicore-engine/SCHEMA.md`.
- Dispatcher: Multi-path timeout fallback engine in `omnicore-engine/src/dispatcher.rs` executing concurrent queues and automated protocol fallback.
- Desktop UI: React 19 + TypeScript + Vite + TailwindCSS running inside Tauri v2 shell with system tray controls.
- Global Versioning: Centrally tracked in `VERSION.md` (currently 0.4.5).
- WebSocket Daemon IPC: `omnicore-engine/src/server.rs` implements asynchronous JSON-RPC protocol bridging desktop and mobile clients to the headless daemon.
- Federated Event Integrity: `omnicore-engine/src/federation.rs` implements canonical event hashing (`digest`), signing (`sign`), and signature verification (`verify_signature`) to reject forged federated events.
- Build Automation: Consolidated in `scripts/build.sh`, `scripts/run.sh`, `scripts/dev.sh`, and `scripts/test.sh`.
- Mobile Shell Settings: `omnicore-mobile/src/screens/SettingsScreen.tsx` provides mobile interface for Tor routing, engine daemon endpoint, identity cards, and zero-knowledge backups.
- Reputation & Anti-Spam: `omnicore-engine/src/reputation.rs` implements sliding-window rate limiting, cryptographic signature verification rewards, delivery acknowledgment tracking, and blacklist enforcement.
- Causal Ordering & Conflict Resolution: `omnicore-engine/src/federation.rs` implements `CausalRelation` and deterministic tie-breaking (`compare_deterministic`, `sort_events_deterministically`) for decentralized multi-replica convergence.
