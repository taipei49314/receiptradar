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

## API map (Cycle 20)

### Identity / device
| Function | Purpose |
|----------|---------|
| `api_version` | Smoke string |
| `product_id` / `core_version` | Branding |
| `supported_ledger_schema` | Migration ceiling (v3) |
| `default_data_dir` / `default_ledger_path` | Paths |
| `default_inbox_path` / `ensure_inbox` | Drop folder |
| `default_rules_path` / `ensure_rules` | Rule packs dir |
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
| `update_transaction_json` | Edit fields (+ **tags**, **attachment_path**; empty string clears) |
| `attach_file_json` | Copy local file into `{db}/attachments/{id}/` + set path |
| `attach_bytes_json` | Store camera/in-memory bytes as attachment |
| `detach_file_json` | Clear path; optional delete stored blob |
| `resolve_attachment_path_string` | Relative → absolute next to ledger |
| `attachments_dir_for_ledger` / `default_attachments_path` | Store roots |
| **`process_confirm_path_json`** | **Capture one-shot:** path → process → confirm → attach/tags |
| **`process_confirm_bytes_json`** | **Camera one-shot:** bytes → process → confirm → attach/tags |
| `stats_all_json` / `stats_month_json` | Totals |
| `stats_by_category_json` | Category breakdown |
| `report_month_markdown` | Monthly report |
| `top_merchants_json` | Rankings |
| `categories_json` | Taxonomy ids |

### Rules / models
| Function | Purpose |
|----------|---------|
| `list_rule_packs_json` | Installed pack paths |
| `models_pins_json` | ONNX pin status |

### Backup / handoff (no cloud)
| Function | Purpose |
|----------|---------|
| `backup_create_file` | Encrypted backup.rradar (**includes attachment blobs**) |
| `handoff_create_file` | Multi-device handoff package (+ attachments) |
| `handoff_info_json` | Inspect handoff |
| `handoff_apply_merge_json` | Merge into local ledger (+ rehydrate blobs) |

## Non-goals (FFI)

- Official sync / relay / accounts  
- Shipping ONNX weights inside the APK without size spike (A04/A05)  
- GPT wrappers  

## FRB generation (when Flutter SDK is installed)

```bash
cd apps/mobile
flutter pub get
# Install flutter_rust_bridge_codegen matching chosen FRB version
# Point at crates/rradar-ffi free functions → lib/bridge/generated/
# Implement NativeRradarApi with generated bindings
# Android: cargo-ndk -t arm64-v8a -o android/app/src/main/jniLibs build -p rradar-ffi --release
```

See [android-ffi.md](./android-ffi.md).

Until generation runs:

1. UI uses `lib/services/rradar_api.dart` **mock** implementation.
2. `lib/bridge/README.md` describes the swap to native.
3. CI stays Rust-only (no Flutter job required).

## Android notes (design KD-15)

- minSdk 26, arm64-v8a  
- FLAG_SECURE default ON (shell documents intent; native later)  
- Offline flavor: no INTERNET permission (Track A)

## Related

- [ledger-schema.md](./ledger-schema.md) — SQLite schema / multi-device  
- [local-api.md](./local-api.md) — desktop loopback HTTP (not required on mobile)  
- [apps/mobile/README.md](../apps/mobile/README.md) — Flutter shell  
