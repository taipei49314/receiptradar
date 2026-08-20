# Install ReceiptRadar CLI (`rradar`)

Local-first. No account. Core path works offline.

## Option A — From source (recommended for contributors)

**Requirements:** Rust 1.88+ ([rustup](https://rustup.rs/)), C toolchain for `rusqlite` bundled SQLite.

```bash
git clone https://github.com/taipei49314/receiptradar.git
cd receiptradar
cargo install --path crates/rradar-cli --locked
rradar version --long
rradar doctor
```

### Windows (GNU toolchain note)

This project’s unattended builder uses `stable-x86_64-pc-windows-gnu`. MSVC also works for default (mock OCR) builds:

```powershell
# MinGW example — adjust PATH if needed
$env:Path = "C:\path\to\mingw64\bin;$env:USERPROFILE\.cargo\bin;" + $env:Path
powershell -File scripts/install-cli.ps1
```

### Optional ONNX OCR build

```bash
cargo install --path crates/rradar-cli --locked --features onnx
# then fetch models + ORT — see models/README.md
powershell -File tools/fetch-models.ps1 -FetchOrt   # Windows
# ./tools/fetch-models.sh && RRADAR_FETCH_ORT=1 ./tools/fetch-models.sh
rradar engines --json
rradar process photo.jpg --engine onnx --explain
# or: --engine auto  (uses onnx when ready, else mock)
```

## Option B — GitHub Release binaries

Tags matching `v*` run [.github/workflows/release.yml](../.github/workflows/release.yml) and attach:

| Artifact | Platform |
|----------|----------|
| `rradar-x86_64-unknown-linux-gnu.tar.gz` | Linux x86_64 |
| `rradar-x86_64-pc-windows-msvc.zip` | Windows x86_64 |
| `rradar-x86_64-apple-darwin.tar.gz` | macOS Intel |
| `rradar-aarch64-apple-darwin.tar.gz` | macOS Apple Silicon |

Each archive includes:

| File | Purpose |
|------|---------|
| `rradar` / `rradar.exe` | CLI binary (mock OCR default) |
| `VERSION` | tag, crate_version, **ledger_schema**, soft_delete (from `version --json`) |
| `version.json` | machine-readable identity |
| `LICENSE`, `THIRD_PARTY_NOTICES`, `CHANGELOG.md` | legal / history |
| `INSTALL.md`, `cli.md`, `privacy.md`, `ledger-schema.md`, `RELEASE.md` | docs |

A sibling `*.sha256` checksum is published. Schema **v4** supports soft-delete trash (no cloud).

### Linux / macOS

```bash
# example: Linux x86_64
curl -fsSL -O https://github.com/taipei49314/receiptradar/releases/latest/download/rradar-x86_64-unknown-linux-gnu.tar.gz
curl -fsSL -O https://github.com/taipei49314/receiptradar/releases/latest/download/rradar-x86_64-unknown-linux-gnu.sha256
sha256sum -c rradar-x86_64-unknown-linux-gnu.sha256
tar -xzf rradar-x86_64-unknown-linux-gnu.tar.gz
sudo install -m 755 rradar-x86_64-unknown-linux-gnu/rradar /usr/local/bin/rradar
rradar version --long
```

Or use the helper:

```bash
./scripts/install-from-release.sh          # auto-detect OS/arch
./scripts/install-from-release.sh v0.1.0-cli.30
```

### Windows (PowerShell)

```powershell
powershell -File scripts/install-from-release.ps1
# or pin a tag:
powershell -File scripts/install-from-release.ps1 -Tag v0.1.0-cli.30
rradar version --long
rradar engines
powershell -File scripts/verify-install.ps1
```

Binaries land in `%USERPROFILE%\.cargo\bin` when that directory exists, else `~\bin` (printed by the script).

## First-run smoke

```bash
rradar init
rradar process fixtures/text/familymart_89.txt --confirm --explain   # from git checkout
rradar list
rradar stats --all
rradar engines
rradar demo --quiet   # full closed loop (needs fixtures/)
# one-shot install/release gate (process + demo + local API smoke):
rradar release-check --fixtures fixtures
# or helpers:
#   ./scripts/verify-install.sh
#   powershell -File scripts/verify-install.ps1
```

## Data locations

| | Path |
|--|------|
| Home | `%APPDATA%\receiptradar` (Windows) / `~/.local/share/receiptradar` |
| Ledger | `…/ledger.db` |
| Override | `RRADAR_HOME`, `RRADAR_DB` |

## Uninstall

```bash
cargo uninstall rradar-cli   # if installed via cargo
# or remove the binary placed by install-from-release.*
```

Your ledger under the data home is **not** deleted automatically.

## See also

- [RELEASE.md](./RELEASE.md) — maintainer checklist  
- [cli.md](./cli.md) — command reference  
- [models/README.md](../models/README.md) — optional ONNX weights  
