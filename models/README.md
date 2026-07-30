# OCR models (ONNX RapidOCR)

**Not bundled in git.** Default CI / `cargo test` uses the **mock** engine.

## Quick path (desktop, real OCR)

```powershell
# 1) Weights + ORT shared lib (Windows)
powershell -File tools/fetch-models.ps1 -FetchOrt

# 2) Build CLI with inference linked
cargo build -p rradar-cli --features onnx --release

# 3) Process an image (JPEG/PNG)
$env:RRADAR_MODELS_DIR = "$PWD\models"   # optional if using ./models
.\target\release\rradar.exe process path\to\receipt.jpg --engine onnx --explain
```

```bash
# Linux / macOS
./tools/fetch-models.sh
RRADAR_FETCH_ORT=1 ./tools/fetch-models.sh   # Linux x64 ORT best-effort
cargo build -p rradar-cli --features onnx --release
./target/release/rradar process photo.jpg --engine onnx --explain
```

Without models or without `--features onnx`, `--engine onnx` fails with a **clear multi-line hint** (not a panic).

## Layout after fetch

| File | Role |
|------|------|
| `ch_PP-OCRv4_det_infer.onnx` | Text detection (DB) |
| `ch_PP-OCRv4_rec_infer.onnx` | Recognition (CRNN, CJK-capable pack) |
| `ch_ppocr_mobile_v2.0_cls_infer.onnx` | Angle classifier |
| `ppocr_keys_v1.txt` | Optional dict override (crate has built-in keys) |
| `ort/onnxruntime.dll` (Windows) / `libonnxruntime.so*` | ORT for `load-dynamic` |
| `manifest.sha256` | Optional SHA-256 pins (`tools/fetch-models.*` verifies when present) |

## Cargo feature

| Build | Inference |
|-------|-----------|
| default (no feature) | mock only; onnx name returns structured “not ready” |
| `--features onnx` on `rradar-cli` / `rradar-ocr` | paddle-ocr-rs + ort **load-dynamic** |

Why `load-dynamic`? Prebuilt ORT tarballs for **windows-gnu** are not published by ort-sys; load-dynamic compiles on GNU and MSVC and loads the Microsoft ORT DLL at runtime.

```toml
# crates/rradar-cli
onnx = ["rradar-ocr/onnx"]
```

## Environment

| Variable | Meaning |
|----------|---------|
| `RRADAR_MODELS_DIR` | Models root (default `./models`) |
| `ORT_DYLIB_PATH` | Full path to `onnxruntime` shared library |
| `RRADAR_MODEL_BASE_URL` | Override download base (default HuggingFace `SWHL/RapidOCR`) |
| `RRADAR_FETCH_ORT=1` | Shell script: also try to fetch ORT |
| `ORT_VERSION` | ORT release tag (default `1.20.1`) |

If `ORT_DYLIB_PATH` is unset, `rradar` looks for `models/ort/onnxruntime.{dll,so,dylib}`.

## Source & license

- Weights: [SWHL/RapidOCR](https://huggingface.co/SWHL/RapidOCR) (Apache-2.0 lineage; see `docs/licenses-checklist.md`)
- Runtime: [ONNX Runtime](https://github.com/microsoft/onnxruntime) releases
- Never commit large `.onnx` weights without a release process and license review

## Traditional Chinese notes

The PP-OCRv4 **Chinese** rec pack is simplified-primary; Traditional Chinese receipts are **best-effort** (glyph confusions possible). Product stance aligns with design Orange path: **TW e-invoice QR first**, OCR as assist + manual edit. Fill measured metrics in `docs/spike-ocr-size.md` when device spike runs.

## Doctor

```bash
cargo run -p rradar-cli --features onnx -- doctor
```

Prints per-file model presence, feature flag, and ORT dylib discovery.
