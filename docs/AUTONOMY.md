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

## Cycle 5–6 — unattended push

### Plan
1. `top` merchants, `stats --from/--to`, `clear --yes`
2. SECURITY.md, AGENTS.md
3. Create GitHub remote + push master

### Next seeds
| Priority | Item | Blocker |
|----------|------|---------|
| P1 | Tag release after push | human optional |
| P2 | Real ONNX + models | weights |
| P2 | Flutter FRB | SDK |

## Rules
- No questions between cycles unless secrets / destructive remote  
- Green tests before re-plan  
- Non-goals: official sync, GPT wrapper  
