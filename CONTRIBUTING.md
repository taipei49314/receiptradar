# Contributing to ReceiptRadar

Thanks for helping build a local-first receipt ledger.

## Scope discipline

v0.1 is a **thin vertical slice**. Please read the design doc before large PRs:

- Prefer changes that advance Track A (CLI OCR, Android capture loop, encryption, fixtures).
- Track B ideas (sync, budgets, iOS, WASM OCR, household ledger) should be issues first, not surprise PRs.

## Development

### Prerequisites

- Rust 1.78+ via [rustup](https://rustup.rs/)
- Later: Flutter 3.x, Android NDK (mobile PRs)

### Commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p rradar-cli -- version
```

### Commit style

- Prefer small, reviewable PRs aligned with design **PR-Axx** names when possible.
- Conventional prefixes: `feat`, `fix`, `chore`, `docs`, `test`, `spike`.

## Privacy & fixtures

- Do **not** commit real receipt photos that contain uncleared PII (membership barcodes, phones, faces).
- Fixture policy will live in `fixtures/README.md` (PR-A11): prefer team-captured consented shots, redaction, or synthetic layouts for public CI.
- Maintainers will not accept raw personal receipts in issues without scrubbing.

## Code of conduct expectations

- Be respectful and specific in review.
- Security-sensitive reports: open a private advisory when the GitHub repo enables it; until then, contact maintainers offline if credentials leak is involved.

## License

By contributing, you agree your contributions are licensed under the **Apache-2.0** license for project source code.
