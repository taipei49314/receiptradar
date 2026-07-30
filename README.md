# ReceiptRadar（發票雷達）

**Snap. Parse. Own your spending.**  
*Offline receipt → ledger in seconds. Local-first. No account.*

> **Status:** early Track A scaffold (`v0.1.0-alpha`). Real ONNX OCR, Flutter camera loop, and encrypted backup are planned — not all shipped yet.

[中文說明](./README.zh-TW.md)

## Why

Most people do not fail at bookkeeping because they cannot categorize expenses — they fail because **typing every receipt is friction**. Cloud finance apps fix friction by uploading your receipt photos to someone else's servers.

ReceiptRadar keeps the capture → parse → ledger path **on your device**:

- **No account**
- **No cloud required** for the core path
- **On-device** OCR (and Taiwan e-invoice **QR-first** when present)
- Optional network features are **opt-in** and build-flavor gated

## Thin vertical slice (v0.1)

| In v0.1 (Track A) | Explicitly later (Track B) |
|-------------------|----------------------------|
| CLI with real ONNX OCR on desktop | Full WASM browser OCR |
| Pixel → field golden fixtures | Desktop Tauri companion |
| Android debug APK: camera → review → ledger | Self-host E2E sync |
| Encrypted backup `backup.rradar` v1 | Budgets, batch queue, iOS |
| At-rest DB + image encryption | Official sync relay (**never** — project policy) |

**Public launch gate (Tier T0):** demo GIF + `rradar process` on fixtures. Store listing is not required.

## Quick start (scaffold)

Requirements: Rust **1.78+** (`rustup`).

```bash
cargo test --workspace
cargo run -p rradar-cli -- version
cargo run -p rradar-cli -- help
```

`rradar process` lands after the OCR spike and pipeline PRs (see design doc).

## Architecture (target)

```text
┌─────────────────────────────────────────────┐
│  Flutter (Android)  ·  rradar CLI           │
│         │ flutter_rust_bridge / CLI         │
│         ▼                                   │
│  rradar-core  (Rust)                        │
│    preprocess → QR prefer → OCR → L1 rules  │
│    → category → SQLite ledger → backup      │
│         ▲                                   │
│  rradar-ocr (OcrEngine: mock → ONNX)        │
└─────────────────────────────────────────────┘
         data stays on device by default
```

## Privacy (defaults)

| Mode | Network | Purpose |
|------|---------|---------|
| **A — offline flavor** | No `INTERNET` permission | Airplane-mode capable build |
| **B — full + user download** | User-initiated model download only | Hash-pinned models |
| **C — opt-in features** | Explicit toggles only | Never silent phone-home |

Core claim: receipt images and ledger data **do not leave the device** unless you export a backup or enable an opt-in feature.

See [docs/privacy.md](./docs/privacy.md).

## Repo layout

```text
receiptradar/
├── crates/
│   ├── rradar-core/     # types, pipeline, ledger (grows in Track A)
│   ├── rradar-ocr/      # OcrEngine trait + mock (+ ONNX later)
│   └── rradar-cli/      # `rradar` binary
├── apps/                # Flutter mobile (PR-A18+)
├── docs/
├── fixtures/            # golden matrix (PR-A11)
└── .github/workflows/
```

## Roadmap pointer

Implementation follows the design document:

- Local copy: [`../ReceiptRadar-design-doc.md`](../ReceiptRadar-design-doc.md) (or your path)
- Track A: PR-A01 (this scaffold) → … → PR-A25 `v0.1.0`
- Binding risk gate: **PR-A04 OCR + size spike** (Green/Yellow/Orange/Red)

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md). Good first issues will include merchant YAML seeds and fixture classes — after taxonomy lands.

## License

- **Source code:** [Apache-2.0](./LICENSE)
- **OCR model weights / third-party:** declared separately in `THIRD_PARTY_NOTICES` (when models ship) and release assets — not silently bundled without hashes

## Trademark

"ReceiptRadar" / "發票雷達" naming is subject to a pre-launch trademark check (open design question OQ-1).
