# Flutter ↔ Rust bridge (PR-A19)

## Today (no Flutter SDK / no FRB codegen)

- Rust contract: `crates/rradar-ffi` free functions (see `docs/ffi.md`).
- Dart facade: `lib/services/rradar_api.dart`
  - `MockRradarApi` — in-memory list + capabilities (schema 3, handoff, rules)
  - `NativeRradarApi` — stub that throws until FRB is generated
- Screens already call the facade: Home, Ledger, About, Capture placeholder.

## When Flutter + FRB are available

1. Add `flutter_rust_bridge` dependency.
2. Generate bindings from `rradar_ffi` into `lib/bridge/generated/`.
3. Implement `NativeRradarApi` by calling generated methods (handoff/rules/models included).
4. Swap `rradarApi = NativeRradarApi()` when native lib loads.
5. Android: `cargo ndk` → `librradar_ffi.so` (see `docs/android-ffi.md`).

## Privacy

- No network in the Rust core path.
- Multi-device = backup / handoff files only (no official relay).
