# Supply chain & release trust

Local-first product policy for **ReceiptRadar**:

| Rule | Detail |
|------|--------|
| No phone-home | Core path offline; CI runs `tools/network-audit/check_offline_deps.py` |
| No official cloud | Multi-device = encrypted backup / handoff files only |
| Project license | **Apache-2.0** (`LICENSE`) |
| Notices | `THIRD_PARTY_NOTICES` ships in release archives |
| Model weights | Not in git; SHA-256 pins in `models/manifest.sha256` |
| Forbidden deps | AGPL / SSPL / BUSL / Commons Clause (hard fail) |

## Local gates

```bash
# Network / URL audit (source text)
python tools/network-audit/check_offline_deps.py

# License + dependency closure (Cargo.lock)
python tools/supply-chain/check_deps.py
python tools/supply-chain/check_deps.py --write-inventory   # refreshes docs/dependency-inventory.md

# Product preflight
rradar release-check --fixtures fixtures
rradar licenses
```

## CI

| Job | Check |
|-----|-------|
| `ci` | tests, clippy, network-audit, **supply-chain**, release-check |
| `release` | multi-platform binaries + release-check + notices in package |

## Commands

```bash
rradar licenses           # print THIRD_PARTY_NOTICES + policy
rradar licenses --json    # machine-readable summary
cargo metadata --locked --format-version 1 | head  # raw graph
```

## Maintainer checklist

See also [licenses-checklist.md](./licenses-checklist.md). Before a public tag:

1. `SUPPLY_CHAIN_OK` on clean tree  
2. `THIRD_PARTY_NOTICES` still accurate for **default mock** CLI  
3. ONNX path documented as optional (`--features onnx`)  
4. `rradar release-check` green  

## Non-goals

- Hosted package registry mirror  
- Automatic CVE auto-fix without review  
- Bundling ONNX Runtime in the default GitHub Release binary  
