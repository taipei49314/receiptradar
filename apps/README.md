# Apps

- `mobile/` — Flutter Android client (Track A)
  - UI shell: onboarding, home, capture placeholder
  - Core access: `lib/services/rradar_api.dart` → `crates/rradar-ffi` (FRB later)
  - Docs: [docs/ffi.md](../docs/ffi.md), [mobile/README.md](./mobile/README.md)
- Desktop / WASM demos are **Track B**, not v0.1

```bash
cargo test -p rradar-ffi
# optional when Flutter installed:
# cd mobile && flutter pub get && flutter run
```
