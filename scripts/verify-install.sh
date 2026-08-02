#!/usr/bin/env bash
# Post-install / release binary verification (local-only).
# Usage: ./scripts/verify-install.sh [path/to/rradar]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BIN="${1:-}"
if [[ -z "$BIN" ]]; then
  if command -v rradar >/dev/null 2>&1; then
    BIN="$(command -v rradar)"
  elif [[ -x target/release/rradar ]]; then
    BIN=target/release/rradar
  else
    echo "rradar not found — pass path or install first" >&2
    exit 1
  fi
fi
FIX="${RRADAR_FIXTURES:-fixtures}"
echo "verify-install | bin=$BIN"
"$BIN" version --long
# Require schema v4 soft-delete builds for install gate.
if command -v python3 >/dev/null 2>&1; then
  SCHEMA=$("$BIN" version --json | python3 -c "import sys,json; print(json.load(sys.stdin).get('ledger_schema',0))")
  if [[ "${SCHEMA:-0}" -lt 4 ]]; then
    echo "ledger_schema $SCHEMA < 4" >&2
    exit 1
  fi
  echo "schema v$SCHEMA soft_delete ok"
fi
"$BIN" engines
"$BIN" release-check --fixtures "$FIX"
echo "VERIFY_INSTALL_OK"
