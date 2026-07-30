# Autonomous loop log

**Mode:** plan → execute → verify → re-plan  
**Project:** ReceiptRadar CLI-first

## Cycle 0 (prior) — CLI product complete

- Delivered full CLI ledger UX (`9856fff`)
- Verified: unit/golden/e2e tests green

## Cycle 1 — CLI hardening (this run)

### Plan
1. Batch `process` multiple files  
2. `manual` entry without OCR  
3. `import json`  
4. Readable table output (`|` separators)  
5. Smoke script `scripts/smoke-cli.ps1`  

### Execute
- See commit after this cycle  

### Verify
- `cargo clippy -D warnings`  
- `cargo test --workspace`  
- `scripts/smoke-cli.ps1`  

### Verify result
- clippy `-D warnings` OK  
- workspace tests OK  
- `scripts/smoke-cli.ps1` → **SMOKE_CLI_OK**

## Cycle 2 — extract noise + docs

### Plan
1. Reduce false amount candidates (invoice id lines, tiny bare ints)  
2. Polish list footer formatting  
3. Update cli.md completeness  

### Execute / Verify
- unit extract tests + full workspace  

### Next plan seeds (Cycle 3+)
| Priority | Item | Notes |
|----------|------|-------|
| P1 | Optional: release tarball / `cargo install` CI | |
| P1 | Real ONNX when models available | Not blocking CLI |
| P2 | Flutter only after Flutter SDK | |

## Rules
- Do not ask user between cycles unless blocked on secrets/destructive remote ops  
- Prefer green tests before re-plan  
- Keep non-goals (no official sync, no GPT wrapper)  
