#!/usr/bin/env bash
# Cross-platform release binary smoke (Git Bash / Linux / macOS).
# Usage:
#   ./scripts/smoke-release-binary.sh [path/to/rradar]
# Env:
#   RRADAR_FIXTURES  fixtures root (default: fixtures)
#
# Regression (v0.1.0-cli.32/.33 Windows): never write version JSON under
# Git-Bash-only paths like /tmp. Native Windows python cannot open those.
# Pipe JSON on stdin to Python instead.
#
# Also: do not trust a bare `python3` on PATH — Windows Store stubs often
# exit 49 without importing json. Probe candidates that can `import json`.
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

echo "smoke-release-binary | bin=$B fixtures=$FIX"
"$B" version --long
"$B" engines
"$B" release-check --fixtures "$FIX" --quiet
"$B" help >/dev/null

pick_python() {
  local cand
  # Prefer `python` before `python3` on Windows (Store stub).
  for cand in python python3 py; do
    if command -v "$cand" >/dev/null 2>&1; then
      if "$cand" -c "import json" >/dev/null 2>&1; then
        printf '%s\n' "$cand"
        return 0
      fi
    fi
  done
  return 1
}

PY="$(pick_python)" || {
  echo "error: python/python3/py required for schema assert (and must import json)" >&2
  exit 2
}

# Stream version JSON directly into Python (no temp path / encoding mismatch).
"$B" version --json | "$PY" -c "
import json, sys
raw = sys.stdin.buffer.read()
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
