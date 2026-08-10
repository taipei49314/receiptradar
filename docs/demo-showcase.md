# Demo showcase (recordable product path)

One-command closed loops for GIF / launch video / CI. **No cloud. No account.**

## 30-second daily path (preferred clip)

```bash
# From repo root
rradar day
rradar day --quiet          # CI-friendly
# Windows helper:
#   powershell -File scripts/day.ps1
```

Shows: 全家 / 麥味登 / 50嵐 / 北捷 / 中華電信 → stamped **UTC today** → `today` glance with short merchant names + soft budget.

Exit line: `DAY_OK n=5`

## Inbox scoop (drop folder → today)

```bash
rradar inbox --ensure
# copy receipts into the printed inbox path, then:
rradar scoop              # confirms + archives to inbox/done/YYYY-MM-DD/
rradar scoop --quiet
rradar scoop --no-archive # leave files in place
# Windows helper (copies sample fixtures into target/scoop/inbox):
#   powershell -File scripts/scoop.ps1
```

Exit line: `SCOOP_OK n=… archived=…`

## Month-end close

```bash
rradar month                              # current UTC month glance
rradar close -o month.md --csv month.csv  # markdown + Excel CSV
rradar month --year 2026 --month 8 --json
```

Exit line: `MONTH_OK | YYYY-MM`

## 60-second full product demo

```bash
# From repo root
rradar fixtures list      # matrix size / classes
rradar fixtures verify    # offline extract totals
rradar demo
# Quiet (CI):
rradar demo --quiet
# Guided recording (Windows):
#   powershell -File scripts/record-demo.ps1
```

### Fixture matrix (Cycle 36+)

| Class | Count (approx) | Examples |
|-------|----------------|----------|
| text | 17 | 全家, 7-11, OK, 家樂福, 高鐵, 鼎泰豐, ibon, … |
| mock_ocr | 5 | familymart, 7-11, mcdonalds, carrefour, starbucks USD |
| image+sidecar | **4** | familymart / 7-11 / starbucks / mcdonalds **synthetic PNGs** (`tools/gen-receipt-png`) |
| onnx_smoke | 5 | same bitmaps; optional `--features onnx` |
| qr | 3 | TW e-invoice left-QR samples |

Index: `fixtures/manifest.json`. Regen photos: `cargo run -p gen-receipt-png -- fixtures/images`.

### What viewers should see

| Beat | Demo step | Talk track |
|------|-----------|------------|
| 0 | fixtures list/verify | Offline matrix is the product proof |
| 1 | text fixtures | Everyday Taiwan receipts → structured draft |
| 2 | mock_ocr bins | Pixel path without shipping weights |
| 3–4 | synthetic photos + attach/tags | Real PNG pixels + sidecar for CI; ONNX-ready |
| 5 | TW e-invoice QR | Prefer left-QR totals when present |
| 6 | list / tags filter | Search by `--tag demo` |
| 6+ | soft budget | Local monthly limit; **never** mix currencies |
| 7–8 | stats + export | Per-currency totals only |
| 9 | CSV import + backup merge | Multi-device = **file you carry** (no cloud) |
| 10–11 | monthly + annual report | Markdown + Budgets + year close |
| 12 | model pins / aliases | True OCR optional; local renames |
| 13 | local API smoke | Loopback HTTP product surface |

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

### Daily path (recommended first clip)

1. Clean shell, large font.
2. `powershell -File scripts/day.ps1` (or `rradar day`).
3. Crop to the `── add --as-today` beats and final `today` table + `DAY_OK`.

### Full product demo

1. Clean shell, large font, dark theme.
2. `cd` to repo root; run `rradar demo` (not quiet).
3. Crop to the step banners (`── step N: … ──`) and final `DEMO_OK`.
4. Optional second clip: `rradar budget status` + `rradar list --tag demo`.

## Policy line (always)

> Core path works offline. Multi-device is encrypted backup/handoff — **no** official cloud relay.
