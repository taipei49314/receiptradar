# Flutter ↔ Rust bridge (PR-A19)

## Today (no Flutter SDK / no FRB codegen)

- Rust contract: `crates/rradar-ffi` free functions (see `docs/ffi.md`).
- Dart facade: `lib/services/rradar_api.dart`
  - `MockRradarApi` — in-memory / hardcoded for UI shells
  - `NativeRradarApi` — stub that throws until FRB is generated

## When Flutter + FRB are available

1. Add `flutter_rust_bridge` dependency.
2. Generate bindings from `rradar_ffi` into `lib/bridge/generated/`.
3. Implement `NativeRradarApi` by calling generated methods.
4. Swap default in `main.dart` / DI to native when `kIsWeb == false` and lib loads.
5. Android: link `librradar_ffi.so` (cdylib) or static lib via CMake / cargo-ndk.

## Privacy

- No network in the Rust core path.
- Multi-device = user copies `backup.rradar` only.
