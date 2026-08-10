# Agent notes (ReceiptRadar)

## Goal

Local-first receipt → ledger. CLI product is the primary shippable surface.

## Commands

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p rradar-cli -- doctor
powershell -File scripts/smoke-cli.ps1
cargo run -p rradar-cli -- measure --fixtures fixtures
```

## Non-goals

- Official sync / cloud accounts
- GPT chat finance wrappers
- Force-push to main without explicit human request

## Autonomy

See `docs/AUTONOMY.md`. Prefer green tests before expanding scope.
