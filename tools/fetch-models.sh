#!/usr/bin/env bash
# Fetch hash-pinned OCR models (PR-A05). Stub until spike pins artifacts.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODELS="$ROOT/models"
mkdir -p "$MODELS"

MANIFEST="$MODELS/manifest.sha256"
if [[ ! -f "$MANIFEST" ]]; then
  cat <<EOF
No models/manifest.sha256 yet.
After PR-A04 spike, add lines:
  <sha256>  <filename>
And set BASE_URL to the GitHub Release asset prefix.
EOF
  exit 1
fi

BASE_URL="${RRADAR_MODEL_BASE_URL:-}"
if [[ -z "$BASE_URL" ]]; then
  echo "Set RRADAR_MODEL_BASE_URL to release asset base URL" >&2
  exit 1
fi

while read -r hash name; do
  [[ -z "${hash:-}" || "$hash" =~ ^# ]] && continue
  dest="$MODELS/$name"
  if [[ -f "$dest" ]]; then
    echo "exists $name"
    continue
  fi
  echo "fetch $name"
  curl -fsSL "$BASE_URL/$name" -o "$dest"
  echo "$hash  $dest" | sha256sum -c -
done < "$MANIFEST"

echo "ok"
