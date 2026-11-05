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
- Offline outbox queue with exponential backoff and forward secrecy ratchet state advancement implemented in `omnicore-engine/src/outbox.rs`.
- Local P2P discovery layer implemented in `omnicore-engine/src/discovery.rs` for LAN peer announcements and zero-server sync.
- VoIP and calling module implemented in `omnicore-engine/src/call.rs` for cross-protocol WebRTC signaling and SDP negotiation.
- Hardware appliance telemetry implemented in `omnicore-engine/src/telemetry.rs` for 24/7 battery and power monitoring.
- The desktop shell was expanded using Tauri v2, incorporating native system tray menus, status indicators, and background server termination commands.
- The desktop frontend UI was completely redesigned and consolidated into a high-value single-page multi-pane interface in React 19 + TypeScript + TailwindCSS.
- Cross-platform mobile shell established in `omnicore-mobile/` using React Native / Expo, providing on-device conversation feeds, transport tags, and bridge telemetry.
- Automation scripts were created in `scripts/`: `build.sh`, `test.sh`, `dev.sh`, `run.sh`.
- Global version is governed by `VERSION.md` (0.3.6).
