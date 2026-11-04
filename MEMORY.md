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
- Local Discovery: `PeerDiscovery` in `omnicore-engine/src/discovery.rs` generating and parsing UDP beacons for zero-server LAN synchronization.
- Domain Models: Explicit types in `omnicore-engine/src/models.rs` for `ProtocolKind`, `UnifiedContact`, `ProtocolMapping`, `ConversationMessage`, `DeliveryState`, and `RoutingStrategy`.
- Encrypted Storage: SQLCipher with AES-256-GCM encryption (`cipher_page_size=4096`, `kdf_iter=256000`, `foreign_keys=ON`). Schema includes `contacts`, `mappings`, `threads`, `messages`, `attachments`, `key_store`, and `sync_tokens`. Complete design documented in `omnicore-engine/SCHEMA.md`.
- Dispatcher: Multi-path timeout fallback engine in `omnicore-engine/src/dispatcher.rs` executing concurrent queues and automated protocol fallback.
- Desktop UI: React 19 + TypeScript + Vite + TailwindCSS running inside Tauri v2 shell with system tray controls.
- Global Versioning: Centrally tracked in `VERSION.md` (currently 0.3.3).
- Build Automation: Consolidated in `scripts/build.sh`, `scripts/run.sh`, `scripts/dev.sh`, and `scripts/test.sh`.
