#!/usr/bin/env bash
# ReceiptRadar recordable demo (Unix)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
HOME_DIR="$ROOT/target/demo"
mkdir -p "$HOME_DIR"
export RRADAR_HOME="$HOME_DIR"
export RRADAR_DB="$HOME_DIR/ledger.db"
export RRADAR_FAST_BACKUP=1
export RRADAR_FIXTURES="$ROOT/fixtures"

echo "=== ReceiptRadar demo (scripts/demo.sh) ==="
cargo run -q -p rradar-cli -- demo --fixtures "$RRADAR_FIXTURES" --db "$RRADAR_DB"
echo "DEMO_SCRIPT_OK"
