# ReceiptRadar（發票雷達）

**Snap. Parse. Own your spending.**  
Offline receipt → ledger. Local-first. No account.

> **CLI product is complete** for daily local bookkeeping (mock OCR + text/QR fixtures).  
> Real ONNX + mobile camera are optional next layers.

[中文](./README.zh-TW.md) · [CLI guide](./docs/cli.md) · [Design](./docs/design-full.md)

## Install & quick start

```bash
cargo install --path crates/rradar-cli
rradar init
rradar process fixtures/text/familymart_89.txt --confirm --explain
rradar list
rradar stats
rradar export csv -o out.csv
rradar backup create -p 'choose-a-passphrase'
```

Default database: `%APPDATA%\receiptradar\ledger.db` (Windows) or `~/.local/share/receiptradar/ledger.db`.

```bash
rradar doctor   # paths, ledger health, engines
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
| Encrypted backup | `backup create\|restore` |
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
| `onnx` | After model pin (A04/A05); not required for CLI product |

## Develop

```bash
cargo test --workspace
cargo run -p rradar-cli -- doctor
cargo run -p rradar-cli -- help process
cargo run -p bench-ocr -- fixtures/text
# Windows smoke:
powershell -File scripts/smoke-cli.ps1
```

Release: [docs/RELEASE.md](./docs/RELEASE.md) · tag `v*` runs [.github/workflows/release.yml](./.github/workflows/release.yml).

## License

Apache-2.0 (source). Model weights declared separately when shipped.
