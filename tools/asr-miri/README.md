# ASR Miri harness

This crate exists only for V1R-ASR-004 safety validation. It compiles the production Moonshine `ffi.rs`, `manifest.rs`, and `runtime.rs` modules directly by path, with `moonshine_native_linked` intentionally unset. The existing fake-ABI unit tests therefore exercise the real Rust RAII transcriber/stream ownership, drop ordering, typed error mapping, transcript copying, and fail-closed unlinked-runtime behavior under Miri.

The harness does **not** claim that Miri executes Moonshine or ONNX Runtime. Native dylib calls and the raw-pointer transcript-copy boundary are outside Miri's practical scope and are instead covered by the supported-macOS native acceptance workflow, which builds the pinned runtime and runs real Tiny/Small streaming transcription.

Run locally with a nightly toolchain that includes Miri:

```bash
rustup toolchain install nightly --profile minimal --component miri
cargo +nightly miri setup
cargo +nightly miri test --manifest-path tools/asr-miri/Cargo.toml
```
