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

## Left for A13+

1. **A13–A14** SQLite ledger + soft/hard dedupe  
2. **A15–A16** `backup.rradar` v1 + at-rest encryption  
3. **A04** OCR size/latency spike on 2 Android devices → pin models  
4. **A05** ONNX RapidOCR feature + hash-pinned fetch  
5. **A18–A22** Flutter camera loop + offline flavor  
6. **A23–A25** egress CI + README demo GIF + v0.1.0 release  

## Known limitations (A12)

- Amount regex still noisy on unstructured lines; ranking prefers 合計/總計 so fixtures pass.
- QR path sets merchant to `seller:{BAN}` until OCR/name dictionary fills display name.
- ONNX engine is a deliberate stub until spike.
