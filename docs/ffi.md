# Mobile FFI contract (`rradar-ffi`)

Local-first Rust API for Flutter via **flutter_rust_bridge** (not generated yet).  
All money/ledger logic stays in Rust; Dart owns UI, camera, and file pickers.

## Crate layout

| Item | Value |
|------|--------|
| Crate | `crates/rradar-ffi` |
| Crate types | `lib`, `staticlib`, `cdylib` |
| Error style | `Result<T, String>` |
| Complex types | JSON strings (`serde_json`) |

```bash
cargo test -p rradar-ffi
cargo build -p rradar-ffi --release
```

## API map (Cycle 14)

### Identity / device
| Function | Purpose |
|----------|---------|
| `api_version` | Smoke string |
| `product_id` / `core_version` | Branding |
| `supported_ledger_schema` | Migration ceiling |
| `default_data_dir` / `default_ledger_path` | Paths |
| `capabilities_json` | Feature flags for About UI |

### Process
| Function | Purpose |
|----------|---------|
| `process_receipt_path_json` | File path → draft JSON |
| `process_receipt_path_json_ex` | + optional TW QR payload |
| `process_image_bytes_json` | Camera bytes → draft JSON |

### Ledger
| Function | Purpose |
|----------|---------|
| `ensure_ledger` | Create/open SQLite |
| `ledger_schema_version` | On-disk schema |
| `count_transactions` | Count |
| `confirm_draft_json` / `_ex` | Insert draft |
| `list_transactions_json` | List |
| `get_transaction_json` | Show one |
| `last_transaction_json` | Last confirmed |
| `delete_transaction` | Delete |
| `update_transaction_json` | Edit fields |
| `stats_all_json` / `stats_month_json` | Totals |
| `top_merchants_json` | Rankings |
| `categories_json` | Taxonomy ids |
| `backup_create_file` | Encrypted backup to path |

## Non-goals (FFI)

- Official sync / relay / accounts  
- Shipping ONNX weights inside the APK without size spike (A04/A05)  
- GPT wrappers  

## FRB generation (when Flutter SDK is installed)

```bash
# Example — adjust versions to project FRB pin when chosen
cd apps/mobile
flutter pub add flutter_rust_bridge
# Point FRB at crates/rradar-ffi free functions; generate into lib/bridge/generated/
# Link staticlib/cdylib per Android NDK + iOS later (v0.1 = Android only)
```

Until generation runs:

1. UI uses `lib/services/rradar_api.dart` **mock** implementation.
2. `lib/bridge/README.md` describes the swap to native.
3. CI stays Rust-only (no Flutter job required).

## Android notes (design KD-15)

- minSdk 26, arm64-v8a  
- FLAG_SECURE default ON (shell documents intent; native later)  
- Offline flavor: no INTERNET permission (Track A)

## Related

- [ledger-schema.md](./ledger-schema.md) — SQLite schema / multi-device backup  
- [apps/mobile/README.md](../apps/mobile/README.md) — Flutter shell  
