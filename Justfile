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

cli-onnx *args:
    cargo run -q -p rradar-cli --features onnx -- {{args}}
