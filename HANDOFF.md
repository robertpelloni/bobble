# OmniCore Handoff

Summarize everything learned during the session that was not obvious at the start. Document the entire session history, findings, structural shifts, and system memories.

- Project started with core Rust engine and workspace initialization.
- Core engine has a hardened SQLite + SQLCipher database schema built with rusqlite (documented in `omnicore-engine/SCHEMA.md`).
- SQLCipher pragmas include page size 4096, KDF iterations 256000, and strict foreign key checks.
- Schema now accommodates:
  - `contacts` & `mappings` (unified identity records across protocols)
  - `threads` (direct channels and group conversations)
  - `messages` (unified ledger with delivery state tracking)
  - `attachments` (encrypted media metadata and sha256 checksums)
  - `key_store` (Double Ratchet key material and session tokens)
  - `sync_tokens` (per-platform cursor state)
- Core dispatcher handles cross-protocol message routing utilizing tokio channels with timeout-driven fallback.
- Skeleton shells for Matrix, Signal, and Discord protocols were created using `async-trait`.
- The desktop shell was expanded using Tauri v2, incorporating native system tray menus, status indicators, and background server termination commands.
- The frontend UI was completely redesigned and consolidated into a high-value single-page multi-pane interface in React 19 + TypeScript + TailwindCSS.
- Automation scripts were created in `scripts/`: `build.sh`, `test.sh`, `dev.sh`, `run.sh`.
- Global version is governed by `VERSION.md` (0.2.2).
