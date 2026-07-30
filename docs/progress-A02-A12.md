# Progress: Track A PR-A02 → PR-A12

**Date:** 2026-07-30  
**Repo:** `C:\Users\1\receiptradar`  
**Status:** Goal milestone complete for CLI-side thin slice through A12 (mock OCR path).

## Done

| PR | Item | Evidence |
|----|------|----------|
| A01 | Scaffold | Already present; extended |
| A02 | `Money` + `Iso4217` exponent, `ReceiptDraft`, `ExplainTrace` | `crates/rradar-core/src/{money,types,explain}.rs` + unit tests |
| A03 | `OcrEngine` trait + mock (+ onnx stub) | `crates/rradar-ocr` |
| A04/A05 | Spike / real ONNX | **Deferred** — `OnnxOcrEngine` returns `OnnxUnavailable`; mock is CI default |
| A06 | Preprocess + `process_*` orchestration | `preprocess.rs`, `pipeline.rs` |
| A07 | TW e-invoice left QR parse (Appendix A shape) | `qr.rs` + 3 payload fixtures |
| A08 | L1 extract (amount rank, date, merchant, invoice) | `extract.rs` |
| A09–A10 | Taxonomy packs + ≥150 merchant seed + categorizer | `category.rs`, `data/categories.*.yaml` |
| A11 | Text/QR fixtures + PII policy note | `fixtures/` |
| A12 | `rradar process` + `--explain` + `--json` + `--qr` | `crates/rradar-cli` |

## Verify

```powershell
$env:Path = "C:\Users\1\.local\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:Path
cd C:\Users\1\receiptradar
cargo test --workspace
cargo run -p rradar-cli -- process fixtures/text/familymart_89.txt --explain
```

Toolchain note: this machine used **stable-gnu + WinLibs MinGW** because MSVC `link.exe` was absent. Prefer VS Build Tools + `stable-msvc` if you already have them.

## Explicitly not done (as planned)

- Flutter mobile / FFI (A18–A22)
- SQLCipher / backup v1 (A15–A16)
- Real ONNX models + device spike report (A04/A05)
- Sync / WASM / desktop / official relay (never)

## A13–A17 (done 2026-07-30)

| PR | Item | Notes |
|----|------|-------|
| A13 | SQLite ledger | `confirm_draft`, `list`, `stats_by_currency_month` |
| A14 | Dedupe | hard = invoice+amount+day; soft = content hash |
| A15 | Export + backup | CSV/JSON; `backup.rradar` v1 AEAD archive |
| A16 | At-rest P2 | `.rrsealed` whole-file Argon2id+XChaCha20-Poly1305 |
| A17 | License checklist | `docs/licenses-checklist.md` (manual before release) |

CLI: `list`, `stats`, `export`, `backup create|restore`, `seal`, `process --confirm --db`.

## A05 / A18 / A19 / CI (done 2026-07-30, autonomous)

| Item | Status |
|------|--------|
| A05 ONNX module | Path validation + engine select; inference runtime post-Green spike |
| A18 Flutter shell | `apps/mobile` onboarding/home/capture placeholder |
| A19 FFI stub | `rradar-ffi` JSON helpers (FRB codegen later) |
| Golden fixtures | `crates/rradar-core/tests/golden_fixtures.rs` |
| CI | fmt/clippy/test + cli smoke + network-audit |

## Left

1. **A04 device** measurements (fill `docs/spike-ocr-size.md`)  
2. **A05** full ORT inference when models pinned  
3. **A19–A22** FRB + camera + flavors (needs Flutter/Android SDK)  
4. **A24–A25** demo GIF + v0.1.0 release  
5. SQLCipher P1 when Android NDK available

## Known limitations (A12)

- Amount regex still noisy on unstructured lines; ranking prefers 合計/總計 so fixtures pass.
- QR path sets merchant to `seller:{BAN}` until OCR/name dictionary fills display name.
- ONNX engine is a deliberate stub until spike.
