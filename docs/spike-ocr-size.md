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

Require ≥2 packs on zh-TW-labeled subset before A05 pin.

| Pack artifact | SHA-256 | Size | zh-TW total exact | Notes |
|---------------|---------|------|-------------------|-------|
| | | | | |

## Decision log

- [ ] Color gate recorded
- [ ] A05 model name + hash frozen in `models/README.md`
- [ ] Marketing latency remains “seconds, on-device” until measured
