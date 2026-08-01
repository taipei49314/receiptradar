# OCR + size spike report (PR-A04)

**Status:** harness ready (`tools/bench-ocr`); **device measurements pending**.  
**Desktop A05 pins:** frozen. **CLI readiness:** `rradar engines` / `process --engine auto`.

## Binding color gate (record outcome here)

| Color | Condition | Action |
|-------|-----------|--------|
| **Green** | e2e accuracy + size within budget on 2 ref devices | Proceed A05 pin models |
| **Yellow** | Accuracy OK, size over | OQ-3 within 1 week (quantize / download-only offline) |
| **Orange** | OCR weak, QR strong | Narrow matrix; manual-entry prominence; optional ML Kit Track B |
| **Red** | Neither path viable | Public launch = T0 CLI only; slip T1 mobile |

**Outcome (desktop):** mock baseline measured (below). **ONNX desktop smoke** path scripted (`scripts/smoke-onnx.ps1`) when weights present. **Device A04 still pending.**

## Mock baseline (desktop, this repo)

```text
cargo run -p bench-ocr -- fixtures/text --json
```

| Field | Value |
|-------|-------|
| Date | 2026-08-01 |
| CPU | Windows x86_64 (dev) |
| rustc | 1.97.1 (stable-gnu) |
| Engine | mock |
| Fixtures | 11 text receipts |
| success / fail | 11 / 0 |
| **p50_ms** | **5** |
| **p95_ms** | **6** |
| Notes | Pipeline L1 extract only — not a substitute for on-device CJK ONNX |

Re-run and paste JSON into release notes if numbers drift.

## Desktop ONNX smoke (optional, local)

```powershell
powershell -File scripts/smoke-onnx.ps1
# or check readiness without weights:
rradar engines --json
rradar process fixtures/images/receipt_en_total89.png --engine auto --explain
```

| Field | Value |
|-------|-------|
| Date | 2026-07-31 (first green local run) |
| Engine | onnx-rapidocr (`--features onnx`) |
| ORT | 1.22.0 load-dynamic |
| Input | `fixtures/images/receipt_en_total89.png` (synthetic) |
| Result | total exact 89 TWD; merchant FAMILYMART* |
| Notes | Pin pack in `models/manifest.sha256`; not in default CI |
| Readiness API | `probe_onnx_readiness` / `rradar engines` (feature + models + pins + ORT) |

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
**Device A04 color gate:** still pending (mock baseline only).

## Decision log

- [ ] Color gate recorded (device)
- [x] A05 desktop model name + hash frozen in `models/README.md` / `manifest.sha256`
- [x] Marketing latency remains “seconds, on-device” until measured
- [x] Desktop mock p50/p95 recorded (2026-08-01)
- [x] `process --engine auto` + readiness catalog shipped
