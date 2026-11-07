# OmniCore TODO

Granular short-term features, immediate bug fixes, and explicit tasks.

- [x] Initialize core engine in Rust.
- [x] Implement secure database with SQLCipher.
- [x] Build dispatcher for cross-protocol message routing.
- [x] Implement ProtocolBridge trait and skeleton bridges (Matrix, Signal, Discord).
- [x] Initialize Tauri desktop UI and configure Cargo workspace.
- [x] Build full UI in React instead of raw HTML.
- [x] Complete matrix bridge implementation.
- [x] Complete signal bridge implementation.
- [x] Complete discord bridge implementation.
- [x] Implement native system tray icon and menu with server management in Tauri.
- [x] Consolidate dashboard UI into an intuitive single-page multi-pane layout.
- [x] Wire all backend bridges and metrics to frontend React dashboard.
- [x] Standardize global version tracking via VERSION.md (v0.4.0).
- [x] Organize build, dev, and test scripts in `scripts/`.
- [x] Define domain models (`ProtocolKind`, `UnifiedContact`, `DeliveryState`) and thread querying in `omnicore-engine`.
- [x] Implement automated timeout-driven fallback dispatching in `omnicore-engine`.
- [x] Harden SQLCipher pragmas and implement `threads`, `attachments`, and `key_store` tables.
- [x] Author comprehensive database schema document (`omnicore-engine/SCHEMA.md`).
- [x] Implement SimpleX Chat bridge in `omnicore-engine/src/bridges/simplex.rs`.
- [x] Implement Tox DHT P2P bridge in `omnicore-engine/src/bridges/tox.rs`.
- [x] Implement Cellular GSM/LTE SMS bridge in `omnicore-engine/src/bridges/sms.rs`.
- [x] Establish React Native / Expo cross-platform mobile shell in `omnicore-mobile/`.
- [x] Implement `BridgeRegistry` and `MultipathRouter` integration across bridges.
- [x] Implement macOS AppleScript / BlueBubbles relay bridge in `omnicore-engine/src/bridges/imessage.rs`.
- [x] Implement local area network P2P peer discovery and beacon handling in `omnicore-engine/src/discovery.rs`.
- [x] Implement cross-protocol voice/video call session negotiation and WebRTC signaling in `omnicore-engine/src/call.rs`.
- [x] Implement standalone hardware power telemetry daemon in `omnicore-engine/src/telemetry.rs`.
- [x] Implement offline outbox queue with exponential backoff and forward secrecy ratchet in `omnicore-engine/src/outbox.rs`.
- [x] Implement Tor onion service routing and SOCKS5 proxy adapter in `omnicore-engine/src/tor.rs`.
- [x] Implement decentralized cryptographic identity card export and signature verification in `omnicore-engine/src/identity.rs`.
- [x] Implement zero-knowledge encrypted cloud backup provider in `omnicore-engine/src/backup.rs`.
- [x] Implement federated group channel state replication with causal vector clocks in `omnicore-engine/src/federation.rs`.
- [x] Implement mobile shell settings with Tor, daemon connection, and cryptographic identity card management in `omnicore-mobile/src/screens/SettingsScreen.tsx`.
- [x] Add decentralized reputation scoring and spam defense heuristics in `omnicore-engine/src/reputation.rs`.
- [x] Implement end-to-end WebSocket proxy bridge connecting omnicore-mobile and omnicore-ui to omnicore-engine daemon in `omnicore-engine/src/server.rs`.
- [x] Implement decentralized key ratchets and pre-key bundles in `omnicore-engine/src/ratchet.rs`.
