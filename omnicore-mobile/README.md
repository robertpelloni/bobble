# OmniCore Mobile Shell

Cross-platform mobile application shell built using React Native / Expo, designed to interface seamlessly with the `omnicore-engine` daemon over local IPC / WebSocket.

## Architecture

- **Headless Backend**: Communicates with the local Rust engine running on-device (via Tauri mobile or standalone FFI) or over an encrypted local network proxy.
- **Unified Identity & Conversations**: Group chats and 1:1 threads aggregate accounts across Signal, Matrix, Discord, SimpleX, Tox, and SMS.
- **Protocol Telemetry**: Real-time monitoring of active protocol daemons, queue brokers, and fallback routing priorities.

## Project Structure
- `App.tsx`: Main application shell with tab-based routing between Conversations and Bridge Deck.
- `src/screens/ConversationScreen.tsx`: Active messaging thread with multi-path routing badges, automatic fallback indicators, and message dispatcher.
- `src/screens/BridgeDeckScreen.tsx`: Live telemetry dashboard for protocol bridges.
- `src/components/ProtocolBadge.tsx`: Color-coded transport tags.

## Development & Execution
```bash
npm install
npm start
```
