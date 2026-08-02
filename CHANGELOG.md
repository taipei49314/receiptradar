# Changelog

## 0.1.0-alpha (unreleased)

### Fixed
- Network audit allowlist for loopback/`rradar serve` (restore CI green after local HTTP API)
- Windows CI: mock OCR fixtures tolerate CRLF magic terminators; `.gitattributes` marks `fixtures/mock_ocr/**` binary

### Changed
- `rradar serve` loopback enforcement in server; GET `/models`; demo steps for report + model pins
- CSV export includes `tags` and `attachment_path` columns
- Backup/handoff packages optionally pack `attachments/**` blobs; restore rehydrates them
- Backup/handoff also pack optional `budgets.toml` (soft limits); restore rehydrates
- Local HTTP API expanded for product demo automation (capabilities, paths, attach, process flags)

### Added
- **Release/CI schema v4 productization (axis #4):** release `VERSION`/`version.json` from binary; pack ledger-schema docs; release-check soft-delete+integrity; CI trash smoke; verify-install requires schema ≥ 4
- **Ledger schema v4 soft-delete (axis #2):** `deleted_at` trash/restore/purge; stats/list/export skip trash; `rradar trash|restore|purge`; doctor integrity; FFI trash surface
- **Recordable synthetic photo matrix (axis #5):** `tools/gen-receipt-png` regenerates real PNG receipts (familymart/7-11/starbucks/mcdonalds); demo step CSV import + backup merge multi-device path; 4 sidecar fixtures ONNX-capable
- **Mobile FFI transfer surface (axis #3):** `ocr_lines_*_json`, `import_json_json`, `backup_info|verify|merge_json`; capture options `max_edge`/`force_ocr`/`low_confidence_retry`; Dart Transfer screen + capability flags (`ocr_raw`, `csv_import`, `backup_merge`, …)
- **Real image preprocess + ONNX product path (axis #1):** decode JPEG/PNG/WebP/GIF; max-edge 1280 downscale + low-conf retry at 1600; `rradar ocr` raw line dump; `rradar bench` A04 harness (`force_ocr`, p50/p95); desktop ONNX warm p50 ~115 ms recorded in spike-ocr-size.md
- **CSV import (axis #2):** `transactions_from_csv` + `rradar import csv` (export header roundtrip, empty id → ULID, skip existing); FFI `import_csv_json`
- **Supply-chain dual gate (axis #4):** root `deny.toml` + `cargo deny check` (pin 0.20.2); `scripts/check-supply-chain.ps1`/`.sh`; CI installs cargo-deny and runs check; CI `fixtures verify`
- **Recordable fixture matrix (axis #5):** +6 TW text fixtures (家樂福/OK/高鐵/康是美/鼎泰豐/ibon), +3 mock_ocr, +1 image sidecar; `rradar fixtures list|verify`; `scripts/record-demo.ps1`; demo processes all sidecar images
- **Year analytics + merchant aliases (axis #2):** `stats --year`, `report --year` annual markdown; `rradar aliases`; backup packs `merchant_aliases.toml`; FFI `report_year_markdown` / `aliases_json`
- **Mobile FFI parity (axis #3):** Dart query/tags/budget/engines API; Budget screen + ledger tag filter; `docs/frb-contract.md` FRB generate checklist
- **Supply-chain / release trust (axis #4):** `tools/supply-chain/check_deps.py` (forbidden license gate + inventory); `rradar licenses`; CI supply-chain step; `docs/SUPPLY-CHAIN.md`; release-check verifies LICENSE + THIRD_PARTY_NOTICES
- **Demo showcase + product search closed-loop (axis #5):** `docs/demo-showcase.md`; local API `/tags` `/budget` + rich `/transactions` filters; filtered `export`; doctor budgets line; api-smoke hits tag filter
- **Tag filter + local budgets (axis #2):** `TxFilter` / `list --tag|--category|--from|--to|--min-amount|--max-amount|--has-attachment`; `rradar tags`; `rradar budget set|status|list|clear` (`budgets.toml`); report Budgets section; FFI `query_transactions_json` / `budget_status_json`
- **Release/CI productization:** `rradar release-check` (alias `self-check`); `scripts/verify-install.ps1`/`.sh`; release archives ship CHANGELOG/cli/privacy/THIRD_PARTY_NOTICES; CI release-check + engines; richer VERSION metadata
- **ONNX readiness productization:** `probe_onnx_readiness`, `rradar engines [--json]`, `process --engine auto`, version/doctor readiness lines; FFI `engines_json`; mock bench p50/p95 recorded in spike-ocr-size.md
- **Mobile capture one-shot (FFI):** `process_confirm_path_json` / `process_confirm_bytes_json`; `store_attachment_bytes` / `attach_bytes_json`; Dart `processConfirmPath` + Capture screen; capabilities `capture_oneshot`
- **Local API product smoke:** `rradar api-smoke`; `serve` GET `/capabilities` `/paths` `/transaction`; POST `/process` `{attach,tags}` + POST `/attach`; demo step 12; `watch --attach`; CI `api-smoke` step
- Mobile **capture mock closed-loop** (process → ledger via MockRradarApi)
- **Attachment store (schema v3 lifecycle):** `{db_parent}/attachments/{tx_id}/…` with relative DB paths; `rradar attach` / `detach`; `process --confirm --attach --tags`; backup `attachment_count`; FFI `attach_file_json` / `detach_file_json`
- **Mobile FFI v3 surface:** handoff create/info/merge, rules/inbox paths, tags/attachment patch, models pins JSON; Dart Ledger/About screens; `docs/android-ffi.md`; optional weekly `onnx-smoke` workflow
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
