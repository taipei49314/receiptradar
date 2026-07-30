#!/usr/bin/env bash
# Install rradar CLI from a source checkout (cargo install).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
echo "Building release rradar from source..."
cargo install --path crates/rradar-cli --force --locked
echo "Installed to cargo bin — ensure ~/.cargo/bin is on PATH"
rradar version --long
rradar doctor
