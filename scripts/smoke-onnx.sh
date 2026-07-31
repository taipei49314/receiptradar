#!/usr/bin/env bash
# Optional desktop ONNX e2e smoke (weights + ORT required; not default CI).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export RRADAR_MODELS_DIR="${RRADAR_MODELS_DIR:-$ROOT/models}"
export ORT_VERSION="${ORT_VERSION:-1.22.0}"
export RRADAR_FETCH_ORT=1

echo "=== smoke-onnx: fetch models + ORT $ORT_VERSION ==="
./tools/fetch-models.sh

echo "=== smoke-onnx: build --features onnx --release ==="
cargo build -p rradar-cli --features onnx --release
RR="$ROOT/target/release/rradar"

echo "=== smoke-onnx: models verify ==="
"$RR" models verify

echo "=== smoke-onnx: image + sidecar ==="
"$RR" process fixtures/images/familymart_photo.png --explain

echo "=== smoke-onnx: real ONNX ==="
"$RR" process fixtures/images/receipt_en_total89.png --engine onnx --explain

echo "SMOKE_ONNX_OK"
