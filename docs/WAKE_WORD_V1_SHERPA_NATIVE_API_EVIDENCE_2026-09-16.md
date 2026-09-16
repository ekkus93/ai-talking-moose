# Wake Word V1 sherpa native API evidence

**Recorded:** 2026-09-16
**TODO:** `WW-100` in `docs/WAKE_WORD_V1_TODO_2026-09-14.md`
**Status:** Native API path confirmed; runtime artifact pinning remains open

## Decision

Wake Word V1 will integrate sherpa-onnx through its native C API from the Rust/Tauri process. It will not introduce a Python sidecar.

The authoritative sherpa-onnx C API documentation exposes keyword spotting through `SherpaOnnxCreateKeywordSpotter()` and `SherpaOnnxKeywordSpotterConfig`. The documented configuration accepts transducer encoder/decoder/joiner paths, a token file, CPU provider selection, `num_threads`, and a keyword file. This is the narrow native boundary required by the V1 architecture: Rust can bind to the C ABI while model execution remains in the pinned sherpa native runtime.

For the frozen V1 policy, the eventual binding must configure the CPU provider and one inference thread, load only the exact verified model/tokenizer/runtime artifacts, and keep normal keyword inference offline. No Python interpreter, Python package environment, subprocess protocol, or Python-side model loader is part of the production design.

## Upstream references

- sherpa-onnx keyword spotting C API: `https://k2-fsa.github.io/sherpa/onnx/c-api/html/keyword_spotting.html`
- `SherpaOnnxKeywordSpotterConfig`: `https://k2-fsa.github.io/sherpa/onnx/c-api/html/structSherpaOnnxKeywordSpotterConfig.html`
- KWS overview and pretrained models: `https://k2-fsa.github.io/sherpa/onnx/kws/index.html`

## Boundary of this evidence

This evidence closes only the WW-100 question of whether a native, non-Python API path exists. It does **not** qualify a sherpa-onnx runtime release or shared library. WW-100/WW-110 still require an immutable runtime version, exact per-platform byte sizes and SHA-256 identities, architecture verification, deterministic preparation, license/notices review, and real Linux x86_64/macOS arm64 acceptance before production loading is enabled.

The existing fail-closed model manifest remains authoritative: no unverified model or runtime bytes become production-eligible because this API path has been confirmed.
