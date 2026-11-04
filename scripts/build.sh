#!/usr/bin/env bash
# OmniCore Universal Build Script
set -e

echo "=== Building OmniCore Engine & Desktop UI ==="

# 1. Build Frontend UI Bundle
echo "--> Compiling React Vite UI..."
cd omnicore-ui/ui
npm install
npm run build
cd ../..

# 2. Build Rust Engine & Desktop Binary
echo "--> Compiling Rust workspace..."
cargo build --workspace

echo "=== OmniCore Build Successful ==="
