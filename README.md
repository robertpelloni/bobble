# bobble (OmniCore)

A messaging aggregator that aims to unify multiple platforms — Signal, SimpleX, Matrix, Tox, Discord, iMessage, and SMS — into a single desktop and mobile client, with multi-path fallback routing across networks (e.g. Signal → Matrix → SMS).

## Status

**Active development (v1.20.2).** Last significant activity: **2026-11-17** — real attachments in the composer (v1.19.0), live LAN peer discovery (v1.20.0), and a network-status refresh fix (v1.20.1). v1.20.2 (2026-11-19) is an honesty pass: the bridges no longer fake delivery (see below). See `CHANGELOG.md` and `MANUAL.md` for the full history back to the consolidated single-page dashboard in v1.1.0.

This is an ambitious, early-stage project. A standalone release binary is verified to launch, serve its embedded UI, and route messages through the engine's router over the local daemon (ws://127.0.0.1:18765); **no live cross-platform network delivery has been proven yet** — all 8 protocol bridges are mocks/skeletons without credentialled transports. As of v1.20.2 the production bridges say so in code: they return errors instead of reporting success for messages that never left the machine, and simulated transports exist only behind explicit `new_mock()` constructors used by tests. Desktop/daemon sends therefore now fail or queue to the outbox honestly until the first live bridge lands (Matrix is the planned first one).

## Architecture

Headless Rust core + multi-target UI:

- **`omnicore-engine`** — Rust core: SQLCipher encrypted storage (AES-256-GCM, PBKDF2-HMAC-SHA512), multi-path protocol fallback router, federation with vector clocks and deterministic conflict resolution, Double Ratchet sessions and pre-keys, Tor SOCKS5 routing, zero-knowledge cloud backup, QR identity pairing. Schema documented in `omnicore-engine/SCHEMA.md`. Note: the router, federation, call, and mesh modules are real code with tests, but they operate in-memory / against mock transports — none of it is verified against live networks or hardware.
- **`omnicore-ui`** — Tauri v2 + React 19 desktop client with system tray; one consolidated dashboard page (People | Conversation | Bridges/Routing/Activity | Security, Groups/Mesh/Calls, Engine). User guide: `MANUAL.md`.
- **`omnicore-mobile`** — React Native + Expo app with conversation feeds, bridge status cards, and settings.

Protocol bridges (all 8 are mock/skeleton — no live transport yet): Signal (signald IPC), Matrix (SDK client constructed, never logged in), Discord (bot HTTP client constructed, never called), SimpleX (queue), Tox (DHT), SMS (modem), Nostr (relay pool), and a macOS iMessage relay.

## Build & run

Unified launchers in `scripts/` (see `scripts/README.md`; legacy scripts kept in `scripts/archive/`):

```powershell
.\scripts\omnicore.ps1 build   # frontend + Rust workspace
.\scripts\omnicore.ps1 test    # frontend build + cargo test --workspace
.\scripts\omnicore.ps1 dev     # Vite preview on http://localhost:1420
.\scripts\omnicore.ps1 run     # desktop Tauri app + system tray
```
