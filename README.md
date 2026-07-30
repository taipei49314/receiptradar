# ReceiptRadar（發票雷達）

[![ci](https://github.com/taipei49314/receiptradar/actions/workflows/ci.yml/badge.svg)](https://github.com/taipei49314/receiptradar/actions/workflows/ci.yml)

**Snap. Parse. Own your spending.**  
Offline receipt → ledger. Local-first. No account.

**Repo:** https://github.com/taipei49314/receiptradar

> **CLI product is complete** for daily local bookkeeping (mock OCR + text/QR fixtures).  
> Real ONNX OCR is available behind `--features onnx` (see `models/README.md`).

[中文](./README.zh-TW.md) · [CLI guide](./docs/cli.md) · [Design](./docs/design-full.md)

## Install & one-click demo

Cross-platform install: **[docs/INSTALL.md](./docs/INSTALL.md)** (source, GitHub Release binaries, ONNX).

```bash
cargo install --path crates/rradar-cli --locked
rradar version --long
# From repo root — full closed loop (parse → ledger → stats → export → backup)
rradar demo
# Windows helper:
#   powershell -File scripts/demo.ps1
# Binary from latest GitHub Release:
#   ./scripts/install-from-release.sh
#   powershell -File scripts/install-from-release.ps1
```

Daily use after `rradar init`:

```bash
rradar process fixtures/text/familymart_89.txt --confirm --explain
rradar list
rradar stats
rradar export csv -o out.csv
rradar backup create -p 'choose-a-passphrase'
```

Default database: `%APPDATA%\receiptradar\ledger.db` (Windows) or `~/.local/share/receiptradar/ledger.db`.  
Demo ledger (isolated): `…/receiptradar/demo/ledger.db` (recreated each `rradar demo`).

```bash
rradar doctor   # paths, schema version, engines
rradar help
```

## What the CLI does

| Feature | Command |
|---------|---------|
| Parse receipt | `process` / `add` |
| Confirm to ledger | `process … --confirm` |
| Override fields | `--merchant --amount --category --date --notes` |
| Browse | `list`, `show` |
| Fix / remove | `edit`, `delete --yes` |
| Monthly totals (per currency) | `stats` |
| Export | `export csv\|json` |
| Encrypted backup | `backup create\|restore\|info\|verify` (+ `--merge`) |
| Import merge | `import json` / `import backup` |
| Schema migrate | `migrate` (local SQLite; no cloud) |
| At-rest seal | `seal` / `unseal` |
| TW e-invoice QR | `--qr` / `--qr-file` |

**Never** sums different currencies together.

## Privacy

- Core path: **no network**
- Images / ledger stay on device unless you export
- Optional `.rrsealed` whole-file encryption + `backup.rradar` (Argon2id + XChaCha20-Poly1305)

## OCR engines

| Engine | Use |
|--------|-----|
| `mock` (default) | Fixtures, CI, development |
| `onnx` | Real RapidOCR: `tools/fetch-models.ps1 -FetchOrt` then `cargo run -p rradar-cli --features onnx -- process img.jpg --engine onnx` |

Details: [models/README.md](./models/README.md).

## Mobile / FFI

Rust mobile contract: [docs/ffi.md](./docs/ffi.md) · crate `rradar-ffi` (`staticlib`/`cdylib`).  
Flutter shell: [apps/mobile](./apps/mobile) (mock `RradarApi` until FRB generate).

```bash
cargo test -p rradar-ffi
```

## Develop

```bash
cargo test --workspace
cargo run -p rradar-cli -- doctor
cargo run -p rradar-cli -- help process
cargo run -p bench-ocr -- fixtures/text
# Windows smoke:
powershell -File scripts/smoke-cli.ps1
# Optional real OCR build (heavy deps + models not in git):
cargo test -p rradar-ocr --features onnx
```

Release: [docs/RELEASE.md](./docs/RELEASE.md) · tag `v*` runs [.github/workflows/release.yml](./.github/workflows/release.yml).

## License

Apache-2.0 (source). Model weights declared separately when shipped.
