# Changelog

## 0.1.0-alpha (unreleased)

### Fixed
- Network audit allowlist for loopback/`rradar serve` (restore CI green after local HTTP API)
- Windows CI: mock OCR fixtures tolerate CRLF magic terminators; `.gitattributes` marks `fixtures/mock_ocr/**` binary

### Changed
- `rradar serve` loopback enforcement in server; GET `/models`; demo steps for report + model pins

### Added
- **ONNX desktop e2e:** ORT default 1.22.0; `scripts/smoke-onnx.ps1`/`.sh`; `fixtures/images/` (sidecar + synthetic receipt); demo pixel-sidecar step
- **ONNX A05 hash pins:** committed `models/manifest.sha256` (det/rec/cls); `rradar models status|verify`; fetch-models `-WritePins`
- **Release/CI skeleton:** locked CI + release binary smoke; GitHub Release packages (incl. macOS aarch64) with LICENSE/VERSION/checksums; `docs/INSTALL.md`; `install-from-release.sh`/`.ps1`; `rradar version --long|--json`
- **Mobile FFI surface** (`rradar-ffi`): process path/bytes, ledger CRUD, stats/top, backup file, capabilities; `staticlib`/`cdylib`; Dart `RradarApi` mock + `docs/ffi.md`
- **Ledger schema v2** + forward migrations (`LEDGER_SCHEMA_VERSION`); `updated_at`; `rradar migrate`; `docs/ledger-schema.md`
- **Backup UX:** `backup info|verify`, `restore --merge`, `import backup`; manifest `ledger_schema_version`
- **`rradar demo`**: isolated closed-loop (fixtures → ledger → export → backup); `scripts/demo.ps1` / `demo.sh`; expanded fixture matrix + mock OCR bins; CI demo step
- FFI: `confirm_draft_json`, `stats_all_json`, `ledger_schema_version`, `ensure_ledger`
- Doctor: ledger schema version; demo hint
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
