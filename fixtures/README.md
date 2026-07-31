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
| `qr/*.payload.txt` | Appendix A left-QR structural decode samples |
| `manifest.json` | Index for golden runners + **demo** flags |

## One-click demo

```bash
# from repo root (isolated demo ledger under target/demo)
cargo run -p rradar-cli -- demo
# or
powershell -File scripts/demo.ps1
# or
./scripts/demo.sh
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
