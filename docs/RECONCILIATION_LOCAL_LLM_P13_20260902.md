# Local LLM P13 Packaging/CI Reconciliation — 2026-09-02

## Scope and source

P13 covers packaging and CI requirements for the Local LLM V1 runtime:

- LLM-130: supported-target native runtime compilation;
- LLM-131: ordinary CI remains model-weight-free;
- LLM-132: shipped runtime dependency/license evidence;
- LLM-133: measured application bundle-size impact and proof that GGUF weights are external.

This tranche starts from `master` `2df8f43d39b43876a0b227840e4685c250fdf553`.
The canonical pre-P13 master CI run is GitHub Actions run `33672240839`.

**P13 is not complete yet.** The code/configuration hardening can merge after ordinary CI passes, but LLM-133 remains open until the separately dispatched bundle comparison runs on both supported macOS architectures and its artifacts are reviewed.

## LLM-130 — Supported-target native compilation

The existing ordinary CI compile-proof matrix is already real evidence rather than a TODO-only claim. Run `33672240839` passed all three jobs at exact master `2df8f43d39b43876a0b227840e4685c250fdf553`:

- `Local LLM compile proof (linux-x86_64)`;
- `Local LLM compile proof (macos-arm64)`;
- `Local LLM compile proof (macos-x86_64)`.

The normal Rust/macOS application jobs also build the application's direct `llama-cpp-2` / `llama-cpp-sys-2` dependency. The compile proof and application both pin `0.1.154` with default features disabled. Hosted CI installs compiler/CMake/Clang prerequisites but does **not** install a system llama.cpp package, so a developer-local llama.cpp installation is not part of the build contract.

This tranche adds `scripts/check_local_llm_packaging_policy.py` so the target matrix, exact dependency pins, CPU/default-feature policy, and absence of a system-llama install step become fail-closed configuration invariants.

## LLM-131 — Ordinary CI remains model-weight-free

Real GGUF execution remains confined to `.github/workflows/local-llm-real-cpu-acceptance.yml`, which is manually dispatched. Ordinary `CI`, release source gates, and `npm run check:all` do not invoke the real-model acceptance binary or reference GGUF artifacts/model-host URLs.

Unit coverage uses explicit injected test runtimes (`FakeEngine`, `CountingEngine`, and `LocalGenerationRuntime` injection) rather than a production fallback. The generated frontend contract serializes settings/catalog metadata; it does not instantiate the native engine or installer download path.

This tranche adds a static fail-closed gate that verifies those boundaries and also requires Git to ignore `*.gguf` / `*.GGUF`. Tauri bundle resources remain notice-only. Bundle verification now rejects any embedded GGUF file even if future configuration changes bypass the static check.

LLM-131 should be marked complete only after the implementation PR's ordinary CI passes.

## LLM-132 — Dependency and license inventory

The application statically links llama.cpp/ggml through:

- `llama-cpp-2 = 0.1.154` — `MIT OR Apache-2.0`;
- `llama-cpp-sys-2 = 0.1.154` — `MIT OR Apache-2.0`.

The upstream binding release is `utilityai/llama-cpp-rs` tag `0.1.154`, commit `bed81ad4ab1a6c904b11d425608e50f976d8ea62`. Its `llama.cpp` submodule is pinned to `5f55650a78f92aff4d48d671423e888fac0469ff`. The native llama.cpp/ggml source at that pin is MIT licensed.

P13 hardening adds:

- `src-tauri/native/macos/notices/LocalLlmRuntime/LLAMA_CPP_LICENSE` with the pinned native MIT text;
- `LocalLlmRuntime/LLAMA_CPP_RS_LICENSE_MIT` with the upstream binding MIT text selected from the crates' `MIT OR Apache-2.0` dual-license option;
- `LocalLlmRuntime/README.md` with exact binding/native provenance;
- explicit collector assertions that both llama binding crates and the expected versions are present with license evidence;
- macOS smoke bundles now run the dependency collector before Tauri packaging;
- `verify_macos_bundle.sh` requires the native llama.cpp notice, both binding rows in `DEPENDENCY_LICENSES.md`, and no unresolved dependency evidence;
- `docs/LOCAL_LLM_MODEL_LICENSES.md` records model licenses separately because GGUF weights are downloads rather than shipped code dependencies;
- `docs/THIRD_PARTY_NOTICES.md` records the shipped Local LLM runtime provenance and keeps model-license documentation separate.

LLM-132 should be marked complete only after ordinary CI proves the collector and both macOS smoke bundles pass with these stronger requirements.

## LLM-133 — Bundle-size impact

The pre-native-Local-LLM baseline is fixed to:

`3253e52f7331ed7b03f0a3a4443eeef6d8e45aac`

That commit is the exact parent of `448df93af1f5c924dd3b8bc9160a55bdef9ac1cf`, which first added `llama-cpp-2` / `llama-cpp-sys-2` to the application.

`.github/workflows/local-llm-p13-packaging-acceptance.yml` is a manual-only two-architecture acceptance workflow. For arm64 and x86_64 it:

1. validates the fixed baseline ancestry and introduction boundary;
2. builds an isolated baseline `.app` from the pre-llama commit;
3. builds the requested current `.app` with the same architecture/deployment floor;
4. verifies the current self-contained bundle and license inventory;
5. sums regular-file logical byte sizes for both `.app` bundles;
6. records current executable size, byte/percentage delta, and file counts;
7. fails if either comparison bundle contains a `.gguf` file;
8. uploads one machine-readable JSON report per architecture.

The workflow is intentionally **not** part of ordinary CI because it performs two full macOS application builds per architecture. It still downloads no LLM model weights.

P13/LLM-133 stays open until this workflow is merged to `master`, manually dispatched on the exact intended master SHA, and both JSON artifacts are reviewed and recorded here.

## Local validation available in the sandbox

The fresh master snapshot had no Rust toolchain or installed frontend dependencies. The implementation was therefore locally checked with dependency-free gates:

- `python3 scripts/check_local_llm_packaging_policy.py`;
- `python3 -m py_compile` for the new/changed Python helpers;
- `bash -n scripts/verify_macos_bundle.sh`;
- a synthetic positive/negative probe of the required llama dependency inventory assertion;
- a synthetic bundle-size measurement probe;
- YAML parse of the new manual workflow;
- `git diff --check`.

Ordinary GitHub Actions remains authoritative for Rust, frontend, real dependency collection, and macOS packaging.
