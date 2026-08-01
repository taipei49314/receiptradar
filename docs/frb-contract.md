# flutter_rust_bridge contract (`rradar-ffi`)

**Status:** FRB codegen deferred until Flutter SDK is on the builder.  
This document is the **generate checklist** for `apps/mobile` native wiring.

## How to generate (when Flutter is available)

```bash
# 1) NDK arm64 lib
cargo ndk -t arm64-v8a -o apps/mobile/android/app/src/main/jniLibs \
  build -p rradar-ffi --release

# 2) FRB codegen (version-pin flutter_rust_bridge in pubspec)
cd apps/mobile
flutter pub get
# flutter_rust_bridge_codegen generate \
#   --rust-input crate::api \
#   --dart-output lib/bridge/generated/

# 3) Implement NativeRradarApi over generated free functions
# 4) Set rradarApi = NativeRradarApi() after DynamicLibrary open
```

See [android-ffi.md](./android-ffi.md), [ffi.md](./ffi.md).

## Free functions to bind (crate `rradar-ffi`)

### Identity / paths
| Rust fn | Dart facade |
|---------|-------------|
| `api_version` | `apiVersion` |
| `product_id` / `core_version` | (about) |
| `capabilities_json` | `capabilities` |
| `engines_json` | `enginesJson` |
| `default_data_dir` / `default_ledger_path` | `defaultLedgerPath` |
| `default_inbox_path` / `ensure_inbox` | `defaultInboxPath` |
| `default_rules_path` / `ensure_rules` | `defaultRulesPath` |
| `default_attachments_path` / `attachments_dir_for_ledger` | (paths) |

### Capture / process
| Rust fn | Notes |
|---------|--------|
| `process_receipt_path_json` / `_ex` | draft only |
| `process_image_bytes_json` | camera bytes |
| **`process_confirm_path_json`** | one-shot path |
| **`process_confirm_bytes_json`** | one-shot camera |
| `confirm_draft_json` / `_ex` | multi-step confirm |

### Ledger
| Rust fn | Notes |
|---------|--------|
| `ensure_ledger` | open/migrate |
| `count_transactions` | |
| `list_transactions_json` | |
| **`query_transactions_json`** | tag/category/query filters |
| **`list_tags_json`** | schema v3 tags |
| `get_transaction_json` / `last_transaction_json` | |
| `update_transaction_json` | tags/attachment clear = empty string |
| `delete_transaction` | |
| `attach_file_json` / `attach_bytes_json` / `detach_file_json` | |

### Budgets (local soft limits)
| Rust fn | Notes |
|---------|--------|
| `budgets_json` | book file |
| **`budget_status_json`** | month evaluation |
| `budget_set_json` | upsert line |

### Analytics / rules / models
| Rust fn | Notes |
|---------|--------|
| `stats_all_json` / `stats_month_json` / `stats_by_category_json` | never mix CCY |
| `report_month_markdown` | |
| `top_merchants_json` | |
| `categories_json` | |
| `list_rule_packs_json` | |
| `models_pins_json` | |

### Backup / handoff (no cloud)
| Rust fn | Notes |
|---------|--------|
| `backup_create_file` | includes attachments + **budgets.toml** when present |
| `handoff_create_file` / `handoff_info_json` / `handoff_apply_merge_json` | same |

## Non-goals

- Official sync / relay / accounts  
- Shipping ONNX weights in APK without A04 Green  

## Policy string

`local-first; multi-device via backup/handoff file only`
