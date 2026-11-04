# OmniCore TODO

Granular short-term features, immediate bug fixes, and explicit tasks.

- [x] Initialize core engine in Rust.
- [x] Implement secure database with SQLCipher.
- [x] Build dispatcher for cross-protocol message routing.
- [x] Implement ProtocolBridge trait and skeleton bridges (Matrix, Signal, Discord).
- [x] Initialize Tauri desktop UI and configure Cargo workspace.
- [x] Build full UI in React instead of raw HTML.
- [x] Complete matrix bridge implementation.
- [x] Complete signal bridge implementation.
- [x] Complete discord bridge implementation.
- [x] Implement native system tray icon and menu with server management in Tauri.
- [x] Consolidate dashboard UI into an intuitive single-page multi-pane layout.
- [x] Wire all backend bridges and metrics to frontend React dashboard.
- [x] Standardize global version tracking via VERSION.md (v0.2.0).
- [x] Organize build, dev, and test scripts in `scripts/`.
- [ ] Add Simplex Chat native C-library binding in `omnicore-engine`.
- [ ] Add Tox DHT bootstrap node integration.
- [ ] Implement macOS AppleScript/BlueBubbles relay client for iMessage.
