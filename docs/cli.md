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

## Daily workflow

```bash
# 1) Parse a receipt (text fixture or image with mock/onnx)
rradar process receipt.txt --explain

# 2) Confirm into default ledger (override fields if needed)
rradar process receipt.txt --confirm \
  --merchant "全家臨江店" --amount 89 --category grocery_convenience

# 3) Browse
rradar list
rradar list --query 全家 --currency TWD
rradar show <id>
rradar stats              # this calendar month, per currency
rradar stats --all
rradar stats --from 2024-01-01 --to 2024-12-31
rradar top --currency TWD --limit 10

# 4) Fix mistakes
rradar edit <id> --amount 99 --notes "招待客戶"
rradar delete <id> --yes

# 5) Export / backup
rradar export csv -o month.csv
rradar backup create -p 'your-passphrase' -o backup.rradar
rradar seal -p 'your-passphrase'          # → ledger.rrsealed
```

## TW e-invoice QR

```bash
rradar process any.txt --qr-file fixtures/qr/tw_einvoice_sample_01.payload.txt --confirm
```

## Engines

| Engine | Status |
|--------|--------|
| `mock` (default) | Deterministic / fixtures; CI-safe |
| `onnx` | Needs models under `models/` + future ORT link |

## Categories

```bash
rradar categories
```

Ids: `food_dining`, `grocery_convenience`, `transport`, `shopping`, `health`, `utilities`, `entertainment`, `other`.

## Exit codes

- `0` success  
- `1` error (message on stderr)

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
- [ ] Real ONNX recognition (optional upgrade)  
