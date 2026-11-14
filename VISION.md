# OmniCore Vision

Comprehensive, extreme-detail outline of the project's ultimate goal, core foundational concepts, and user-satisfaction design.

OmniCore is a universal, fully featured next-generation universal messaging client that bridges Signal, Simplex, Molly, LINE, Matrix, Tox, Discord, iMessage, RCS, Android Messaging, and SMS.
It uses a Headless Core + Multi-Client UI Architecture built on a Rust universal abstraction layer.

## User-Satisfaction Architecture
1. **Zero Fragmentation**: Users communicate with a single person across any platform without needing to switch apps or guess where the recipient is active.
2. **Autonomous Fallback Routing**: When primary transmission fails or times out, the dispatcher transparently falls back to secondary and tertiary networks (e.g. Signal -> Matrix -> SMS).
3. **Consolidated High-Value Dashboard**: A unified single-page interface gives immediate access to active conversations, contact routing rules, and bridge health without buried menus or scattered windows.
4. **Desktop Native Presence**: An integrated system tray service keeps routing workers running continuously in the background while providing quick window toggles and server management.
5. **Consolidated High-Value Dashboard (v1.1.0)**: A unified single-page interface gives immediate access to active conversations, contact routing rules, and bridge health without buried menus or scattered windows. Former Bridges and Routing Monitor pages were merged into the main page; the bottom band carries Security, Groups/Mesh/Calls, and the full Engine capability map. Ordering principle: the conversation is the product — everything else on the page exists to make one person's message arrive.
6. **Explain Everything In Place**: Every control carries a descriptive label and an accessible tooltip naming what it does, what happens on failure, and (for engine features) the exact Rust symbol that implements it. No feature may exist only in the backend.
7. **Quit What You Started**: The system tray can pause workers, probe bridges, quit servers without closing the window, or quit everything — from the OS tray or the in-app Tray menu.
