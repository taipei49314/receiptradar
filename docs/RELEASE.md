# CLI release checklist

## Pre-flight

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `scripts/smoke-cli.ps1` (Windows) or equivalent
- [ ] `python tools/network-audit/check_offline_deps.py`
- [ ] `docs/licenses-checklist.md` reviewed for binary deps
- [ ] CHANGELOG updated

## Version

1. Bump `workspace.package.version` in root `Cargo.toml` if needed  
2. Tag: `git tag v0.1.0-cli.1`  
3. Push tag → `.github/workflows/release.yml` builds multi-OS `rradar` artifacts  

## Install from source

```bash
cargo install --path crates/rradar-cli --locked
rradar doctor
```

Windows helper: `scripts/install-cli.ps1`

## Smoke after install

```bash
rradar init
rradar process fixtures/text/familymart_89.txt --confirm -q
rradar list
rradar stats --all
# From a checkout with fixtures/:
rradar demo --quiet
```

## Not in CLI release

- Real ONNX weights (separate asset)  
- Flutter APK  
- Official sync / cloud  
