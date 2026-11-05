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

## Milestone 3: Protocol Bridges & Multipath Routing Integration (Completed)
- Matrix homeserver bridge (`matrix-sdk`) with sync token persistence.
- Signal Protocol IPC daemon bridge (`signald`).
- Discord REST and gateway bridge (`serenity`).
- SimpleX Chat cryptographic queue bridge (`simplex.rs`).
- Tox DHT P2P transport module (`tox.rs`).
- GSM/LTE Cellular SMS modem bridge (`sms.rs`).
- macOS AppleScript / BlueBubbles iMessage relay bridge (`imessage.rs`).
- Dynamic `BridgeRegistry` and `MultipathRouter` executing sequential fallback cascades.
- WebRTC & Matrix VoIP cross-protocol call session negotiation (`call.rs`).
- Standalone hardware appliance power and health telemetry (`telemetry.rs`).
- Offline outbox queue with exponential backoff and forward secrecy ratchet (`outbox.rs`).

## Milestone 4: Cross-Platform Shells, LAN Discovery & Mobile Deployment (In Progress)
- React Native / Expo mobile application shell (`omnicore-mobile/`).
- Local P2P network peer discovery and beacon synchronization (`discovery.rs`).
- Multi-target bundling (Desktop + Android + iOS).
