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
- Tor onion service routing and SOCKS5 proxy adapter (`tor.rs`).
- Decentralized cryptographic identity cards and signature verification (`identity.rs`).
- Zero-knowledge encrypted cloud backup provider (`backup.rs`).
- Federated group channel synchronization with causal vector clocks (`federation.rs`).

## Milestone 4: Cross-Platform Shells, LAN Discovery & Mobile Deployment (Completed)
- React Native / Expo mobile application shell (`omnicore-mobile/`).
- Local P2P network peer discovery and beacon synchronization (`discovery.rs`).
- Multi-target bundling (Desktop + Android + iOS) with automated packaging pipeline (`scripts/bundle.sh`).
- Mobile daemon settings, Tor SOCKS5 proxy, and cryptographic identity pairing.

## Milestone 5: Production Hardening, Key Ratchets & Native Performance (Completed)
- Double Ratchet session persistence and X3DH pre-key bundles in SQLCipher key_store (`ratchet.rs`).
- WebSocket IPC daemon server with session-targeted event broadcasting (`server.rs`).
- Cross-platform federation and multi-tier cascade routing with circuit breakers (`router.rs`, `federation.rs`).
- Matrix and Discord bridge delivery reliability and retry loops (`bridges/`).

## Milestone 6: Autonomous Hardware Appliance & Mesh Expansion (In Progress)
- [x] Standalone compute module appliance telemetry evaluation with automated cellular LTE failover and safe shutdown policies (`telemetry.rs`).
- [x] Multi-hop P2P mesh announcement format with hop limit decrement and transport tagging (`MeshTransportType`).
- [x] In-memory neighbor peer discovery caching (`PeerDiscovery::cache_peer`, `get_cached_peers`).
- [x] Multi-hop store-and-forward mesh packet relay across BLE and Wi-Fi Direct transports (`mesh.rs`).
- [x] BLE background GATT service and Wi-Fi Direct socket daemon coordinator (`MeshTransportDaemon` in `mesh.rs`).
- [ ] Physical Bluetooth HCI adapter binding and Android Wi-Fi P2P JNI drivers.
- [x] End-to-end audio/video call aggregation across WebRTC, Matrix VoIP, and Discord voice gateways (`call.rs`).
- [x] Multi-bridge optimal cascade execution with automated circuit-breaker bypass and metric updates (`router.rs`).
- [x] Protocol bridge reliability hardening across iMessage and SMS Hayes AT modem drivers (`bridges/`).
- [x] Real-time bridge health diagnostics and anomaly detection for routing latency spikes and circuit locks (`telemetry.rs`).
- [x] Advanced RTP jitter buffer with sequence reordering and packet loss concealment pipeline (`call.rs`).
- [x] Native WebRTC audio track encoding and dynamic bit-rate adaptation (`call.rs`).
- [ ] Video track simulcast and VP8/VP9 temporal layer switching.
