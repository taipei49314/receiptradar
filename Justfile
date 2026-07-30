# ReceiptRadar — common commands (requires `just`)

default:
    @just --list

test:
    cargo test --workspace

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

fmt:
    cargo fmt --all

check: fmt clippy test

cli *args:
    cargo run -q -p rradar-cli -- {{args}}

smoke:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/smoke-cli.ps1

install:
    cargo install --path crates/rradar-cli --force --locked

doctor:
    cargo run -q -p rradar-cli -- doctor
