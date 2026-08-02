# Fixtures

## Policy (PII)

1. Prefer team-captured receipts with written consent.
2. Redact membership / phone / personal QR when not required for the class.
3. Day-to-day CI uses **synthetic text**, **mock OCR binaries**, and **synthetic QR payloads** (no personal data).
4. Release sign-off (≥30 real photos) is a later gate under this policy — not required for A12.
5. Do not attach live personal receipts to public issues without scrubbing.

## Layout

| Path | Purpose |
|------|---------|
| `text/*.txt` | Metric (a): extract/category given perfect OCR text |
| `mock_ocr/*.bin` | Mock “image” path: `RRADAR_MOCK_OCR` + LF/CRLF + UTF-8 lines (binary in `.gitattributes`) |
| `images/*.png` + `*.png.ocr.txt` | Synthetic receipt PNGs (`tools/gen-receipt-png`) + sidecar for CI |
| `images/receipt_en_total89.png` | Same family; optional ONNX smoke without sidecar |
| `qr/*.payload.txt` | Appendix A left-QR structural decode samples |
| `manifest.json` | Index for golden runners + **demo** flags |

```bash
# Regen synthetic PNGs (ASCII bitmap receipts; no PII)
cargo run -p gen-receipt-png -- fixtures/images

# Sidecar image path (default mock engine — uses .ocr.txt)
rradar process fixtures/images/familymart_photo.png --explain

# Real ONNX on pixel path (local only — ignore sidecar)
rradar process fixtures/images/familymart_photo.png --engine onnx --explain
# or full smoke:
powershell -File scripts/smoke-onnx.ps1
rradar bench fixtures/images/receipt_en_total89.png --engine onnx --json
```

## Matrix index

```bash
rradar fixtures list              # table from manifest.json
rradar fixtures verify            # process each entry; check totals (mock)
rradar fixtures list --json
# Real ONNX on synthetic PNG matrix (local; needs --features onnx + models):
# cargo run -p rradar-cli --features onnx -- fixtures verify --engine onnx --onnx-smoke
```

## One-click demo

```bash
# from repo root (isolated demo ledger under target/demo)
cargo run -p rradar-cli -- demo
# or
powershell -File scripts/demo.ps1
# or
./scripts/demo.sh
# Guided terminal GIF narrative:
powershell -File scripts/record-demo.ps1
```

## CLI samples

```bash
rradar process fixtures/text/familymart_89.txt --explain
rradar process fixtures/mock_ocr/familymart_mock.bin --explain
rradar process fixtures/text/familymart_89.txt --json
# QR prefer (PowerShell):
$q = Get-Content -Raw fixtures/qr/tw_einvoice_sample_01.payload.txt
rradar process fixtures/text/familymart_89.txt --qr $q --explain
```
