# bobble (OmniCore)

A messaging aggregator that aims to unify multiple platforms — Signal, SimpleX, Matrix, Tox, Discord, iMessage, and SMS — into a single desktop and mobile client, with multi-path fallback routing across networks (e.g. Signal → Matrix → SMS).

## Status

**Active development (v1.1.0).** Last significant activity: **2026-11-13** — consolidated single-page dashboard representing the full engine (see `CHANGELOG.md` and `MANUAL.md`). Earlier Nov 9–11 work shipped v0.9.8–v1.0.4: bandwidth quota/traffic shaping, circuit auto-reset, federated channel bridge handover, route dispersal, a federated delivery ledger, percentile-bounded routing, and a silent-delivery-drop canary detector.

This is an ambitious, early-stage project. The architecture below is as documented in the repo; no release, packaged build, or evidence of live cross-platform messaging is in the tree. The previous README listed v0.5.2 — it was out of date.

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
