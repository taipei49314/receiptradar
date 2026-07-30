# Third-party license checklist (A17)

Complete before first public binary / `v0.1.0` release tag.

| Component | Version source | License | Redistribution OK? | Notes |
|-----------|----------------|---------|--------------------|-------|
| Rust crates (Cargo.lock) | `cargo tree` | various | ☐ | Run `cargo license` or cargo-deny |
| rusqlite / SQLite amalgamation | bundled | blessing / public domain | ☐ | |
| chacha20poly1305, argon2, hkdf, sha2 | crates.io | MIT/Apache | ☐ | |
| ONNX Runtime (when enabled) | release asset | check ORT | ☐ | Post A05 |
| RapidOCR / Paddle weights | model pack | check NOTICE | ☐ | Separate release asset |
| Flutter + plugins | pubspec (later) | various | ☐ | A18+ |
| SQLCipher community (if P1) | amalgamation | zlib-like | ☐ | Prefer P2 sealed if not used |

## Commands

```bash
cargo tree -i openssl  # should be empty for core path
# optional: cargo install cargo-deny && cargo deny check
```

## Project source

- **Apache-2.0** — see `LICENSE`
- Models never silently embedded without hash + NOTICE
