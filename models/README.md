# OCR models (ONNX RapidOCR)

**Not bundled in git.** Default CI / `cargo test` uses the **mock** engine.  
**A05 pins are frozen** in [`manifest.sha256`](./manifest.sha256) (hashes only).

## Pinned pack (desktop default)

Source: [SWHL/RapidOCR](https://huggingface.co/SWHL/RapidOCR) on HuggingFace (Apache-2.0 lineage).

| File | Role | SHA-256 (prefix) | ~Size |
|------|------|------------------|-------|
| `ch_PP-OCRv4_det_infer.onnx` | Text detection (DB) | `d2a7720d45a5…` | ~4.5 MiB |
| `ch_PP-OCRv4_rec_infer.onnx` | Recognition (CRNN, CJK) | `48fc40f24f6d…` | ~10 MiB |
| `ch_ppocr_mobile_v2.0_cls_infer.onnx` | Angle classifier | `e47acedf6632…` | ~0.7 MiB |

Full digests: `manifest.sha256`. After download:

```bash
rradar models verify
# or
cargo run -p rradar-cli -- models verify
```

## Quick path (desktop, real OCR)

```powershell
# 1) Weights (+ optional ORT shared lib on Windows)
powershell -File tools/fetch-models.ps1
powershell -File tools/fetch-models.ps1 -FetchOrt
# rewrite pins after a trusted re-download:
# powershell -File tools/fetch-models.ps1 -WritePins

# 2) Build CLI with inference linked
cargo build -p rradar-cli --features onnx --release

# 3) Process an image (JPEG/PNG)
$env:RRADAR_MODELS_DIR = "$PWD\models"
.\target\release\rradar.exe models verify
.\target\release\rradar.exe process path\to\receipt.jpg --engine onnx --explain
```

```bash
# Linux / macOS
./tools/fetch-models.sh
RRADAR_FETCH_ORT=1 ./tools/fetch-models.sh   # Linux x64 ORT best-effort
cargo build -p rradar-cli --features onnx --release
./target/release/rradar models verify
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
| `manifest.sha256` | **Committed** SHA-256 pins |

## Cargo feature

| Build | Inference |
|-------|-----------|
| default (no feature) | mock only; onnx name returns structured “not ready” |
| `--features onnx` on `rradar-cli` / `rradar-ocr` | paddle-ocr-rs + ort **load-dynamic** |

Why `load-dynamic`? Prebuilt ORT tarballs for **windows-gnu** are not published by ort-sys; load-dynamic compiles on GNU and MSVC and loads the Microsoft ORT DLL at runtime.

## Environment

| Variable | Meaning |
|----------|---------|
| `RRADAR_MODELS_DIR` | Models root (default `./models`) |
| `ORT_DYLIB_PATH` | Full path to `onnxruntime` shared library |
| `RRADAR_MODEL_BASE_URL` | Override download base (default HuggingFace `SWHL/RapidOCR`) |
| `RRADAR_FETCH_ORT=1` | Shell script: also try to fetch ORT |
| `ORT_VERSION` | ORT release tag (**default `1.22.0`** — must match ort 2.0.0-rc.10 / `1.22.x`) |

## Desktop e2e smoke

```powershell
powershell -File scripts/smoke-onnx.ps1
# expects: models verify OK + onnx process fixtures/images/receipt_en_total89.png
```

Verified on Windows (2026-07-31): `--engine onnx` on synthetic English receipt → `engine=onnx-rapidocr`, total 89 TWD.

If `ORT_DYLIB_PATH` is unset, `rradar` looks for `models/ort/onnxruntime.{dll,so,dylib}`.

## Source & license

- Weights: [SWHL/RapidOCR](https://huggingface.co/SWHL/RapidOCR) (Apache-2.0 lineage; see `docs/licenses-checklist.md`)
- Runtime: [ONNX Runtime](https://github.com/microsoft/onnxruntime) releases
- Never commit large `.onnx` weights without a release process and license review

## Traditional Chinese notes

The PP-OCRv4 **Chinese** rec pack is simplified-primary; Traditional Chinese receipts are **best-effort** (glyph confusions possible). Product stance aligns with design Orange path: **TW e-invoice QR first**, OCR as assist + manual edit. Fill measured metrics in `docs/spike-ocr-size.md` when device spike runs.

## Doctor

```bash
cargo run -p rradar-cli -- doctor
cargo run -p rradar-cli -- models status
cargo run -p rradar-cli --features onnx -- models verify
```
