#!/usr/bin/env bash
# OmniCore unified launcher (Linux/macOS) — supersedes the five single-purpose
# scripts in this folder. Originals are kept and archived under scripts/archive/.
# Usage: ./scripts/omnicore.sh build|test|dev|run|bundle [target]|status
set -euo pipefail
cd "$(dirname "$0")/.."
cmd="${1:-status}"
case "$cmd" in
  build)
    # SQLCipher needs OpenSSL headers on some distros; fail early with guidance.
    (cd omnicore-ui/ui && npm install && npm run build)
    # Keep the host responsive during heavy Rust builds.
    if command -v nice >/dev/null 2>&1; then nice -n 10 cargo build --workspace; else cargo build --workspace; fi
    echo "=== OmniCore build successful ===" ;;
  test)
    (cd omnicore-ui/ui && npm run build)
    if command -v nice >/dev/null 2>&1; then nice -n 10 cargo test --workspace; else cargo test --workspace; fi
    echo "=== All OmniCore tests passed ===" ;;
  dev) (cd omnicore-ui/ui && npm run dev) ;;
  run) cargo run -p omnicore-ui ;;
  bundle) bash scripts/bundle.sh "${2:-all}" ;;
  status)
    echo "OmniCore version: $(cat VERSION.md)"
    node --version; npm --version; cargo --version
    git status -sb; git log --oneline -3 ;;
  *) echo "Usage: $0 build|test|dev|run|bundle [target]|status"; exit 1 ;;
esac
