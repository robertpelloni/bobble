# OmniCore Memory

Ongoing internal architectural observations, codebase traits, and design preferences.

- Core Engine: Rust workspace containing `omnicore-engine` (headless daemon & protocol bridges) and `omnicore-ui` (Tauri desktop app).
- Encrypted Storage: SQLCipher with AES-256-GCM encryption for unified identity records and message history.
- UI: React 19 + TypeScript + Vite + TailwindCSS running inside Tauri v2 shell.
- Consolidated Design: Single-page multi-pane interface consolidating chat feed, identity management, protocol control deck, and routing audit monitor.
- System Tray: Native Tauri v2 `tray-icon` with background worker toggle and graceful server termination.
- Global Versioning: Centrally tracked in `VERSION.md` (currently 0.2.0).
- Build Automation: Consolidated in `scripts/build.sh`, `scripts/run.sh`, `scripts/dev.sh`, and `scripts/test.sh`.
