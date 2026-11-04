# OmniCore Changelog

Document every adjustment. Track a single, global version string/date/build number sourced from a single text file (like VERSION.md) rather than hardcoding it into application logic.

- 0.2.1:
  - Core Engine Data Architecture: Added comprehensive domain models in `omnicore-engine/src/models.rs` (`ProtocolKind`, `DeliveryState`, `UnifiedContact`, `ProtocolMapping`, `ConversationMessage`, `RoutingStrategy`).
  - Enhanced Encrypted Database Layer: Added indices, cascade constraints, sync token upsert, delivery state updating, and thread history retrieval in `omnicore-engine/src/db.rs`.
  - Multi-Path Dispatch Fallback Engine: Added `dispatch_with_fallback` in `omnicore-engine/src/dispatcher.rs` supporting timeout and failure-triggered automatic rerouting.
- 0.2.0:
  - System Tray Integration: Native desktop tray icon, status monitor, and window show/hide toggles.
  - Consolidated Dashboard: Unified single-page multi-pane interface organizing highest-value messaging, cross-protocol routing, identity management, and live bridge controls.
  - Interactive Protocol Routing: Multi-protocol dispatching with automated fallback selection (Signal, Matrix, Discord, Simplex, SMS).
  - Automation & Build Scripts: Added automated `scripts/build.sh`, `scripts/run.sh`, `scripts/dev.sh`, and `scripts/test.sh`.
  - Comprehensive Tooltips & Explanations: Interactive badges and tooltips added across all UI elements explaining transport encryption and fallback paths.
- 0.1.0: Initial commit, core engine setup, SQLCipher storage, and skeleton protocol bridges.
