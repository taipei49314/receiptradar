# CLI release checklist

## Pre-flight (local)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build --release -p rradar-cli --locked
./target/release/rradar version --long
./target/release/rradar demo --fixtures fixtures --db /tmp/rradar-rel-demo.db --quiet
python tools/network-audit/check_offline_deps.py
```

Windows:

```powershell
powershell -File scripts/smoke-cli.ps1
powershell -File scripts/demo.ps1
```

Also:

- [ ] `docs/licenses-checklist.md` reviewed for binary deps  
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
| cli smoke + demo | Product path |
| **release binary smoke** | `cargo build --release` + demo |
| network audit | No surprise egress in source |

## Install from release

Users: [INSTALL.md](./INSTALL.md) · helpers `scripts/install-from-release.sh` / `.ps1`.

## Not in CLI release assets

- Real ONNX weights (fetch separately; `models/README.md`)  
- Flutter APK  
- Official sync / cloud  

## Rollback

- Ledger migrations are **forward-only**; keep a `backup.rradar` before major upgrades.  
- Users on newer schema need a newer binary (`SchemaTooNew`).  
