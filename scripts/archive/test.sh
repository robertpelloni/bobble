#!/usr/bin/env bash
# OmniCore Workspace Test Script
set -e

echo "=== Running OmniCore Test Suite ==="

# 1. Typecheck and build frontend
echo "--> Testing frontend build..."
cd omnicore-ui/ui
npm run build
cd ../..

# 2. Run Rust workspace tests
echo "--> Running Rust workspace unit and integration tests..."
cargo test --workspace

echo "=== All OmniCore Tests Passed Successfully ==="
