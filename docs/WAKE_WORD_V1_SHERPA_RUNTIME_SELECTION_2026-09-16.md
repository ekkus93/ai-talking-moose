# Wake Word V1 sherpa native runtime selection

**Recorded:** 2026-09-16
**TODO:** `WW-100` / `WW-110` in `docs/WAKE_WORD_V1_TODO_2026-09-14.md`
**Status:** Runtime release selected; per-platform byte identities and real acceptance remain open

## Frozen release

Wake Word V1 selects **sherpa-onnx v1.13.8** as the native runtime release for the first Linux x86_64 and macOS arm64 acceptance work.

Production preparation must resolve artifacts from the immutable `v1.13.8` release identity. It must not resolve `latest`, an unversioned branch, or another mutable alias. A later sherpa release is a deliberate dependency update requiring new immutable identities and requalification; it is not an automatic substitution.

The production KWS path is CPU-only and uses sherpa's native C API from Rust/Tauri. Wake Word V1 does not require a Python sidecar and does not select a CUDA runtime.

## Supported V1 acceptance architectures

The initial claimed runtime targets remain deliberately narrow:

- Linux x86_64;
- macOS arm64 (Apple Silicon).

Additional architectures are not production-supported merely because upstream publishes binaries for them. They require their own immutable runtime identity and real acceptance before the support claim expands.

The repository's deterministic runtime-identity freezer is authoritative for architecture validation before an artifact can be admitted. Linux input must validate as ELF x86_64 and macOS input as Mach-O arm64. A wrong-architecture input fails closed rather than reaching inference or packaging.

## Packaging policy

Wake Word V1 uses a platform-specific shared native runtime layout. Preparation and package/install paths must be deterministic, and CI caches are only transport/performance aids: cache presence never substitutes for SHA-256 verification.

The runtime release selection does **not** authorize any archive by filename alone. Before production use, each selected platform archive and every consumed native library must have independently derived byte size and SHA-256 evidence recorded in the fail-closed artifact manifest. The deterministic offline runtime-identity freezer added under WW-100/WW-110 exists specifically to produce that evidence from independently obtained archives.

## Upstream provenance

Authoritative sherpa documentation currently uses release `v1.13.8` in its precompiled-runtime installation examples and documents CPU support across Linux x64 and macOS arm64. The project runtime is Apache-2.0; repository-local third-party notices and any notices required by bundled runtime dependencies remain a separate closure requirement.

Upstream documentation references:

- `https://k2-fsa.github.io/sherpa/onnx/install/linux.html`
- `https://k2-fsa.github.io/sherpa/onnx/python/install.html`
- `https://k2-fsa.github.io/sherpa/onnx/kws/index.html`

## Remaining WW-100 / WW-110 closure

This document closes the runtime **version-selection** ambiguity only. It does not claim full runtime packaging acceptance. Remaining mandatory evidence includes:

1. exact Linux x86_64 selected archive and consumed-library byte sizes/SHA-256 values;
2. exact macOS arm64 selected archive and consumed-library byte sizes/SHA-256 values;
3. repository-local third-party notices/license attribution for the shipped runtime closure;
4. deterministic offline package preparation using only verified inputs;
5. real Linux x86_64 load/KWS acceptance;
6. real macOS arm64 load/KWS acceptance;
7. exact-master requalification of the final runtime/package boundary.

Until those items are complete, runtime loading remains fail-closed and Wake Word V1 must not silently fall back to another sherpa version, full-time ASR, Python, or a network service.
