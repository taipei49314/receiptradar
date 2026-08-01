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

# Recordable closed-loop demo (isolated target/demo ledger)
demo:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/demo.ps1

fixtures:
    cargo run -q -p rradar-cli -- fixtures list --fixtures fixtures

fixtures-verify:
    cargo run -q -p rradar-cli -- fixtures verify --fixtures fixtures

record-demo:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/record-demo.ps1

# Ephemeral loopback HTTP product smoke (no curl)
api-smoke:
    cargo run -q -p rradar-cli -- api-smoke --fixtures fixtures

# Pre-flight for release/install (local-only)
release-check:
    cargo run -q -p rradar-cli -- release-check --fixtures fixtures

verify-install:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-install.ps1

install:
    cargo install --path crates/rradar-cli --force --locked

# Build with real ONNX RapidOCR (needs models + ORT at runtime — models/README.md)
install-onnx:
    cargo install --path crates/rradar-cli --force --locked --features onnx

doctor:
    cargo run -q -p rradar-cli -- doctor

doctor-onnx:
    cargo run -q -p rradar-cli --features onnx -- doctor

fetch-models:
    powershell -NoProfile -ExecutionPolicy Bypass -File tools/fetch-models.ps1 -FetchOrt

models-verify:
    cargo run -q -p rradar-cli -- models verify

# Optional: real ONNX e2e (weights + ORT 1.22 — not default CI)
smoke-onnx:
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts/smoke-onnx.ps1

cli-onnx *args:
    cargo run -q -p rradar-cli --features onnx -- {{args}}

# Full local gate (matches CI spirit)
ci-local:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --locked
    cargo build --release -p rradar-cli --locked
    ./target/release/rradar version --long

release-smoke:
    cargo build --release -p rradar-cli --locked
    ./target/release/rradar version --long
    ./target/release/rradar demo --fixtures fixtures --db target/release-smoke/ledger.db --quiet
