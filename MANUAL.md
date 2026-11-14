# OmniCore User Manual (v1.1.0)

One page. Everything is on it — there are no other dashboard pages to find.

## The page, in order of value

1. **Conversation (center)** — Pick a person on the left, type, **Send**. The label above the input shows `Send via` (primary network), `Fallback` (automatic backup network), and the active **Routing profile**. A network tag on each message shows what actually carried it; "fallback" in the tag means the primary failed and OmniCore re-routed for you.
2. **People (left)** — One card per person across all their networks (Signal, Matrix, Discord, Simplex, Tox, iMessage, SMS, Nostr). Search by name or network. **+ Person** creates a unified contact — choose the primary network (tried first) and fallback (tried automatically on timeout/failure).
3. **Bridges (right, top)** — One row per network connection with live status, latency, and fallback priority (lower tries earlier). **Connect/Disconnect** toggles a bridge; **Probe all** health-checks degraded circuits. If a bridge is down, messages cascade to the next network instead of failing.
4. **Routing (right, middle)** — Choose how delivery behaves: *Optimal cascade* (default, health+latency ordered), *Hedged racing* (parallel, fastest wins), *Confirmed receipt* (waits for signed acknowledgement), *Exact-once*, *Canary drop detection*, *Percentile bounded*, *Bandwidth throttled*, *Cost & QoS*, and more. Every profile names its engine strategy.
5. **Activity (right, bottom)** — The live ledger: dispatches, fallbacks, probes, Tor and server changes. Check here first when a message seems stuck.
6. **Security & privacy (bottom left)** — Your identity fingerprint (share it so contacts can verify you), signature status, storage/session encryption, encrypted backup, **Tor On/Off**, and **Export identity card (JSON)** for pairing another device.
7. **Groups, mesh & calls (bottom middle)** — Federated group channels (**+ Group**), offline mesh/discovery state, call gateway state, and anti-spam reputation. Groups replicate across networks; the delivery ledger retries only members who have not confirmed.
8. **Engine & system (bottom right)** — Device telemetry, appliance policy, offline queue, daemon endpoint, and the complete engine capability map. Filter by area (Routing, Federation, Calls & Media, Mesh & Discovery, Security, System) to audit exactly what the Rust engine implements — each row names its source symbol.

Every heading and most controls have a **? tooltip** — hover or keyboard-focus it for a plain-language explanation.

## System tray

The native tray icon (created by Tauri) is always available, even with the window hidden: **left-click** shows the window; the menu offers **Show/Hide Dashboard**, **Pause/Resume Routing Workers**, **Probe Bridges**, **Quit Servers Only (keep window)**, and **Quit OmniCore & Servers**. The **Tray** button in the header mirrors the same menu inside the app. "Quit servers only" stops dispatch workers without closing the window; **Quit OmniCore & Servers** exits everything.

## If something seems stuck

1. Look at **Activity** — a `HEALTH_PROBE`, fallback, or error will be newest.
2. Check **Bridges** — is the person's primary network connected? Toggle it, or let the fallback carry the message.
3. Try **Probe all**, then confirm the header says **Engine active** (not paused).
4. On Windows builds from source, SQLCipher needs `OPENSSL_DIR` set — the PowerShell launcher does this automatically (see `scripts/README.md`).
