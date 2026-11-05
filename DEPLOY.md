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
