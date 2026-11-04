# OmniCore Memory

Ongoing internal architectural observations, codebase traits, and design preferences.

- Core Engine: Rust workspace containing `omnicore-engine` (headless daemon & protocol bridges) and `omnicore-ui` (Tauri desktop app).
- Domain Models: Explicit types in `omnicore-engine/src/models.rs` for `ProtocolKind`, `UnifiedContact`, `ProtocolMapping`, `ConversationMessage`, `DeliveryState`, and `RoutingStrategy`.
- Encrypted Storage: SQLCipher with AES-256-GCM encryption (`cipher_page_size=4096`, `kdf_iter=256000`, `foreign_keys=ON`). Schema includes `contacts`, `mappings`, `threads`, `messages`, `attachments`, `key_store`, and `sync_tokens`. Complete design documented in `omnicore-engine/SCHEMA.md`.
- Dispatcher: Multi-path timeout fallback engine in `omnicore-engine/src/dispatcher.rs` executing concurrent queues and automated protocol fallback.
- UI: React 19 + TypeScript + Vite + TailwindCSS running inside Tauri v2 shell.
- Consolidated Design: Single-page multi-pane interface consolidating chat feed, identity management, protocol control deck, and routing audit monitor.
- System Tray: Native Tauri v2 `tray-icon` with background worker toggle and graceful server termination.
- Global Versioning: Centrally tracked in `VERSION.md` (currently 0.2.2).
- Build Automation: Consolidated in `scripts/build.sh`, `scripts/run.sh`, `scripts/dev.sh`, and `scripts/test.sh`.
