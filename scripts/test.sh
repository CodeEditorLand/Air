#!/usr/bin/env bash
set -euo pipefail

echo "Running Air test suite (unit + integration)..."
cargo test --workspace --all-features

if command -v cargo-tarpaulin >/dev/null 2>&1; then
  echo "Running coverage with cargo-tarpaulin..."
  cargo tarpaulin --out Xml
else
  echo "cargo-tarpaulin not found; skipping coverage run. To enable, install via 'cargo install cargo-tarpaulin'"
fi

echo "Tests complete."
