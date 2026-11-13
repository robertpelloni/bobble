# bobble (OmniCore)

A messaging aggregator that aims to unify multiple platforms — Signal, SimpleX, Matrix, Tox, Discord, iMessage, and SMS — into a single desktop and mobile client, with multi-path fallback routing across networks (e.g. Signal → Matrix → SMS).

## Status

**Active development (v1.0.4).** Last significant activity: **2026-11-12** — merge of a Jules feature branch (Protocol #281). Nov 9–11 work shipped v0.9.8–v1.0.4: bandwidth quota/traffic shaping, circuit auto-reset, federated channel bridge handover, route dispersal, a federated delivery ledger, percentile-bounded routing, and a silent-delivery-drop canary detector.

This is an ambitious, early-stage project. The architecture below is as documented in the repo; no release, packaged build, or evidence of live cross-platform messaging is in the tree. The previous README listed v0.5.2 — it was out of date.

## Architecture

Headless Rust core + multi-target UI:

- **`omnicore-engine`** — Rust core: SQLCipher encrypted storage (AES-256-GCM, PBKDF2-HMAC-SHA512), multi-path protocol fallback router, federation with vector clocks and deterministic conflict resolution, Double Ratchet sessions and pre-keys, Tor SOCKS5 routing, zero-knowledge cloud backup, QR identity pairing. Schema documented in `omnicore-engine/SCHEMA.md`.
- **`omnicore-ui`** — Tauri v2 + React 19 desktop client with system tray; single-page multi-pane dashboard (Unified Chat Hub, Identity Manager, Protocol Bridge Deck, Routing Monitor).
- **`omnicore-mobile`** — React Native + Expo app with conversation feeds, bridge status cards, and settings.

Protocol bridges (per repo docs): Signal (IPC), Matrix (SDK), Discord (REST/GW), SimpleX (queue), Tox (DHT), SMS, and a macOS iMessage relay.

## Build & run

Helper scripts in `scripts/`:

```bash
bash scripts/build.sh    # build React frontend + compile Cargo workspace
bash scripts/test.sh     # full test suite (TypeScript build + Cargo workspace tests)
bash scripts/dev.sh      # development preview server
bash scripts/run.sh      # run desktop Tauri application
bash scripts/bundle.sh all   # package multi-target bundles (desktop, android, ios)
```
