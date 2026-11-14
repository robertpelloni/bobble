# OmniCore Handoff

Summarize everything learned during the session that was not obvious at the start. Document the entire session history, findings, structural shifts, and system memories.

- Project started with core Rust engine and workspace initialization.
- Core engine has a hardened SQLite + SQLCipher database schema built with rusqlite (documented in `omnicore-engine/SCHEMA.md`).
- SQLCipher pragmas include page size 4096, KDF iterations 256000, and strict foreign key checks.
- Protocol bridges are implemented across 7 distinct transports:
  - Matrix (`matrix-sdk`)
  - Signal (`signald` IPC)
  - Discord (`serenity`)
  - SimpleX Chat (`simplex.rs` queue broker)
  - Tox DHT (`tox.rs` P2P transport)
  - SMS Cellular Modem (`sms.rs` AT serial driver)
  - iMessage Relay (`imessage.rs` BlueBubbles/AppleScript)
- Bridge integration layer (`omnicore-engine/src/router.rs`) contains `BridgeRegistry` and `MultipathRouter` executing sequential fallback cascades.
- Decentralized multi-party federation implemented in `omnicore-engine/src/federation.rs` with causal vector clocks.
- Client-side zero-knowledge encrypted cloud backup provider implemented in `omnicore-engine/src/backup.rs`.
- Decentralized identity card export and signature verification implemented in `omnicore-engine/src/identity.rs`.
- Censorship resistance and privacy layer implemented in `omnicore-engine/src/tor.rs` via SOCKS5 proxy routing and v3 .onion hidden service endpoint validation.
- Offline outbox queue with exponential backoff and forward secrecy ratchet state advancement implemented in `omnicore-engine/src/outbox.rs`.
- Local P2P discovery layer implemented in `omnicore-engine/src/discovery.rs` for LAN peer announcements and zero-server sync.
- VoIP and calling module implemented in `omnicore-engine/src/call.rs` for cross-protocol WebRTC signaling and SDP negotiation.
- Hardware appliance telemetry implemented in `omnicore-engine/src/telemetry.rs` for 24/7 battery and power monitoring.
- The desktop shell was expanded using Tauri v2, incorporating native system tray menus, status indicators, and background server termination commands.
- The desktop frontend UI was completely redesigned and consolidated into a high-value single-page multi-pane interface in React 19 + TypeScript + TailwindCSS.
- Cross-platform mobile shell established in `omnicore-mobile/` using React Native / Expo, providing on-device conversation feeds, transport tags, and bridge telemetry.
- Automation scripts were created in `scripts/`: `build.sh`, `test.sh`, `dev.sh`, `run.sh`.
- Global version is governed by `VERSION.md` (1.0.4).
- Silent Delivery Drop Canary Detector: Implemented `CanaryDropDetector` and `MultipathRouter::route_with_canary_drop_detection` in `omnicore-engine/src/router.rs` to detect unacknowledged packet drops and blackholed bridges before standard timeouts expire.
- Dynamic Latency Window & Percentile-Bounded Routing: Implemented `RouteLatencyEstimator` and `MultipathRouter::route_with_percentile_bounded_cascade` in `omnicore-engine/src/router.rs` to compute historical p50/p95 latency envelopes and eliminate tail latency stalls.
- Multi-Party Delivery Ledger: Implemented `FederatedDeliveryLedger` and `FederationManager::broadcast_event_with_delivery_ledger` in `omnicore-engine/src/federation.rs` to track member receipts across multi-bridge mesh relays and avoid duplicate transmissions during selective retries.
- Route Dispersal Balancer: Implemented `RouteDispersalBalancer` and `MultipathRouter::route_with_dispersal_balancing` in `omnicore-engine/src/router.rs` rotating egress across healthy bridges to eliminate socket exhaustion and rate-limiting.
- Federated Channel Dynamic Bridge Handover: Implemented `FederatedChannelBridgeHandover`, `handover_channel_bridge`, and `broadcast_event_with_bridge_handover` in `omnicore-engine/src/federation.rs` enabling decentralized group channel bridge migrations.
- Circuit Auto-Reset & Backoff Decay: Implemented `decay_trips_if_stable`, `rehabilitate_stable_circuits`, and `route_with_rehabilitated_circuits` in `omnicore-engine/src/router.rs` decaying consecutive trip penalties on stable candidate bridges.
- Bandwidth Quota & Traffic Shaper: Implemented `RouteBandwidthShaper` and `MultipathRouter::route_with_bandwidth_throttled_cascade` in `omnicore-engine/src/router.rs` enforcing per-transport sliding-window byte quotas and cascading upon budget exhaustion.
- Cross-Protocol Route Cost & QoS Optimizer: Implemented `RouteCostOptimizer` and `MultipathRouter::route_with_cost_and_qos_optimization` in `omnicore-engine/src/router.rs` balancing financial cost, battery drain profiles, and low-latency transport modes.
- Multi-Bridge Active Health Probing: Implemented `BridgeHeartbeatProber` and `probe_and_recover_circuits`, `route_with_active_health_probing` in `omnicore-engine/src/router.rs` proactively probing open and half-open bridges to test recovery and transition circuits back to `Closed`.
- Multi-bridge egress deduplication ledger and exact-once delivery (`EgressDeduplicationLedger`, `route_with_exact_once_deduplication`) added in `omnicore-engine/src/router.rs`.
- Latency anomaly spike filtering and damped cascade routing (`RouteAnomalyFilter`, `route_with_anomaly_damped_cascade`) added in `omnicore-engine/src/router.rs`.
- Bridge concurrency permit bounding and saturation fallback (`BridgeConcurrencyGuard`, `route_with_concurrency_bounded_cascade`) added in `omnicore-engine/src/router.rs`.
- Cryptographic delivery receipt confirmation and silent drop cascading (`route_with_confirmed_receipt`) added in `omnicore-engine/src/router.rs`.
- Asymmetric network partition resilient broadcasting (`broadcast_event_with_partition_resilience`) added in `omnicore-engine/src/federation.rs`.
- Cross-protocol content transcoding and format sanitization (`PayloadContentTranscoder`, `route_with_transcoding_cascade`) added in `omnicore-engine/src/router.rs`.
- Nostr censorship-resistant relay bridge (`NostrBridge`) added in `omnicore-engine/src/bridges/nostr.rs`.
- Cross-protocol packet fragmentation and MTU adaptation (`PayloadFragmenter`, `route_with_fragmentation_adaptive_cascade`) added in `omnicore-engine/src/router.rs`.
- Store-and-forward mesh relay bridge gateway adapter (`MeshRelayBridgeAdapter`) added in `omnicore-engine/src/mesh.rs`.
- Cross-protocol address book translation (`CrossProtocolAddressBook`) and unified contact routing (`route_unified_message`) added in `omnicore-engine/src/router.rs`.
- Penalty-weighted candidate selection (`select_penalty_weighted_route`) and cascade routing added in `omnicore-engine/src/router.rs`.
- High-throughput batch event broadcasting (`broadcast_batch_events_via_router`) with rate-limiting and stress testing added in `omnicore-engine/src/federation.rs`.
- Flapping network resilience (`route_with_flapping_resilience`) and exponential circuit cooldown backoff added in `omnicore-engine/src/router.rs`.
- Adaptive multi-pass fallback retry router (`route_with_adaptive_fallback_retry`) added in `omnicore-engine/src/router.rs`.
- Multi-bridge mesh relay hedged broadcast (`broadcast_event_via_hedged_racing`) added in `omnicore-engine/src/federation.rs` with multi-platform stress test across 5 bridges.
- Hedged multi-path racing router (`route_hedged_racing`) and dynamic circuit cooldown jitter implemented in `omnicore-engine/src/router.rs`.
- Hardware video decoder pipeline with IDR synchronization (`HardwareVideoDecoder`) added in `omnicore-engine/src/call.rs`.
- Replay attack protection with nonce caching (`NonceReplayCache`) added in `omnicore-engine/src/router.rs`.
- Hardware video encoder abstraction with automatic software fallback (`HardwareVideoEncoder`) added in `omnicore-engine/src/call.rs`.
- Video spatial downsampling and RGBA to Planar I420 conversion (`VideoColorConverter`) added in `omnicore-engine/src/call.rs`.
- Zero-copy video frame pool and buffer reclamation (`VideoFramePool`) added in `omnicore-engine/src/call.rs`.
- Cryptographic payload digest integrity verification (`PayloadIntegrityGuard`) added in `omnicore-engine/src/router.rs`.
- End-to-end encrypted screen sharing pipeline (`ScreenSharePipeline`) added in `omnicore-engine/src/call.rs`.
- Video simulcast layer adaptation and temporal resolution switching (`VideoSimulcastController`) added in `omnicore-engine/src/call.rs`.
- Starvation prevention and dynamic candidate probing added in `omnicore-engine/src/router.rs`.
- Real-time bridge health monitoring and anomaly detection (`BridgeHealthMonitor`) added in `omnicore-engine/src/telemetry.rs`.
- Dynamic audio bit-rate scaling and frame duration adaptation (`BitrateController`) added in `omnicore-engine/src/call.rs`.
- `IMessageBridge` and `SmsBridge` reliability hardening and iMessage-to-SMS fallback tests added in `omnicore-engine`.
- Mesh cache pruning and `MeshTransportDaemon` multi-channel forwarding added to `omnicore-engine/src/mesh.rs`.
- RTP Jitter Buffer and Packet Loss Concealment (PLC) added to `omnicore-engine/src/call.rs`.
- Optimal cascade routing with circuit-breaker bypass and real-time metric updates added to `omnicore-engine/src/router.rs`.
- Cross-protocol call and media gateway aggregation between WebRTC, Matrix VoIP, and Discord added to `omnicore-engine/src/call.rs`.
- Multi-hop store-and-forward mesh packet routing, loop detection, and deduplication added to `omnicore-engine/src/mesh.rs`.
- Milestone 6 multi-hop mesh propagation, hop decrementing, and neighbor peer caching added to `omnicore-engine/src/discovery.rs`.
- Milestone 6 compute module hardware appliance failovers and safe shutdown logic added to `omnicore-engine/src/telemetry.rs`.
- Milestone 5 production hardening completed, establishing Milestone 6 on ROADMAP.md.
- WebSocket event broadcasting with channel-subscription filtering added to `omnicore-engine/src/server.rs`.
- Circuit breaker state machine and timeout avoidance added to `omnicore-engine/src/router.rs`.
- Multi-protocol adaptive latency routing and dynamic bridge health reordering implemented in `omnicore-engine/src/router.rs`.
- Tox-to-SMS and SimpleX-to-Matrix fallback cascades tested in `omnicore-engine/src/router.rs`.
- Ratchet session SQLCipher persistence and ephemeral pre-key generation implemented in `omnicore-engine/src/ratchet.rs`.
- 4-node multi-bridge mesh broadcast integration test added to `omnicore-engine/src/federation.rs`.
- Matrix and Discord protocol bridge reliability and retry loops hardened in `omnicore-engine/src/bridges/`.
- Root README.md thoroughly updated with comprehensive architecture diagrams, SQLCipher schema, and script guides.
- Federated event router delivery gateway implemented in `omnicore-engine/src/federation.rs`.
- Milestone 4 completed with multi-target bundling automation in `scripts/bundle.sh`.
- Decentralized QR identity card pairing and mutual handshake verification added to `omnicore-engine/src/identity.rs`.
- Multi-platform federated message delivery and delta synchronization test added to `omnicore-engine/src/federation.rs`.
- Multi-tier protocol cascade fallback testing and BridgeRegistry query inspection added to `omnicore-engine/src/router.rs`.
- Decentralized key ratchets and pre-key bundles implemented in `omnicore-engine/src/ratchet.rs`.
- End-to-end WebSocket IPC daemon server implemented in `omnicore-engine/src/server.rs`.
- Cryptographic event signatures and tamper verification added to `omnicore-engine/src/federation.rs`.
- Mobile shell settings screen implemented in `omnicore-mobile/src/screens/SettingsScreen.tsx` wired into `omnicore-mobile/App.tsx`.
- Reputation & Anti-Spam engine implemented in `omnicore-engine/src/reputation.rs` with sliding-window rate limiting, cryptographic signature verification rewards, delivery acknowledgment tracking, and blacklist enforcement.
- Causal conflict resolution and deterministic multi-replica event ordering implemented in `omnicore-engine/src/federation.rs`.

---

## Session 2026-11-13 (v1.1.0) — [muse-wonder-1.0 in Muse Code in C:\Users\hyper\workspace\bobble on laptop]

Coordinator identity for cross-agent handoff: **[muse-wonder-1.0 in Muse Code in C:\Users\hyper\workspace\bobble on laptop]**. Edge CDP 9222 was live with muse.ai, ChatGPT, and Gemini tabs open; no messages were injected into those other agents' chats (would disrupt their sessions) — this file is the coordination surface. Successors: read this section first.

### What was done
- **Dashboard consolidated to one page** (`omnicore-ui/ui/src/App.tsx`, rewritten): People (left) | Conversation (center) | Bridges + Routing + Activity (right rail) | Security & Privacy, Groups/Mesh/Calls, Engine & System (bottom band). Former Bridges and Routing Monitor tabs are gone as separate views; nothing was dropped — groups/channels, mesh, calls, reputation, outbox, telemetry, daemon, and all 23 router cascades are represented with labels + accessible "?" tooltips citing engine symbols.
- **Backend exposure** (`omnicore-ui/src/main.rs`): new Tauri commands `get_capabilities`, `get_routing_profiles`, `set_routing_profile`, `get_security_status`, `toggle_tor`, `export_identity_card`, `get_network_status`, `get_activity_log`, `get_channels`, `create_channel`, `run_bridge_probe`, using real engine types (IdentityCard, TorManager, TelemetryCollector/ApplianceConfig). Nostr added as the 8th bridge. Activity log records sends, profile changes, Tor toggles, probes, channel creation, tray actions.
- **System tray completed**: native menu = Show/Hide, Pause/Resume Routing Workers, Probe Bridges, Quit Servers Only (keep window), Quit OmniCore & Servers. In-app Tray button mirrors it. Left-click tray icon shows/focuses the window.
- **Scripts unified**: `scripts/omnicore.ps1` + `scripts/omnicore.sh` (build/test/dev/run/bundle/status; heavy builds at BelowNormal priority). Originals untouched and copied to `scripts/archive/`. `scripts/README.md` documents everything. Vite boilerplate `App.css` (unimported) archived to `omnicore-ui/ui/archive/`.
- **Version**: 1.1.0 in VERSION.md, both Cargo crates, desktop + mobile package.json. Commit message carries the version (see git log).

### What was broken and got fixed
1. Frontend never built: Tailwind v4 installed, v3 PostCSS wiring. Fixed via `@tailwindcss/postcss` + `@import "tailwindcss"`. `npm run build` now passes (tsc + Vite).
2. Windows Rust build: `OPENSSL_DIR` unset broke bundled SQLCipher; launchers auto-detect `C:\Program Files\OpenSSL-Win64`.
3. `omnicore-ui/icons/icon.ico` missing (tauri-build Windows requirement) — generated from icon.png.
4. `omnicore-mobile/tsconfig.json` missing — added (Expo base); mobile `tsc --noEmit` passes.
5. Dashboard hid most of the engine: only chat/contacts/bridges/metrics were wired; Nostr missing on desktop.

### Verified this session (all observed, not assumed)
- `cargo test --workspace`: **130 passed, 0 failed** (127 engine + 3 UI), OPENSSL_DIR set.
- `cargo build -p omnicore-ui`: produces `target/debug/omnicore-ui.exe` (16.4 MB).
- `npm run build` in omnicore-ui/ui: passes. Vite dev server: HTTP 200 on http://localhost:1420 (needs ~7s to become ready).
- Mobile: `npx tsc --noEmit` passes.
- NOT live-verified: native tray click-through on a real desktop, Tauri GUI chat round-trip, Expo mobile run, live bridge traffic (bridges are engine skeletons/mocks — no real Signal/Matrix/Discord delivery proven).

### What is not working / not fully running (honest list)
- **Routing profile picker records the choice but does not yet change Dispatcher execution** in the desktop shell. Next task #1.
- Desktop/mobile poll via Tauri invoke; **WebSocketServer event subscription is not wired** into either client.
- **Mesh shows 0 peers**: native Bluetooth HCI + Android Wi-Fi Direct JNI drivers do not exist yet (engine mesh logic is unit-tested only).
- Bridges remain skeleton/mock transports; no live cross-platform message has been proven end-to-end.
- `registry.npmjs.org` rejects Node TLS on this machine (`ERR_SSL_TLSV1_ALERT_PROTOCOL_VERSION`); PowerShell reaches it fine. Workaround used: npmmirror registry + lockfile URL normalization. User could fix proxy/TLS policy; no further probing done.
- Dependabot branch `origin/dependabot/npm_and_yarn/omnicore-ui/ui/npm_and_yarn-4bb469df06` shows no diff vs main (already superseded). Jules branch is fully merged into main. No submodules exist in this repo (Section 2 submodule steps: N/A).

### What's next (priority order)
1. Map dashboard routing profiles to real MultipathRouter execution in the desktop shell; show per-message strategy in Activity.
2. Wire WebSocketServer subscriptions into desktop + mobile; retire polling.
3. Live tray verification pass on Windows desktop; then a real two-bridge delivery proof (Matrix + one fallback).
4. Native mesh drivers (HCI/JNI) — the only engine feature with no path to hardware.

### What would help from the user
- Whether live bridge credentials (Matrix homeserver, signald, Discord token) should be provisioned for an end-to-end proof, and on which host.
- Confirmation that npmmirror-as-fallback is acceptable, or a proxy/TLS fix for registry.npmjs.org.
- Whether the 15-minute autopilot cron (job b8144614) should keep running after this session.
