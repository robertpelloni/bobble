# OmniCore Changelog

Document every adjustment. Track a single, global version string/date/build number sourced from a single text file (like VERSION.md) rather than hardcoding it into application logic.

- 0.2.0:
  - System Tray Integration: Native desktop tray icon, status monitor, and window show/hide toggles.
  - Consolidated Dashboard: Unified single-page multi-pane interface organizing highest-value messaging, cross-protocol routing, identity management, and live bridge controls.
  - Interactive Protocol Routing: Multi-protocol dispatching with automated fallback selection (Signal, Matrix, Discord, Simplex, SMS).
  - Automation & Build Scripts: Added automated `scripts/build.sh`, `scripts/run.sh`, `scripts/dev.sh`, and `scripts/test.sh`.
  - Comprehensive Tooltips & Explanations: Interactive badges and tooltips added across all UI elements explaining transport encryption and fallback paths.
- 0.1.0: Initial commit, core engine setup, SQLCipher storage, and skeleton protocol bridges.
