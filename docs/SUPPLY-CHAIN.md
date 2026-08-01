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

## Dual gate (Cycle 32+)

| Gate | Tool | Always? |
|------|------|---------|
| **1** | `python tools/supply-chain/check_deps.py` | **Yes** (CI + local) |
| **2** | `cargo deny check` via root `deny.toml` | **Yes in CI**; optional locally |

```bash
# One-shot dual gate
./scripts/check-supply-chain.sh
# Windows:
powershell -File scripts/check-supply-chain.ps1
# Install cargo-deny if missing:
powershell -File scripts/check-supply-chain.ps1 -InstallDeny
./scripts/check-supply-chain.sh --install-deny

# Python only (no cargo-deny binary):
powershell -File scripts/check-supply-chain.ps1 -SkipDeny
```

`cargo-deny` **0.20+** is required (CVSS 4.0 advisory entries). Pin used in CI: **0.20.2**.

```bash
cargo install cargo-deny --locked --version 0.20.2
cargo deny check
```

Config: [`deny.toml`](../deny.toml) — allowlist of OSI-friendly licenses; crates.io sources only; bans AGPL-class via cargo-deny license deny + python forbidden substrings.

## Local gates

```bash
# Network / URL audit (source text)
python tools/network-audit/check_offline_deps.py

# License + dependency closure (Cargo.lock)
python tools/supply-chain/check_deps.py
python tools/supply-chain/check_deps.py --write-inventory   # refreshes docs/dependency-inventory.md

# Product preflight + fixture matrix
rradar release-check --fixtures fixtures
rradar fixtures verify
rradar licenses
```

## CI

| Job | Check |
|-----|-------|
| `ci` | tests, clippy, network-audit, **python supply-chain**, **cargo-deny**, **fixtures verify**, release-check |
| `release` | multi-platform binaries + release-check + notices in package |

## Commands

```bash
rradar licenses           # print THIRD_PARTY_NOTICES + policy
rradar licenses --json    # machine-readable summary
cargo metadata --locked --format-version 1 | head  # raw graph
cargo deny check
```

## Maintainer checklist

See also [licenses-checklist.md](./licenses-checklist.md). Before a public tag:

1. `SUPPLY_CHAIN_OK` + `cargo deny check` on clean tree  
2. `THIRD_PARTY_NOTICES` still accurate for **default mock** CLI  
3. ONNX path documented as optional (`--features onnx`)  
4. `rradar release-check` + `rradar fixtures verify` green  

## Non-goals

- Hosted package registry mirror  
- Automatic CVE auto-fix without review  
- Bundling ONNX Runtime in the default GitHub Release binary  
