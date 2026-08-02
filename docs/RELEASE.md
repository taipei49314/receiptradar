# CLI release checklist

## Pre-flight (local)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --release -p rradar-cli --locked
./target/release/rradar version --long
./target/release/rradar engines
./target/release/rradar release-check --fixtures fixtures
./target/release/rradar demo --fixtures fixtures --db /tmp/rradar-rel-demo.db --quiet
python tools/network-audit/check_offline_deps.py
python tools/supply-chain/check_deps.py
cargo deny check   # dual gate; cargo install cargo-deny --version 0.20.2
./target/release/rradar fixtures verify --fixtures fixtures
# dual wrapper:
# ./scripts/check-supply-chain.sh --install-deny
# optional post-install style:
# ./scripts/verify-install.sh ./target/release/rradar
```

Windows:

```powershell
powershell -File scripts/smoke-cli.ps1
powershell -File scripts/demo.ps1
cargo run -p rradar-cli -- release-check --fixtures fixtures
cargo run -p rradar-cli -- fixtures verify --fixtures fixtures
powershell -File scripts/check-supply-chain.ps1 -InstallDeny
powershell -File scripts/verify-install.ps1 -Bin target\release\rradar.exe
```

Also:

- [ ] `docs/licenses-checklist.md` reviewed for binary deps  
- [ ] `THIRD_PARTY_NOTICES` still accurate for default mock CLI  
- [ ] `deny.toml` / `cargo deny check` green  
- [ ] CHANGELOG updated  
- [ ] `docs/INSTALL.md` still accurate for artifact names  

## Version & tag

1. Optionally bump `workspace.package.version` in root `Cargo.toml`  
2. Commit on `master`  
3. Tag (lightweight or annotated):

```bash
git tag -a v0.1.0-cli.N -m "describe the milestone"
git push origin master
git push origin v0.1.0-cli.N
```

4. Tag push runs [`.github/workflows/release.yml`](../.github/workflows/release.yml):
   - Linux x86_64, Windows MSVC, macOS Intel, **macOS aarch64**
   - `--locked` release build + `version --long` smoke
   - Archives + per-file SHA-256 + GitHub Release notes  

## CI gates (every PR / push)

[`.github/workflows/ci.yml`](../.github/workflows/ci.yml):

| Step | Purpose |
|------|---------|
| fmt / clippy / test `--locked` | Correctness |
| `rradar-ffi` tests | Mobile contract |
| cli smoke + engines | Product path |
| **soft-delete trash smoke** | schema v4 trash → restore |
| demo + api-smoke | Closed loops |
| **release-check** | Pre-flight (process, soft-delete, integrity, demo, api) |
| **fixtures verify** | Offline extract matrix totals |
| **release binary smoke** | `cargo build --release` + release-check + demo |
| network audit | No surprise egress in source |
| supply-chain python + **cargo-deny** | Dual license/source gate |

### Release archives

`VERSION` / `version.json` are written from **`rradar version --json`** at package time (ledger_schema must match the binary; never hardcode). Includes `docs/ledger-schema.md`.

## Install from release

Users: [INSTALL.md](./INSTALL.md) · helpers `scripts/install-from-release.sh` / `.ps1`.

## Not in CLI release assets

- Real ONNX weights (fetch separately; `models/README.md`)  
- Flutter APK  
- Official sync / cloud  

## Rollback

- Ledger migrations are **forward-only**; keep a `backup.rradar` before major upgrades.  
- Users on newer schema need a newer binary (`SchemaTooNew`).  
