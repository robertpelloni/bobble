# bobble (OmniCore)

A messaging aggregator that aims to unify multiple platforms — Signal, SimpleX, Matrix, Tox, Discord, iMessage, and SMS — into a single desktop and mobile client, with multi-path fallback routing across networks (e.g. Signal → Matrix → SMS).

## Status

**Active development (v1.17.1).** Last significant activity: **2026-11-16** — proper Tauri CLI release builds with embedded frontend (v1.17.1) and the Expo 58 / React Native 0.88 mobile migration (v1.17.0). See `CHANGELOG.md` and `MANUAL.md` for the full history back to the consolidated single-page dashboard in v1.1.0.

This is an ambitious, early-stage project. A standalone release binary is verified to launch, serve its embedded UI, and route messages through the engine's mock bridges over the local daemon (ws://127.0.0.1:18765); no live cross-platform network delivery has been proven yet — the bridges are engine skeletons without credentialled transports.

## Architecture

Headless Rust core + multi-target UI:

- **`omnicore-engine`** — Rust core: SQLCipher encrypted storage (AES-256-GCM, PBKDF2-HMAC-SHA512), multi-path protocol fallback router, federation with vector clocks and deterministic conflict resolution, Double Ratchet sessions and pre-keys, Tor SOCKS5 routing, zero-knowledge cloud backup, QR identity pairing. Schema documented in `omnicore-engine/SCHEMA.md`.
- **`omnicore-ui`** — Tauri v2 + React 19 desktop client with system tray; one consolidated dashboard page (People | Conversation | Bridges/Routing/Activity | Security, Groups/Mesh/Calls, Engine). User guide: `MANUAL.md`.
- **`omnicore-mobile`** — React Native + Expo app with conversation feeds, bridge status cards, and settings.

Protocol bridges (per repo docs): Signal (IPC), Matrix (SDK), Discord (REST/GW), SimpleX (queue), Tox (DHT), SMS, Nostr (relay pool), and a macOS iMessage relay.

## Build & run

Unified launchers in `scripts/` (see `scripts/README.md`; legacy scripts kept in `scripts/archive/`):

```powershell
.\scripts\omnicore.ps1 build   # frontend + Rust workspace
.\scripts\omnicore.ps1 test    # frontend build + cargo test --workspace
.\scripts\omnicore.ps1 dev     # Vite preview on http://localhost:1420
.\scripts\omnicore.ps1 run     # desktop Tauri app + system tray
```
