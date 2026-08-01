# Autonomous loop log

**Mode:** plan → execute → verify → re-plan → push  
**Project:** ReceiptRadar CLI-first  

## Standing schedule

| 項目 | 設定 |
|------|------|
| 節奏 | **每 3 小時**推進一次 |
| 粒度 | **大框架**（架構／產品主軸），不是小修 nits |
| 方式 | Grok 排程（durable），`fire` 後自動 pull → 做 → test/clippy → push |
| Remote | `https://github.com/taipei49314/receiptradar` |

### 大框架主軸候選
1. 真 OCR / ONNX 可跑路徑  
2. 帳本／備份／schema 邊界（無官方 cloud）  
3. Mobile / FFI 骨架  
4. 發版與 CI  
5. 可演示閉環（demo + fixtures）

## Cycle 0 — CLI product complete
- Full ledger CLI (`9856fff`)

## Cycle 1 — CLI hardening
- Batch process, manual, import, tables, smoke → **OK**

## Cycle 2 — extract noise
- Invoice lines / tiny ints → **OK**

## Cycle 3 — release hygiene + help + extract (this run)

### Plan
1. `rradar help <cmd>`  
2. `RRADAR_DEFAULT_CURRENCY`  
3. Release workflow + RELEASE.md + install script + Justfile  
4. TOTAL vs SUBTOTAL ranking fix + more total keywords  

### Verify
- `cargo test --workspace` green  
- `cargo clippy -D warnings` green  
- `rradar help process` works  

## Cycle 4 — docs/changelog closeout
- CHANGELOG + this log; commit cycle 3–4  

## Cycle 5–6 — unattended push ✅

### Plan / Execute
1. `top`, `stats --from/--to`, `clear --yes`
2. SECURITY.md, AGENTS.md
3. **GitHub:** https://github.com/taipei49314/receiptradar  
4. Pushed `master` + tag **`v0.1.0-cli.1`**

## Cycle 7 — post-publish
- Seeds, badge, repo URL

## Cycle 8 — unattended resume
- rustfmt CI fix; config.toml; costco + amount 1280 fix; count

## Cycle 10 — last/undo/recategorize
- `last`, `undo --yes`, `recategorize`, `list --year/--month`
- Removed `rust-toolchain.toml` (forced MSVC, broke local GNU)
- Push `c1c3986`

## Cycle 11 — true ONNX runnable path (main axis #1)

### Plan
1. Feature-gate real RapidOCR: `rradar-ocr` / `rradar-cli` `--features onnx`
2. Wire `paddle-ocr-rs` + `ort` **load-dynamic** (compiles on windows-gnu)
3. Fetch scripts: `tools/fetch-models.ps1` (+ `-FetchOrt`) and improved `tools/fetch-models.sh`
4. Clear errors without models / without feature; `rradar doctor` status lines
5. Docs: `models/README.md`, `docs/cli.md`, README engines table

### Verify
- `cargo test --workspace` (default mock) green  
- `cargo test -p rradar-ocr --features onnx` green  
- `cargo clippy --workspace --all-targets -- -D warnings` green  

### Result
- Desktop can run `process --engine onnx` when models + ORT present and binary built with `--features onnx`
- CI remains mock-only (no weights in git)

## Cycle 12 — product demo closed loop (main axis #5)

### Plan
1. `rradar demo` isolated closed-loop (text + mock_ocr + QR → confirm → stats → export → backup)
2. Expand fixtures matrix (7-ELEVEN, 麥當勞, 萊爾富, 屈臣氏, 中油 + mock image bins)
3. `scripts/demo.ps1` / `demo.sh`, Justfile `demo`, CI `demo closed-loop` step
4. FFI: confirm / stats / schema / ensure_ledger for mobile prep
5. Doctor shows ledger schema version

### Verify
- `cargo test --workspace` green (incl. `demo_closed_loop`, golden mock_ocr)
- `cargo clippy --workspace --all-targets -- -D warnings` green
- `rradar demo --quiet` → `DEMO_OK n=14`

### Result
- One-command recordable path from repo root; default user ledger untouched
- Fixtures sized for GIF / launch T0 narrative

## Cycle 13 — ledger schema / backup architecture (main axis #2)

### Plan
1. Forward-only schema migration framework; **v2** (`updated_at` + migration meta)
2. `SchemaTooNew` guard; `LEDGER_SCHEMA_VERSION` constant; `rradar migrate`
3. Backup UX: `info` / `verify` / `restore --merge`; `import backup`
4. Manifest carries `ledger_schema_version`; docs `ledger-schema.md` (no cloud relay)

### Verify
- `cargo test --workspace` green (incl. schema + backup merge CLI tests)
- `cargo clippy --workspace --all-targets -- -D warnings` green

### Result
- Local-first multi-device path is **backup file only**, with clear schema boundaries
- Opening any ledger auto-migrates to v2

## Cycle 14 — Mobile / FFI skeleton (main axis #3)

### Plan
1. Expand `rradar-ffi`: paths, capabilities, process bytes/QR, CRUD, stats/top, backup file
2. Crate types `staticlib` + `cdylib` for future NDK link
3. Dart `RradarApi` facade + Mock + bridge README; home shows core status
4. `docs/ffi.md` contract map (no Flutter SDK → no FRB generate)

### Verify
- `cargo test --workspace` / `cargo test -p rradar-ffi` green
- `cargo clippy --workspace --all-targets -- -D warnings` green

### Result
- Mobile closed-loop Rust API is callable from tests; UI can develop against Mock
- FRB still deferred until Flutter is on the machine

## Cycle 15 — release & CI skeleton (main axis #4)

### Plan
1. CI: `--locked` tests, ffi job step, **release binary smoke** (`cargo build --release` + demo)
2. Release: macOS aarch64 matrix, package LICENSE/README/VERSION, flatten artifacts + SHA256SUMS
3. Install: `docs/INSTALL.md`, `install-from-release.sh`/`.ps1`, unix `install-cli.sh`
4. `rradar version --long|--json` for release verification

### Verify
- `cargo test --workspace --locked` / clippy green  
- Local release smoke path documented in Justfile  

### Result
- Tag `v*` → 4 platform artifacts with checksums; CI gates include release-shaped build  

## Cycle 16 — CI green on Windows (main axis #4 fix)

### Plan
1. Diagnose `golden_mock_ocr_binaries` failure on windows-latest (8900 vs 545)
2. Root cause: mock magic required LF; Git autocrlf could rewrite fixtures
3. Fix: `.gitattributes` binary for `fixtures/mock_ocr/**`; resilient LF/CRLF magic strip in ocr + pipeline
4. Add CRLF unit/golden coverage; harden CI release smoke bash on Windows

### Verify
- `cargo test --workspace --locked` green  
- `cargo clippy --workspace --all-targets -- -D warnings` green  

### Result
- Windows CI should pass mock OCR fixtures; demo/matrix stay reliable  

## Cycle 17 — ONNX model hash pin (main axis #1)

### Plan
1. Trusted download of SWHL/RapidOCR det/rec/cls; freeze `models/manifest.sha256`
2. `rradar-ocr` manifest parse/verify (sha2); doctor pin summary
3. CLI `rradar models status|verify|pins`; fetch-models `-WritePins`
4. Docs: models/README pin table, spike A05 check, licenses pin note

### Verify
- `cargo test --workspace --locked` / clippy green  
- Local `rradar models verify` green when weights present  

### Result
- Desktop A05 artifact names + SHA-256 frozen; CI stays mock-only (weights gitignored)  
- CI windows green confirmed on prior cycle  

## Full-speed sprint (2026-07-31 user request)

### Plan
Product analytics + inbox automation + FFI analytics surface

### Delivered
- `stats --by-category --currency`
- `report` monthly markdown
- `watch <dir>` auto-ingest
- FFI `stats_by_category_json` / `report_month_markdown`

### Verify
- cargo test/clippy green; smoke report OK

### Repo
- tag v0.1.0-cli.8

## Cycle 18 — CI green + local API boundary + demo expand (axes #4 + #5 + #2)

### Plan
1. Fix CI network-audit false positive on `rradar serve` (`http://{}`)
2. Harden serve: loopback check in server, no `http://` banner, `/models` endpoint
3. Expand `rradar demo` with monthly report + model pin status + serve/inbox next steps
4. Document `docs/local-api.md`

### Verify
- `python tools/network-audit/check_offline_deps.py` exit 0
- `cargo test --workspace --locked` / clippy green
- `rradar demo --quiet` OK

### Result
- All-OS CI should pass network audit again
- Demo closed loop covers report + models; serve is explicitly local-only

## Three big loops (user request 2026-07-31)

1. **Schema v3** — tags + attachment_path columns, forward migration
2. **Rule packs** — `data_dir/rules/*.yml` + `rradar rules` + categorizer merge
3. **Handoff** — encrypted multi-device package create/info/apply (no cloud)

Tag: v0.1.0-cli.11

## Cycle 19 — ONNX desktop e2e + image fixtures (main axis #1 + #5)

### Plan
1. Pin ORT fetch default to **1.22.0** (match ort 2.0.0-rc.10)
2. `fixtures/images/`: sidecar pixel path + synthetic `receipt_en_total89.png` for real OCR
3. `scripts/smoke-onnx.ps1` / `.sh` — fetch → build onnx → verify → process
4. Demo step for image sidecar; golden `image_sidecar_fixtures`; spike/docs

### Verify
- Local: ONNX process synthetic receipt → total 89, `engine=onnx-rapidocr`
- `cargo test --workspace --locked` / clippy green (CI still mock-only)

### Result
- Desktop true OCR path is runnable and scripted; CI remains mock + sidecar

## Cycle 20 — Mobile/FFI surface for v3 + handoff/rules (main axis #3)

### Plan
1. Expand `rradar-ffi`: inbox/rules paths, tags/attachment patch, handoff create/info/merge, models pins, richer capabilities
2. Dart `RradarApi` mock + Ledger/About screens; strings update
3. `docs/ffi.md` / `docs/android-ffi.md`; optional `onnx-smoke` workflow (dispatch/weekly)

### Verify
- `cargo test -p rradar-ffi` / workspace / clippy green  
- CI default remains mock-only  

### Result
- Mobile shell exercises ledger/about against mock API matching Rust contract  
- FRB still deferred (no Flutter SDK); NDK notes ready  

### Next seeds
| Priority | Item |
|----------|------|
| P1 | Keep CI + weekly onnx-smoke green |
| P2 | FRB generate when Flutter present |
| P3 | Attachments UX + camera → process_image_bytes_json |

## Cycle 21 — attachment store + backup blobs (main axis #2)

### Plan
1. Local attachment store next to ledger (`attachments/{tx_id}/…`); relative `attachment_path`
2. CLI `attach` / `detach`, `process --attach --tags`, `edit --tags`
3. Backup + handoff pack/restore attachment blobs; manifest `attachment_count`
4. FFI attach/detach/resolve; docs ledger-schema v3 + backup format; demo step

### Verify
- `cargo test --workspace --locked` / clippy green  
- Demo includes attach + backup attachment count  

### Result
- Schema v3 fields have a real lifecycle and multi-device path (user-mediated file only)  
- Empty tags/attachment_path on update clears columns  

### Next seeds
| Priority | Item |
|----------|------|
| P1 | CI green + release tag for attachment milestone |
| P2 | Mobile camera → process_image_bytes + attach (needs Flutter/NDK) |
| P3 | Optional SQLCipher P1 when NDK available |

## Cycle 22 — product demo closed-loop via local API (main axis #5)

### Plan
1. Expand loopback `serve` to product surface (capabilities/paths/transaction/attach/process flags)
2. Ephemeral `api-smoke` + demo step 12; unit test smoke over TcpStream
3. `watch --attach`; mobile capture mock closed-loop
4. Docs `local-api.md` / AUTONOMY / CHANGELOG

### Verify
- `cargo test --workspace --locked` / clippy green (incl. `api_smoke_process_attach_and_list`)
- `rradar demo --quiet` still `DEMO_OK`

### Result
- Recordable path: fixtures → ledger → backup → **local HTTP product smoke** without curl  
- Capture screen demos mock process→ledger until FRB/camera  

### Next seeds
| Priority | Item |
|----------|------|
| P1 | README terminal GIF from `rradar demo` output |
| P2 | FRB + camera when Flutter/NDK present |
| P3 | ONNX weekly smoke keep green |

## Cycle 23 — mobile capture one-shot FFI (main axis #3)

### Plan
1. `store_attachment_bytes` for camera/in-memory frames
2. FFI `process_confirm_path_json` / `process_confirm_bytes_json` + `attach_bytes_json`
3. Dart `processConfirmPath` + Capture screen; capabilities `capture_oneshot`
4. CI `api-smoke` step; docs ffi/android-ffi

### Verify
- `cargo test -p rradar-ffi` (incl. `capture_oneshot_path_and_bytes`) / workspace / clippy green

### Result
- Mobile can call one FFI entry for process→confirm→attach without multi-step glue  
- FRB still deferred (no Flutter SDK on builder); mock shell matches contract  

### Next seeds
| Priority | Item |
|----------|------|
| P1 | FRB generate when Flutter present |
| P2 | ONNX desktop polish / measured spike fill |
| P3 | Release notes / install pin to latest cli tag |

## Cycle 24 — ONNX readiness + engine auto (main axis #1)

### Plan
1. `OnnxReadiness` probe (feature / models / pins / ORT / ready_for_inference)
2. `engine auto` + `engines_catalog_json`; CLI `rradar engines`
3. version --long/--json + doctor surface readiness
4. Measure mock bench → spike-ocr-size.md; docs INSTALL/models/README

### Verify
- `cargo test --workspace --locked` / clippy green (incl. auto→mock tests)
- `rradar engines` works without weights

### Result
- True OCR path is **discoverable** and auto-selects when ready; CI stays mock-only  
- Desktop mock p50≈5ms / p95≈6ms recorded (11 text fixtures)

### Next seeds
| Priority | Item |
|----------|------|
| P1 | Device A04 matrix fill when Android available |
| P2 | FRB + camera when Flutter present |
| P3 | Release package docs polish |

## Rules
- No questions between cycles unless secrets / destructive remote  
- Green tests before re-plan  
- Non-goals: official sync, GPT wrapper  




