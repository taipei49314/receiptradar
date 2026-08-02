#!/usr/bin/env bash
# Cross-platform release binary smoke (Git Bash / Linux / macOS).
# Usage:
#   ./scripts/smoke-release-binary.sh [path/to/rradar]
# Env:
#   RRADAR_FIXTURES  fixtures root (default: fixtures)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -n "${1:-}" ]]; then
  B="$1"
elif [[ -f target/release/rradar.exe ]]; then
  B=target/release/rradar.exe
elif [[ -f target/release/rradar ]]; then
  B=target/release/rradar
elif command -v rradar >/dev/null 2>&1; then
  B="$(command -v rradar)"
else
  echo "error: rradar binary not found" >&2
  exit 2
fi

FIX="${RRADAR_FIXTURES:-fixtures}"
VJSON="${TMPDIR:-/tmp}/rradar-version-$$.json"
# Windows Git Bash: prefer $TEMP when set
if [[ -n "${TEMP:-}" ]]; then
  VJSON="${TEMP}/rradar-version-$$.json"
elif [[ -n "${TMP:-}" ]]; then
  VJSON="${TMP}/rradar-version-$$.json"
fi

echo "smoke-release-binary | bin=$B fixtures=$FIX"
"$B" version --long
"$B" version --json | tee "$VJSON"
"$B" engines
"$B" release-check --fixtures "$FIX" --quiet
"$B" help >/dev/null

PY=python3
if ! command -v python3 >/dev/null 2>&1; then
  if command -v python >/dev/null 2>&1; then
    PY=python
  else
    echo "error: python3/python required for schema assert" >&2
    exit 2
  fi
fi
"$PY" -c "
import json, sys
p = r'''$VJSON'''
raw = open(p, 'rb').read()
if raw.startswith(b'\xff\xfe'):
    s = raw.decode('utf-16-le')
elif raw.startswith(b'\xfe\xff'):
    s = raw.decode('utf-16-be')
elif raw.startswith(b'\xef\xbb\xbf'):
    s = raw[3:].decode('utf-8')
else:
    s = raw.decode('utf-8')
d = json.loads(s.lstrip('\ufeff'))
schema = int(d.get('ledger_schema') or 0)
assert schema >= 4, d
assert d.get('soft_delete') is True or schema >= 4, d
print(f'schema_ok v{schema} soft_delete={d.get(\"soft_delete\")}')
"
echo "SMOKE_RELEASE_BINARY_OK"
