# Changelog

## 0.1.0-alpha (unreleased)

### Added
- **ONNX RapidOCR path** (`--features onnx`): paddle-ocr-rs + ORT load-dynamic; `tools/fetch-models.ps1` / `.sh`; doctor status; clear no-model errors (`models/README.md`)
- Local-first receipt process pipeline (mock OCR, L1 extract, TW e-invoice QR)
- **CLI product complete:** `init`, `doctor`, `process` (batch), `manual`, `import`, `list`/`show`/`edit`/`delete`, `stats`, `export`, `backup`, `seal`/`unseal`
- `rradar help <command>` topic help; `RRADAR_DEFAULT_CURRENCY`
- SQLite ledger with soft/hard dedupe
- `backup.rradar` v1 + `.rrsealed` at-rest (P2)
- Merchant seed categorizer (≥150)
- Flutter mobile UI shell (A18, no FFI yet)
- `rradar-ffi` free functions for upcoming FRB
- OCR bench harness + golden fixture tests
- Network-audit stub; GitHub Actions **release** workflow on `v*` tags
- `scripts/smoke-cli.ps1`, `scripts/install-cli.ps1`, `Justfile`, `docs/RELEASE.md`, `docs/AUTONOMY.md`

### Fixed
- Amount ranking: do not treat SUBTOTAL as TOTAL; quieter invoice-line noise
