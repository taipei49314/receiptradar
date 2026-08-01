# ReceiptRadar mobile (Flutter)

**PR-A18 shell** — privacy onboarding, home, ledger list, about/capabilities, capture placeholder.  
**PR-A19** Rust contract: `crates/rradar-ffi` + `lib/services/rradar_api.dart` (FRB codegen pending).

## Requirements

- Flutter 3.22+ / Dart 3.3+ (optional for Rust-only CI)
- Android minSdk **26**, **arm64-v8a** only (design KD-15)

## Architecture (local-first)

```
UI (Dart)  →  RradarApi facade  →  [Mock now | FRB later]  →  rradar-ffi  →  rradar-core
```

- No official cloud sync / relay.
- Multi-device: encrypted backup / **handoff** files only.
- Schema **v3**: tags + attachment_path via FFI; **capture one-shot** `process_confirm_*`.
- **Tag filter** ledger UI + **Budgets** screen (mock API); FRB map: [docs/frb-contract.md](../../docs/frb-contract.md).

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
| `lib/services/rradar_api.dart` | API facade + mock (schema 3, handoff caps) |
| `lib/screens/ledger_screen.dart` | Transaction list |
| `lib/screens/about_screen.dart` | Capabilities / paths |
| `lib/bridge/README.md` | FRB wiring notes |
| `../../docs/ffi.md` | Full FFI function map |
| `../../docs/android-ffi.md` | NDK / cargo-ndk notes |
| `../../crates/rradar-ffi` | Rust free functions |

## Platform notes

| Item | Plan |
|------|------|
| FLAG_SECURE default ON | Method channel in A21 |
| Offline flavor no INTERNET | productFlavors in A22 |
| `process_image_bytes_json` | Camera → draft (FFI ready) |
| `handoff_*` | Multi-device file package (FFI ready) |
