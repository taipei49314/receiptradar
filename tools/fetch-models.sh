#!/usr/bin/env bash
# Fetch hash-pinned OCR models (PR-A05) from HuggingFace SWHL/RapidOCR.
# Optional: also fetch ONNX Runtime shared library for load-dynamic builds.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODELS="${RRADAR_MODELS_DIR:-$ROOT/models}"
mkdir -p "$MODELS" "$MODELS/ort"

# Default base: HuggingFace resolve URLs (override with RRADAR_MODEL_BASE_URL).
HF_BASE="${RRADAR_MODEL_BASE_URL:-https://huggingface.co/SWHL/RapidOCR/resolve/main}"

# name|relative_url_path (or empty if name is used as path under PP-OCRv4)
declare -a FILES=(
  "ch_PP-OCRv4_det_infer.onnx|PP-OCRv4/ch_PP-OCRv4_det_infer.onnx"
  "ch_PP-OCRv4_rec_infer.onnx|PP-OCRv4/ch_PP-OCRv4_rec_infer.onnx"
  "ch_ppocr_mobile_v2.0_cls_infer.onnx|PP-OCRv1/ch_ppocr_mobile_v2.0_cls_infer.onnx"
)

MANIFEST="$MODELS/manifest.sha256"
if [[ ! -f "$MANIFEST" ]]; then
  # Bootstrap empty pin file; after first successful download we recommend hashing.
  cat >"$MANIFEST" <<'EOF'
# sha256  filename  (fill after first trusted download; fetch verifies when non-comment)
# Generate: sha256sum models/*.onnx
EOF
  echo "created $MANIFEST (hashes optional until you pin them)"
fi

fetch_one() {
  local name="$1" rel="$2"
  local dest="$MODELS/$name"
  if [[ -f "$dest" ]]; then
    echo "exists $name"
  else
    local url
    if [[ "$HF_BASE" == *"huggingface.co"* ]]; then
      url="$HF_BASE/$rel"
    else
      url="$HF_BASE/$name"
    fi
    echo "fetch $name"
    echo "  from $url"
    curl -fL --retry 3 --retry-delay 2 -o "$dest" "$url"
  fi
  # Verify if pin present in manifest
  if grep -qE "^[0-9a-fA-F]{64}[[:space:]]+$name\$" "$MANIFEST" 2>/dev/null; then
    (cd "$MODELS" && grep -E "^[0-9a-fA-F]{64}[[:space:]]+$name\$" "$MANIFEST" | sha256sum -c -)
  fi
}

for entry in "${FILES[@]}"; do
  name="${entry%%|*}"
  rel="${entry#*|}"
  fetch_one "$name" "$rel"
done

# Optional ORT runtime (load-dynamic)
if [[ "${RRADAR_FETCH_ORT:-0}" == "1" ]]; then
  echo "ORT fetch: set RRADAR_FETCH_ORT=1 with platform-specific URLs (see models/README.md)"
  OS="$(uname -s)"
  ARCH="$(uname -m)"
  # Microsoft ONNX Runtime CPU releases (best-effort; pin in release process)
  # Match ort 2.0.0-rc.10 expectation (1.22.x).
  ORT_VER="${ORT_VERSION:-1.22.0}"
  case "$OS-$ARCH" in
    Linux-x86_64)
      ORT_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VER}/onnxruntime-linux-x64-${ORT_VER}.tgz"
      TMP="$(mktemp -d)"
      curl -fL "$ORT_URL" -o "$TMP/ort.tgz"
      tar -xzf "$TMP/ort.tgz" -C "$TMP"
      cp "$TMP"/onnxruntime-linux-x64-"${ORT_VER}"/lib/libonnxruntime.so* "$MODELS/ort/" || true
      rm -rf "$TMP"
      echo "ORT libs in $MODELS/ort — export ORT_DYLIB_PATH=\$MODELS/ort/libonnxruntime.so"
      ;;
    Darwin-*)
      echo "macOS: download onnxruntime from GitHub releases into models/ort/ and set ORT_DYLIB_PATH"
      ;;
    *)
      echo "Unknown OS for auto ORT; see models/README.md"
      ;;
  esac
fi

echo "ok — models in $MODELS"
echo "next: cargo run -p rradar-cli --features onnx -- process photo.jpg --engine onnx"
echo "      (set ORT_DYLIB_PATH if not using models/ort auto-detect)"
