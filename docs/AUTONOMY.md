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

### Next seeds
| Priority | Item |
|----------|------|
| P1 | Confirm Actions green after this push |
| P2 | FRB when Flutter present |
| P3 | ONNX e2e image smoke (optional nightly) with pinned models |

## Rules
- No questions between cycles unless secrets / destructive remote  
- Green tests before re-plan  
- Non-goals: official sync, GPT wrapper  

## Three big loops (user request 2026-07-31)

1. **Schema v3** — tags + attachment_path columns, forward migration
2. **Rule packs** — `data_dir/rules/*.yml` + `rradar rules` + categorizer merge
3. **Handoff** — encrypted multi-device package create/info/apply (no cloud)

Tag: v0.1.0-cli.11
