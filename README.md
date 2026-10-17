# bobble


please create detailed instructions for a comprehensive design for a multi messenger new generation like trillian or pidgin (or a fork of those if they are still the best design and support more plugins or have implemenations already) that uses all messenger platforms including new ones like signal, simplex, molly, line, matrix, tox, discord, and uses imessage and rcs and android messaging and sms and has a mobile build and app and desktop program that is FULLY FEATURED for EVERY PLATFORM and is not missing ANY features from ANY platform, even resorting to sending messages through MULTIPLE platforms to the same users if necessary, and detailed, comprehensive build instructions and prompts to give to google jules to build it on autopilot 

Executive Architecture & Strategy: Project "OmniCore"
Building a universal, fully featured next-generation universal messaging client that bridges Signal, Simplex, Molly, LINE, Matrix, Tox, Discord, iMessage, RCS, Android Messaging, and SMS requires moving away from ancient legacy architectures like Pidgin (libpurple) or Trillian. While Pidgin 3.0 (with GTK4) and its plugin heritage are nostalgic, they lack native support for modern decentralized cryptographic ratchets, end-to-end encrypted (E2EE) multi-device state synchronization, and complex media streaming pipelines required by platforms like Discord or Matrix.   
Linuxiac

Instead, the optimal design choice for a 2026 universal client is a Headless Core + Multi-Client UI Architecture built on a Rust universal abstraction layer, leveraging Matrix as the local/federated synchronization bus, and utilizing isolated containerized bridging daemons (via a modernized Beeper/Matrix-Appservice bridge model extended for local execution).

System Architecture Blueprint
 ┌───────────────────────────────────────────────────────────────┐
 |                   UI LAYER (Cross-Platform)                   |
 |  [React Native / Flutter Mobile]  [Tauri / Rust Desktop App]  |
 └──────────────────────────────┬────────────────────────────────┘
                                │ gRPC / Local IPC Unix Domain Sockets
 ┌──────────────────────────────▼────────────────────────────────┐
 |                 OMNICORE UNIFIED ENGINE (Rust)                |
 |  - Unified Contact Graph & Identity Resolution Engine         |
 |  - Message Routing & Fallback/Multi-Send Dispatcher           |
 |  - Local SQLite Database (Encrypted via SQLCipher)            |
 └──────┬──────────┬──────────┬──────────┬──────────┬──────────┬──┘
        │          │          │          │          │          │
    ┌───▼──┐   ┌───▼──┐   ┌───▼──┐   ┌───▼──┐   ┌───▼──┐   ┌───▼──┐
    | Matrix|   |Signal|   | Simplex|  | Tox  |   |Discord|  |iMsg/ |
    |Bridge|   | Bridge|  | Bridge|  |Bridge|   |Bridge|   | SMS  |
    └──────┘   └──────┘   └──────┘   └──────┘   └──────┘   └──────┘
Core Technology Stack
Core Engine: Rust (Memory-safe, high concurrency, zero-cost abstractions, cross-platform compilation).

Desktop Shell: Tauri 2.0 (Rust backend, web-tech frontend using TypeScript/React for pixel-perfect, fully featured UIs with tiny resource footprints).

Mobile Shell: React Native (New Architecture enabled) or Flutter, communicating with the local Rust core via native foreign function interfaces (FFI/gRPC).

Protocol Bridging Engine: A modular daemon architecture where each messenger runs as an isolated plugin wrapper communicating with the Rust core via standard protocol buffers (Protobuf).

Platform-Specific Integration Strategy
To ensure zero feature loss (reactions, threads, polls, file transfers, inline edits, read receipts, and E2EE verification), the client uses direct protocol wrappers rather than lowest-common-denominator text conversions:

Matrix: Native first-class citizen client implementation using Matrix Rust SDK. Supports full rooms, spaces, E2EE, and widgets.

Signal & Molly: Integration via a modified signald daemon running locally, maintaining its own registered device session or linking as a secondary device. Molly features (hardening, alternate servers) map cleanly via configuration.

Simplex Chat: Embedded simplex-chat Go/Rust library bindings, leveraging out-of-band queue-based addressing for absolute metadata privacy.

Discord: Gateway and HTTP API integration via a high-performance Rust Discord bot/user-bot wrapper (discord-rs), supporting threads, slash commands, voice channel state, and reactions.

Tox: Native integration via c-toxcore bindings, ensuring peer-to-peer decentralized text, A/V calling, and file sharing without central servers.

LINE: Integration via un-official LINE client protocol bridging or Chrome DevTools automation helpers wrapped in a secure headless browser container.

iMessage & RCS & SMS (Android/iOS):

For Apple Ecosystem: Integration with BlueBubbles API or local macOS bridging node for native iMessage routing, tapbacks, and SMS forwarding.

For Android/RCS: Integration via Google Messages web multi-device pairing endpoints and local Telephony API intents for raw SMS/MMS fallback.

Universal Identity & Multi-Platform Fallback Engine
When sending a message to a contact who exists across multiple services, the Multi-Send and Fallback Dispatcher handles delivery according to user-defined priority matrices or auto-failover rules:

Simultaneous Multi-Platform Broadcast: Option to send a single message payload simultaneously to a user's Matrix, Signal, and Discord accounts if they belong to a unified contact card.

Smart Fallback Routing: If a message sent via Signal fails to deliver within 15 seconds (e.g., recipient offline or app uninstalled), the engine automatically reroutes and translates the payload into an SMS or RCS message via the local gateway.

Comprehensive Build Instructions for Development
Phase 1: Environment Setup
Bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Install Node.js, pnpm, and Tauri CLI prerequisites
npm install -g pnpm tauri-cli
Phase 2: Repository Initialization & Workspace Structure
Bash
cargo new omnicore-engine --lib
cd omnicore-engine
# Set up Cargo workspace for core, bridges, and CLI runner
Prompts for Google Jules (Autonomous Builder)
Copy and paste the following prompt packages directly into Google Jules to execute the build pipeline step-by-step on autopilot.

Prompt 1: Core Engine & Unified Storage Schema
Plaintext
Act as an expert systems architect. Initialize the Rust workspace for "OmniCore", a universal multi-messenger client engine. 
1. Create a core database module using SQLCipher and rusqlite to store unified contacts, multi-platform account mappings, message logs, and raw sync tokens securely.
2. Design a thread-safe message routing dispatcher using Tokio channels that receives outbound messages from the UI layer, determines the destination protocol (Signal, Matrix, Simplex, Tox, Discord, iMessage, SMS), and serializes payloads via Protocol Buffers.
3. Write comprehensive unit tests verifying database encryption keys, concurrent message queue processing, and fallback triggers when a primary protocol handler returns a timeout error. Ensure zero unsafe Rust blocks unless strictly required for FFI.
Prompt 2: Protocol Bridge Integration Daemons
Plaintext
Implement the protocol adapter plugins for the OmniCore engine:
1. Integrate the `matrix-rust-sdk` to handle native Matrix rooms, E2EE key verification, and real-time event streaming.
2. Build an IPC wrapper interface for `signald` to manage Signal and Molly message encryption, session linking, media attachments, and read receipts.
3. Implement `c-toxcore` bindings for peer-to-peer Tox messaging and a `discord-rs` gateway client handling guilds, channels, threads, reactions, and markdown formatting.
4. Provide robust error handling, automatic reconnection logic with exponential backoff for dropped socket connections, and mock test fixtures for each protocol adapter.
Prompt 3: Desktop UI & Mobile Cross-Platform Shell
Plaintext
Build the frontend application shell for OmniCore:
1. Initialize a Tauri 2.0 desktop application with a React, TypeScript, and Tailwind CSS frontend. Implement a multi-pane layout featuring a unified chat feed, a consolidated contact list with platform indicator badges, and a rich message composition box supporting markdown, file attachments, and emoji pickers.
2. Connect the Tauri frontend to the Rust OmniCore backend via Tauri commands and event listeners for real-time message pushing.
3. Build a React Native mobile companion app shell that mirrors the desktop state model, util
