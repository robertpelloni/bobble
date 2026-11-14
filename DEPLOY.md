# OmniCore Deployment

The latest, highly detailed deployment and environment setup instructions.

## Prerequisites
- Rust toolchain (1.78+ recommended, 2021 edition)
- Node.js (v18+ or v20+) and npm
- Linux System Packages (Debian/Ubuntu):
  `sudo apt-get install -y libglib2.0-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`

## Build
Run the automated build script:
```bash
./scripts/build.sh
```
Or build components individually:
```bash
cd omnicore-ui/ui && npm install && npm run build
cargo build --workspace
```

## Running
Run the desktop app:
```bash
./scripts/run.sh
```
Or start the frontend live preview:
```bash
./scripts/dev.sh
```

## Testing
Run the complete automated test suite:
```bash
./scripts/test.sh
```

## Windows notes (verified 2026-11-13)
- Set `$env:OPENSSL_DIR = 'C:\Program Files\OpenSSL-Win64'` before raw `cargo` commands (bundled SQLCipher), or use `scripts\omnicore.ps1` which auto-detects it.
- `omnicore-ui/icons/icon.ico` is required by tauri-build on Windows (generated from `icon.png`).
- If `npm install` fails with `ERR_SSL_TLSV1_ALERT_PROTOCOL_VERSION` against registry.npmjs.org, install with `--registry https://registry.npmmirror.com` and rewrite the lockfile `resolved` URLs back to registry.npmjs.org (integrity hashes are registry-independent).
- Verified: `npm run build` (tsc + Vite) passes; `cargo test --workspace` = 130 passed; `cargo build -p omnicore-ui` produces `target/debug/omnicore-ui.exe`; Vite dev server serves HTTP 200 on port 1420 (allow ~7s startup).
