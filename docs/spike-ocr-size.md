# OCR + size spike report (PR-A04)

**Status:** harness ready (`tools/bench-ocr`); **device measurements pending**.

## Binding color gate (record outcome here)

| Color | Condition | Action |
|-------|-----------|--------|
| **Green** | e2e accuracy + size within budget on 2 ref devices | Proceed A05 pin models |
| **Yellow** | Accuracy OK, size over | OQ-3 within 1 week (quantize / download-only offline) |
| **Orange** | OCR weak, QR strong | Narrow matrix; manual-entry prominence; optional ML Kit Track B |
| **Red** | Neither path viable | Public launch = T0 CLI only; slip T1 mobile |

**Outcome (fill):** _not run yet — mock-only baseline below._

## Mock baseline (desktop, this repo)

```text
cargo run -p bench-ocr -- fixtures/text --json
```

Record date / machine / rustc:

| Field | Value |
|-------|-------|
| Date | |
| CPU | |
| rustc | |
| Engine | mock |
| p50 / p95 | see command output |
| Notes | Not a substitute for on-device CJK ONNX |

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
