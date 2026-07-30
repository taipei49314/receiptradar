# Changelog

## 0.1.0-alpha (unreleased)

### Added
- Local-first receipt process pipeline (mock OCR, L1 extract, TW e-invoice QR)
- CLI: `process`, `list`, `stats`, `export`, `backup`, `seal`
- SQLite ledger with soft/hard dedupe
- `backup.rradar` v1 + `.rrsealed` at-rest (P2)
- Merchant seed categorizer (≥150)
- Flutter mobile UI shell (A18, no FFI yet)
- `rradar-ffi` free functions for upcoming FRB
- OCR bench harness + golden fixture tests
- Network-audit stub for offline claims
