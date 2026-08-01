# Demo showcase (recordable product path)

One-command closed loop for GIF / launch video / CI. **No cloud. No account.**

## 60-second script

```bash
# From repo root
rradar demo
# Quiet (CI):
rradar demo --quiet
```

### What viewers should see

| Beat | Demo step | Talk track |
|------|-----------|------------|
| 1 | text fixtures | Everyday Taiwan receipts → structured draft |
| 2 | mock_ocr bins | Pixel path without shipping weights |
| 3–4 | attach + tags | Schema v3 local blobs next to the ledger |
| 5 | TW e-invoice QR | Prefer left-QR totals when present |
| 6 | list / tags filter | Search by `--tag demo` |
| 6+ | soft budget | Local monthly limit; **never** mix currencies |
| 7–8 | stats + export | Per-currency totals only |
| 9 | backup.rradar | Multi-device = **file you carry** |
| 10 | monthly report | Markdown + Budgets section |
| 11 | model pins | True OCR optional, hash-pinned |
| 12 | local API smoke | Loopback HTTP product surface |

Exit line: `DEMO_OK n=…`

## Follow-ups after demo

```bash
rradar list --tag demo
rradar budget set --currency TWD --monthly 30000
rradar budget status
rradar export csv --tag demo -o demo-tagged.csv
rradar serve --bind 127.0.0.1:7432   # then GET /tags /budget /transactions?tag=demo
```

## Terminal GIF recipe

1. Clean shell, large font, dark theme.
2. `cd` to repo root; run `rradar demo` (not quiet).
3. Crop to the step banners (`── step N: … ──`) and final `DEMO_OK`.
4. Optional second clip: `rradar budget status` + `rradar list --tag demo`.

## Policy line (always)

> Core path works offline. Multi-device is encrypted backup/handoff — **no** official cloud relay.
