# OmniCore Vision

Comprehensive, extreme-detail outline of the project's ultimate goal, core foundational concepts, and user-satisfaction design.

OmniCore is a universal, fully featured next-generation universal messaging client that bridges Signal, Simplex, Molly, LINE, Matrix, Tox, Discord, iMessage, RCS, Android Messaging, and SMS.
It uses a Headless Core + Multi-Client UI Architecture built on a Rust universal abstraction layer.

## User-Satisfaction Architecture
1. **Zero Fragmentation**: Users communicate with a single person across any platform without needing to switch apps or guess where the recipient is active.
2. **Autonomous Fallback Routing**: When primary transmission fails or times out, the dispatcher transparently falls back to secondary and tertiary networks (e.g. Signal -> Matrix -> SMS).
3. **Consolidated High-Value Dashboard**: A unified single-page interface gives immediate access to active conversations, contact routing rules, and bridge health without buried menus or scattered windows.
4. **Desktop Native Presence**: An integrated system tray service keeps routing workers running continuously in the background while providing quick window toggles and server management.
