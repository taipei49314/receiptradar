# ReceiptRadar mobile (Flutter)

**PR-A18 shell** — privacy onboarding + home + capture placeholder.  
**PR-A19** Rust FFI contract lives in `crates/rradar-ffi` + `lib/services/rradar_api.dart` (FRB codegen pending).

## Requirements

- Flutter 3.22+ / Dart 3.3+ (optional for Rust-only CI)
- Android minSdk **26**, **arm64-v8a** only (design KD-15)

## Architecture (local-first)

```
UI (Dart)  →  RradarApi facade  →  [Mock now | FRB later]  →  rradar-ffi  →  rradar-core
```

- No official cloud sync / relay.
- Multi-device: encrypted `backup.rradar` only (`backup_create_file` on FFI).

## Run (when Flutter is installed)

```bash
cd apps/mobile
flutter pub get
flutter run
```

Without Flutter on PATH, this tree is source-only; **Rust CI still validates `rradar-ffi`**.

```bash
# from repo root
cargo test -p rradar-ffi
```

## Key files

| Path | Role |
|------|------|
| `lib/main.dart` | App entry, FLAG_SECURE intent |
| `lib/services/rradar_api.dart` | API facade + mock |
| `lib/bridge/README.md` | FRB wiring notes |
| `../../docs/ffi.md` | Full FFI function map |
| `../../crates/rradar-ffi` | Rust free functions |

## Platform notes

| Item | Plan |
|------|------|
| FLAG_SECURE default ON | Method channel in A21 |
| Offline flavor no INTERNET | productFlavors in A22 |
| `process_image_bytes_json` | Camera → draft (FFI ready) |
| `process_receipt_path_json` | File / share-sheet path |
