# ReceiptRadar mobile (Flutter)

**PR-A18 shell** — privacy onboarding + home + capture placeholder.  
Rust FFI (`flutter_rust_bridge`) is **PR-A19**; camera confirm loop is **A20**.

## Requirements

- Flutter 3.22+ / Dart 3.3+
- Android minSdk **26**, **arm64-v8a** only (see design KD-15)

## Run (when Flutter is installed)

```bash
cd apps/mobile
flutter pub get
flutter run
```

Until Flutter is on PATH, this tree is source-only and CI does not build APK.

## Platform notes

| Item | Plan |
|------|------|
| FLAG_SECURE default ON | Method channel in A21 |
| Offline flavor no INTERNET | productFlavors in A22 |
| `process_receipt_path` | FRB in A19 |
