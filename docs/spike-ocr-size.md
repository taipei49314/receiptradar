# OCR + size spike report (PR-A04)

**Status:** harness productized (`rradar bench` + `tools/bench-ocr`); **device measurements pending**.  
**Desktop A05 pins:** frozen. **CLI readiness:** `rradar engines` / `process --engine auto` / real max-edge preprocess.

## Binding color gate (record outcome here)

| Color | Condition | Action |
|-------|-----------|--------|
| **Green** | e2e accuracy + size within budget on 2 ref devices | Proceed A05 pin models |
| **Yellow** | Accuracy OK, size over | OQ-3 within 1 week (quantize / download-only offline) |
| **Orange** | OCR weak, QR strong | Narrow matrix; manual-entry prominence; optional ML Kit Track B |
| **Red** | Neither path viable | Public launch = T0 CLI only; slip T1 mobile |

**Outcome (desktop):** mock + **ONNX warm inference** measured (below). Preprocess max-edge 1280 + low-conf retry 1600 shipped. **Device A04 still pending.**

## Mock baseline (desktop, this repo)

```text
rradar bench fixtures/text --engine mock --json
# or: cargo run -p bench-ocr -- fixtures/text --json
```

| Field | Value |
|-------|-------|
| Date | 2026-08-02 |
| CPU | Windows x86_64 (dev) |
| rustc | 1.97.1 (stable-gnu) |
| Engine | mock |
| Fixtures | text receipts under `fixtures/text` |
| success / fail | all / 0 |
| **p50_ms** | **~0–5** (sub-ms floor on Instant::as_millis) |
| **p95_ms** | **~0–6** |
| Notes | Pipeline L1 extract only — not a substitute for on-device CJK ONNX |

## Desktop ONNX (warm, synthetic receipt)

```powershell
cargo build -p rradar-cli --features onnx --release
$env:RRADAR_MODELS_DIR = "$PWD\models"
.\target\release\rradar.exe models verify
.\target\release\rradar.exe process fixtures/images/receipt_en_total89.png --engine onnx --explain
.\target\release\rradar.exe ocr fixtures/images/receipt_en_total89.png --engine onnx
.\target\release\rradar.exe bench fixtures/images/receipt_en_total89.png --engine onnx --json
powershell -File scripts/smoke-onnx.ps1
```

| Field | Value |
|-------|-------|
| Date | **2026-08-02** (re-measure after preprocess productization) |
| Engine | onnx-rapidocr (`--features onnx`) |
| ORT | 1.22.0 load-dynamic (`models/ort/onnxruntime.dll`) |
| Input | `fixtures/images/receipt_en_total89.png` (480×640 synthetic) |
| Preprocess | max_edge=1280, decoded=true, resized=false |
| Result | **total exact 89 TWD**; merchant `FAMILYMART LINJIANG`; invoice AB12345678 |
| Line conf | ~0.98–1.00 (raw `rradar ocr`) |
| **Warm p50_ms** | **~115** (shared engine after warmup) |
| Cold first infer | ~500–600 ms (model load + first detect) |
| Pins | 3/3 OK (`models/manifest.sha256`) |
| Readiness | `rradar engines` → onnx ready; auto → onnx |

**Placeholder photo fixtures** (`familymart_photo.png`, `seven_eleven_photo.png`) are tiny PNG stubs with `.ocr.txt` sidecars for demo; pixel OCR correctly returns no lines. Bench uses `force_ocr` so sidecars do not zero timings.

## Preprocess product path (Cycle 34)

| Item | Behavior |
|------|----------|
| Decode | JPEG/PNG/WebP/GIF via `image` crate |
| Pass 1 | longest edge ≤ **1280** (never upscale) |
| Pass 2 | retry **1600** if overall conf &lt; 0.45 after L1 |
| Non-image | text / mock magic passthrough |
| CLI | `rradar ocr` (raw lines), `rradar bench` (p50/p95, force_ocr) |

## Device matrix (to fill)

| Device | SoC | Android | Model pack A | Model pack B | total exact% | merchant% | p50 ms | p95 ms | APK/model MB |
|--------|-----|---------|--------------|--------------|--------------|-----------|--------|--------|--------------|
| | | | | | | | | | |

## zh-TW model pack comparison

Require ≥2 packs on zh-TW-labeled subset before **device** A04 Green; desktop pin frozen for CLI path.

| Pack artifact | SHA-256 (full in `models/manifest.sha256`) | Size | zh-TW total exact | Notes |
|---------------|--------------------------------------------|------|-------------------|-------|
| ch_PP-OCRv4_det_infer.onnx | d2a7720d45a5…f49da9 | ~4.5 MiB | TBD (device) | HF SWHL/RapidOCR PP-OCRv4 |
| ch_PP-OCRv4_rec_infer.onnx | 48fc40f24f6d…3683b | ~10 MiB | TBD (device) | Simplified-primary rec |
| ch_ppocr_mobile_v2.0_cls_infer.onnx | e47acedf6632…6215c | ~0.7 MiB | n/a | Angle cls (PP-OCRv1) |

**Desktop A05 pin:** frozen 2026-07-31 in `models/manifest.sha256` + `rradar models verify`.  
**Device A04 color gate:** still pending (desktop mock + ONNX synthetic green).

## Decision log

- [ ] Color gate recorded (device)
- [x] A05 desktop model name + hash frozen in `models/README.md` / `manifest.sha256`
- [x] Marketing latency remains “seconds, on-device” until device measured
- [x] Desktop mock p50/p95 recorded (2026-08-01 / recheck 2026-08-02)
- [x] Desktop ONNX warm p50 ~115 ms on synthetic receipt (2026-08-02)
- [x] `process --engine auto` + readiness catalog shipped
- [x] Real max-edge preprocess + low-conf retry + `rradar bench` / `ocr` (Cycle 34)
