# Android FFI link notes (no SDK required to author)

v0.1 mobile = **Android arm64-v8a only** (KD-15). This doc is the build contract for when NDK + Flutter are available.

## Outputs

| Artifact | Source |
|----------|--------|
| `librradar_ffi.so` | `cargo ndk -t arm64-v8a build -p rradar-ffi --release` |
| Dart bindings | flutter_rust_bridge codegen → `apps/mobile/lib/bridge/generated/` |

## Suggested toolchain

```text
rustup target add aarch64-linux-android
cargo install cargo-ndk
# ANDROID_NDK_HOME pointed at installed NDK
```

```bash
cd receiptradar
cargo ndk -t arm64-v8a -o apps/mobile/android/app/src/main/jniLibs \
  build -p rradar-ffi --release
```

## Constraints

- **Do not** enable `--features onnx` on-device until A04 size gate is Green.
- Core path must remain offline-capable (no required INTERNET for capture → ledger).
- Multi-device: handoff/backup files via SAF / share sheet — never an official relay.

## Flutter shell today

Without FRB, `MockRradarApi` exercises Home / Ledger / About navigation so UI can ship independently of NDK.
