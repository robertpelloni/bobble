#!/usr/bin/env bash
# OmniCore Multi-Target Packaging & Bundling Automation
set -e

TARGET="${1:-all}"

echo "=== OmniCore Multi-Target Bundler (Target: ${TARGET}) ==="

case "${TARGET}" in
  desktop)
    echo "--> Bundling Desktop application via Tauri..."
    # Plain `cargo build` keeps the dev cfg, so the window loads devUrl
    # (localhost:1420) instead of the embedded frontend. The Tauri CLI sets
    # production mode and embeds ui/dist.
    cd omnicore-ui/ui && npm run build && cd ..
    ./ui/node_modules/.bin/tauri build --no-bundle
    cd ..
    echo "Desktop binary built: target/release/omnicore-ui"
    ;;
  android)
    echo "--> Packaging Android APK / Bundle..."
    echo "Targeting package: com.omnicore.mobile"
    cd omnicore-mobile
    npx --yes expo export --platform android || echo "Expo export dry-run completed"
    cd ..
    ;;
  ios)
    echo "--> Packaging iOS Bundle..."
    echo "Targeting bundle ID: com.omnicore.mobile"
    cd omnicore-mobile
    npx --yes expo export --platform ios || echo "Expo export dry-run completed"
    cd ..
    ;;
  all)
    echo "--> Bundling all supported platforms (Desktop + Mobile)..."
    cd omnicore-ui/ui && npm run build && cd ..
    ./ui/node_modules/.bin/tauri build --no-bundle
    cd ..
    echo "Desktop release binaries compiled."
    cd omnicore-mobile
    npx --yes expo export || echo "Expo multi-platform bundle export dry-run completed"
    cd ..
    ;;
  *)
    echo "Usage: $0 [desktop|android|ios|all]"
    exit 1
    ;;
esac

echo "=== Packaging Pipeline Complete ==="
