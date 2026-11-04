# OmniCore Roadmap

Major, long-term structural milestones and structural plans.

## Milestone 1: Universal Abstraction Engine & Core Crypto (Completed)
- Rust-based headless engine (`omnicore-engine`).
- SQLCipher cryptographic database with contact, mapping, thread, attachment, key_store, sync token, and message tables.
- Async tokio message dispatcher with multi-protocol fallback routing.
- Strong domain models for cross-protocol identity resolution and message delivery states.
- Fully documented database architecture in `omnicore-engine/SCHEMA.md`.

## Milestone 2: Desktop Shell & System Tray Integration (Completed)
- Tauri v2 desktop integration with native system tray icon and background server quit controls.
- Consolidated single-page multi-pane React dashboard prioritizing highest-value messaging, cross-protocol routing, and bridge management.
- Live bridge status control deck and routing monitor.

## Milestone 3: Native Protocol Implementations (In Progress)
- Native bindings for SimpleX Chat isolated broker.
- Full Matrix E2EE Olm/Megolm session persistence.
- Signal Protocol Rust ratchet implementation (direct libsignal / signald IPC).
- Tox DHT P2P transport module.
- iMessage and SMS relay integration.

## Milestone 4: Mobile Shells & Peer Discovery (Future)
- React Native / Flutter mobile clients connecting to the omnicore-engine daemon.
- P2P local network contact synchronization and direct LAN message routing.
