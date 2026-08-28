# ASR Miri harness

This crate exists only for V1R-ASR-004 safety validation. It compiles the production Moonshine `ffi.rs`, `manifest.rs`, and `runtime.rs` modules directly by path, with `moonshine_native_linked` intentionally unset.

The fake-ABI tests therefore exercise the real Rust RAII transcriber/stream ownership, drop ordering, typed error mapping, transcript ownership, and fail-closed unlinked-runtime behavior under Miri. The production FFI module also exposes its C-layout transcript structs and `copy_transcript` helper to unit tests, so synthetic Rust-owned C-layout fixtures directly exercise the audited unsafe transcript-copy boundary under Miri: transcript pointer dereference, contiguous-line slice construction, NUL-terminated text reads, metadata copying, null-pointer rejection, and conversion to Rust-owned values. The same 64-bit ABI size/alignment/offset assertions used by native-linked builds are active in the Miri test build.

The harness does **not** claim that Miri executes Moonshine or ONNX Runtime dylib calls. Those external native calls remain outside Miri's practical scope and are covered by the supported-macOS native acceptance workflow, which builds the pinned runtime and runs real Tiny/Small streaming transcription.

The CI workflow pins the known-good nightly used by the accepted V1R-ASR-004 Miri evidence rather than floating on the latest nightly. Run locally with:

```bash
rustup toolchain install nightly-2026-08-22 --profile minimal --component miri
cargo +nightly-2026-08-22 miri setup
cargo +nightly-2026-08-22 miri test --manifest-path tools/asr-miri/Cargo.toml
```
