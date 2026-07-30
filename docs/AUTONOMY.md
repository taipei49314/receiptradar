# Autonomous loop log

**Mode:** plan → execute → verify → re-plan  
**Project:** ReceiptRadar CLI-first

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

- More merchant seeds, fixture pxmart, CI badge, correct repo URL

### Next seeds
| Priority | Item | Blocker |
|----------|------|---------|
| P2 | Watch GH Actions release | CI |
| P2 | Real ONNX + models | weights |
| P2 | Flutter FRB | SDK |

## Rules
- No questions between cycles unless secrets / destructive remote  
- Green tests before re-plan  
- Non-goals: official sync, GPT wrapper  
