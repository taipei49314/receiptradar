#!/usr/bin/env bash
# Dual supply-chain gate: Python license scan + optional cargo-deny.
# Usage: ./scripts/check-supply-chain.sh [--install-deny] [--skip-deny] [--write-inventory]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
SKIP_DENY=0
INSTALL_DENY=0
WRITE_INV=0
QUIET=0
for a in "$@"; do
  case "$a" in
    --skip-deny) SKIP_DENY=1 ;;
    --install-deny) INSTALL_DENY=1 ;;
    --write-inventory) WRITE_INV=1 ;;
    --quiet) QUIET=1 ;;
  esac
done
info() { if [[ "$QUIET" -eq 0 ]]; then echo "$*"; fi; }

info "=== gate 1/2: tools/supply-chain/check_deps.py ==="
PY=(python tools/supply-chain/check_deps.py)
[[ "$QUIET" -eq 1 ]] && PY+=(--quiet)
[[ "$WRITE_INV" -eq 1 ]] && PY+=(--write-inventory)
"${PY[@]}"

if [[ "$SKIP_DENY" -eq 1 ]]; then
  info "=== gate 2/2: cargo-deny SKIPPED ==="
  echo "SUPPLY_CHAIN_DUAL_OK python-only"
  exit 0
fi

if ! command -v cargo-deny >/dev/null 2>&1; then
  if [[ "$INSTALL_DENY" -eq 1 ]]; then
    info "=== installing cargo-deny 0.20.2 ==="
    cargo install cargo-deny --locked --version 0.20.2
  else
    info "=== gate 2/2: cargo-deny not installed (optional) ==="
    echo "SUPPLY_CHAIN_DUAL_OK python-only (cargo-deny missing)"
    exit 0
  fi
fi

info "=== gate 2/2: cargo deny check (deny.toml) ==="
cargo deny check
echo "SUPPLY_CHAIN_DUAL_OK python+deny"
