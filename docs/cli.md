# ReceiptRadar CLI product guide

Local-first household ledger from receipt text/images. **No account. No cloud required.**

## Install

```bash
cd receiptradar
cargo install --path crates/rradar-cli
# or
cargo run -p rradar-cli -- help
```

Windows (this machine): add MinGW + cargo to `PATH` first.

## Data location

| | Path |
|--|------|
| Home | `%APPDATA%\receiptradar` (Windows) / `~/.local/share/receiptradar` |
| Ledger | `…/ledger.db` |
| Override | `RRADAR_HOME`, `RRADAR_DB` |

```bash
rradar init
rradar path
rradar doctor
```

## One-click demo (recordable)

```bash
# From repo root — isolated demo ledger, full closed loop
rradar demo
rradar demo --quiet          # CI-friendly
powershell -File scripts/demo.ps1
```

Covers: text + mock_ocr + TW QR → confirm → list/stats/top → export → backup → **monthly report** → model pin status.  
Next steps printed: `inbox`/`watch`, `serve` (loopback HTTP — [local-api.md](./local-api.md)).

## Daily workflow

```bash
# 1) Parse a receipt (text fixture or image with mock/onnx)
rradar process receipt.txt --explain

# 2) Confirm into default ledger (override fields if needed)
rradar process receipt.txt --confirm \
  --merchant "全家臨江店" --amount 89 --category grocery_convenience

# 3) Browse
rradar list
rradar list --year 2024 --month 5
rradar list --query 全家 --currency TWD
rradar last
rradar show <id>
rradar count
rradar stats              # this calendar month, per currency
rradar stats --all
rradar stats --from 2024-01-01 --to 2024-12-31
rradar top --currency TWD --limit 10

# 4) Fix mistakes
rradar edit <id> --amount 99 --notes "招待客戶"
rradar undo --yes         # remove last confirmed
rradar delete <id> --yes
rradar recategorize       # only category=other
rradar config set default_currency TWD

# 5) Export / backup (local-only multi-device via file copy)
rradar export csv -o month.csv
rradar backup create -p 'your-passphrase' -o backup.rradar
rradar backup info --in backup.rradar -p 'your-passphrase'
rradar backup verify --in backup.rradar -p 'your-passphrase'
rradar backup restore --in backup.rradar -p 'your-passphrase' --merge
rradar import backup --in backup.rradar -p 'your-passphrase'
rradar migrate                              # schema version / migrations
rradar seal -p 'your-passphrase'          # → ledger.rrsealed
```

Schema notes: [ledger-schema.md](./ledger-schema.md).

## TW e-invoice QR

```bash
rradar process any.txt --qr-file fixtures/qr/tw_einvoice_sample_01.payload.txt --confirm
```

## Engines

| Engine | Status |
|--------|--------|
| `mock` (default) | Deterministic / fixtures; CI-safe |
| `onnx` | Real RapidOCR: build with `--features onnx`, fetch models + ORT (`tools/fetch-models.ps1 -FetchOrt`), `rradar models verify`, then `--engine onnx` |

```bash
rradar models status
rradar models verify   # requires weights matching models/manifest.sha256
```

```powershell
powershell -File tools/fetch-models.ps1 -FetchOrt
cargo run -p rradar-cli --features onnx -- process receipt.jpg --engine onnx --explain
```

See `models/README.md` for layout, `ORT_DYLIB_PATH`, and zh-TW notes.

## Categories

```bash
rradar categories
```

Ids: `food_dining`, `grocery_convenience`, `transport`, `shopping`, `health`, `utilities`, `entertainment`, `other`.

## Exit codes

- `0` success  
- `1` error (message on stderr)

## Inbox + local API

```bash
rradar inbox --ensure          # create %APPDATA%/receiptradar/inbox
# drop files into inbox, then:
rradar watch                   # default watches inbox
rradar watch --once            # process existing new files and exit

# Local-only HTTP (loopback only; no cloud)
rradar serve                   # http://127.0.0.1:7432
# GET  /health /version /transactions /stats /report?y=2024&m=5
# POST /process  {"path":"C:/tmp/r.txt","confirm":true}
```

## Completeness (CLI product)

- [x] init / doctor / default paths  
- [x] process + confirm + field overrides (+ **batch paths**)  
- [x] **manual** entry without OCR  
- [x] **import json**  
- [x] list / show / edit / delete  
- [x] stats (month + all), no cross-currency sum  
- [x] export CSV (UTF-8 BOM) / JSON  
- [x] encrypted backup + sealed DB  
- [x] QR prefer path  
- [x] `scripts/smoke-cli.ps1`  
- [x] Real ONNX recognition path (`--features onnx` + models + ORT load-dynamic)  

