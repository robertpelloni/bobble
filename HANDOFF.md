# OmniCore Handoff

Summarize everything learned during the session that was not obvious at the start. Document the entire session history, findings, structural shifts, and system memories.

- Project started with core Rust engine and workspace initialization.
- Core engine has a hardened SQLite + SQLCipher database schema built with rusqlite (documented in `omnicore-engine/SCHEMA.md`).
- SQLCipher pragmas include page size 4096, KDF iterations 256000, and strict foreign key checks.
- Protocol bridges are implemented across 6 distinct transports:
  - Matrix (`matrix-sdk`)
  - Signal (`signald` IPC)
  - Discord (`serenity`)
  - SimpleX Chat (`simplex.rs` queue broker)
  - Tox DHT (`tox.rs` P2P transport)
  - SMS Cellular Modem (`sms.rs` AT serial driver)
- The desktop shell was expanded using Tauri v2, incorporating native system tray menus, status indicators, and background server termination commands.
- The desktop frontend UI was completely redesigned and consolidated into a high-value single-page multi-pane interface in React 19 + TypeScript + TailwindCSS.
- Phase 3 cross-platform mobile shell established in `omnicore-mobile/` using React Native / Expo, providing on-device conversation feeds, transport tags, and bridge telemetry.
- Automation scripts were created in `scripts/`: `build.sh`, `test.sh`, `dev.sh`, `run.sh`.
- Global version is governed by `VERSION.md` (0.3.0).
