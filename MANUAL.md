# OmniCore User Manual (v1.16.0)

One page. Everything is on it — there are no other dashboard pages to find.

## The page, in order of value

1. **Conversation (center)** — Pick a person on the left, type, **Send**. The label above the input shows `Send via` (primary network), `Fallback` (automatic backup network), and the active **Routing profile**. A network tag on each message shows what actually carried it, including the profile that delivered it; failed sends are queued in the offline outbox instead of being lost.
2. **People (left)** — One card per person across all their networks (Signal, Matrix, Discord, Simplex, Tox, iMessage, SMS, Nostr), with a **Trust** badge (Neutral/Vetted/Trusted/Untrusted/Blacklisted + score; successful deliveries build trust). Search by name or network. **+ Person** creates a unified contact — choose the primary network (tried first) and fallback (tried automatically on timeout/failure).
3. **Bridges (right, top)** — One row per network connection with live status, latency, and fallback priority (lower tries earlier). **Connect/Disconnect** toggles a bridge; **Probe all** runs real heartbeat probes against degraded circuits and reports recoveries; **Diagnose** evaluates accumulated routing metrics (attempts, latency, circuit state, anomalies such as latency spikes or never-attempted bridges). Routing metrics persist across sends for the life of the app.
4. **Routing (right, middle)** — Choose how delivery behaves: *Optimal cascade* (default, health+latency ordered), *Hedged racing* (parallel, fastest wins), *Confirmed receipt* (waits for signed acknowledgement), *Exact-once*, *Canary drop detection*, *Percentile bounded*, *Bandwidth throttled*, *Cost & QoS*, *Penalty weighted*, *Adaptive retry*. The selected profile genuinely executes on every send, and your choice is remembered across restarts.
5. **Activity (right, bottom)** — The live ledger: dispatches with the delivering network and profile, trust updates, outbox queue/retry/drop events, group posts, bridge handovers, mesh attaches, call start/end, database and daemon events. Check here first when anything seems stuck.
6. **Security & privacy (bottom left)** — Your identity fingerprint (share it so contacts can verify you), signature status, storage/session encryption, **Tor On/Off**, **Export identity card (JSON)** and **QR payload** for pairing another device, and **Create backup / Verify backup**: an encrypted snapshot of contacts, messages, and settings using ChaCha20-Poly1305 AEAD (per-chunk nonces, SHA-256 checksums), keyed by `OMNICORE_DB_KEY`.
7. **Groups, mesh & calls (bottom middle)** — Federated group channels: **+ Group** creates one, the member picker adds people, the composer posts (recorded in the channel's causal history and broadcast to members), and each channel card can **Migrate** it to a different bridge with a signed handover (old bridge becomes fallback). **Call** starts a cross-protocol audio/video call to the selected person (primary → fallback bridge) with **End call** per active call. Mesh peers can be attached (BLE / Wi-Fi Direct / UDP Broadcast) to the local mesh daemon; peer counts, queued forwards, and seen packets are live. Native Bluetooth/Wi-Fi hardware drivers are not built yet, and the panel says so.
8. **Engine & system (bottom right)** — Device telemetry and appliance policy, the **offline outbox** (real queued count; **Retry outbox** re-runs ready messages through the active profile with backoff, dropping after 5 attempts), the daemon endpoint (`ws://127.0.0.1:18765`) with a channel **Subscribe** control for event notifications, and the complete engine capability map. Filter by area to audit exactly what the Rust engine implements — each row names its source symbol.

Every heading and most controls have a **? tooltip** — hover or keyboard-focus it for a plain-language explanation.

## Persistence

Contacts, message history, settings (routing profile, Tor, bridge toggles), and group channels all persist in an encrypted SQLCipher database and are restored on launch. Set `OMNICORE_DB_KEY` for a real passphrase (the app warns in Activity when the default dev key is in use) and `OMNICORE_DB_PATH` to move the database file.

## Mobile

The Expo app mirrors conversations, all 8 bridge networks, and settings (daemon endpoint, Tor, identity, backup). It connects to the engine daemon over WebSocket when reachable and honestly reports "offline — messages stay on this device" when it is not.

## System tray

The native tray icon (created by Tauri) is always available, even with the window hidden: **left-click** shows the window; the menu offers **Show/Hide Dashboard**, **Pause/Resume Routing Workers**, **Probe Bridges**, **Quit Servers Only (keep window)**, and **Quit OmniCore & Servers**. The **Tray** button in the header mirrors the same menu inside the app. "Quit servers only" stops dispatch workers without closing the window; **Quit OmniCore & Servers** exits everything.

## If something seems stuck

1. Look at **Activity** — the newest event (`ROUTE_FAILED`, `OUTBOX_*`, `HEALTH_PROBE`, `DB_PERSIST_ERROR`) usually explains it.
2. Check **Bridges** and run **Diagnose** — is the person's primary network connected and its circuit closed?
3. Try **Probe all** to recover degraded circuits, then **Retry outbox** for queued messages; confirm the header says **Engine active** (not paused).
4. On Windows builds from source, SQLCipher needs `OPENSSL_DIR` set — the PowerShell launcher does this automatically (see `scripts/README.md`). Note the daemon uses port **18765** (8765 was already occupied on the reference machine).
