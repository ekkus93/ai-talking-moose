# Local LLM implementation notes

Companion planning documents:

- `docs/SPEC(20260831-081800).md`
- `docs/TODO(20260831-081800).md`

## Baseline

The Local LLM Ralph loop starts from authoritative `master` commit:

`b7387acfeaa4328759d227b2826a23db6d1cfd86`

The implementation branch for the first tranche is:

`ralph/local-llm-p0-p2-20260831`

## Local LLM V1 boundary

Local LLM V1 changes **text generation only**.

```text
Local Text V1

Ambient event ─┐
               ├─> TextModel ─> Local or Google text generation ─> existing TTS/speech path
Typed message ─┘

Voice conversation remains:

Microphone ─> Moonshine local ASR or Gemini Live audio ─> Gemini Live session ─> Gemini audio
```

The future fully-local voice path is deliberately deferred:

```text
Deferred fully-local voice

Microphone ─> Moonshine ASR ─> Local LLM ─> local/provider-neutral TTS ─> speakers
```

`LocalTextModel` therefore implements only the existing `TextModel` trait. It does not implement `RealtimeConversationProvider`, and this phase does not change Moonshine ASR ownership or Gemini Live session behavior.

## Current provider-setting audit

### Rust reads and writes

The generic persisted `AppSettings.provider` field is currently consumed only by the application-state/provider factory and settings validation/restart policy:

- `src-tauri/src/app/state.rs`
  - `AppSettings.provider`
  - `AppSettings::default()` sets `"google"`
  - `get_text_model()` treats `provider == "fake"` **or a missing Google API key** as permission to construct `FakeTextModel`
  - `get_speech_synthesizer()` treats `provider == "fake"` **or a missing Google API key** as permission to construct `FakeSpeechSynthesizer`
  - `get_live_provider()` constructs `FakeConversationProvider` only when `provider == "fake"`; otherwise it constructs Google Live and lets missing credentials fail through Google authentication
- `src-tauri/src/app/settings_policy.rs`
  - validation accepts persisted ordinary values `"google" | "fake"`
  - conversation restart comparison includes the generic provider field
  - settings-consumer inventory describes the field as provider selection

Fake conversation providers used by lower-level conversation/session tests are instantiated explicitly and do not require the persisted application setting.

### TypeScript and generated contract

- `src/types/moose.ts` exposes `provider: "google" | "fake"` and the ambiguous `text_model` string.
- `src/generated/backendContract.json` serializes both fields.
- `src-tauri/src/bin/export_frontend_contract.rs` derives the representative `AppSettings` shape from Rust.
- Browser preview and production-like frontend test fixtures obtain their settings from the generated backend contract rather than maintaining an independent provider value.
- `src/components/Settings/AiTab.tsx` currently presents Gemini-specific model/API-key controls but does not expose a generic provider selector.

### Fake-provider classification

- `src-tauri/src/ai/fake.rs`: test/development implementation module; useful as an explicit test double.
- Conversation/session/integration tests: direct Fake provider construction is test-only and should remain supported.
- Browser preview: explicit Vite-development adapter; simulated responses remain development-only.
- Persisted `AppSettings.provider = "fake"`: **production-visible and unsafe** because settings deserialization can make native production select Fake behavior.
- Google text with no API key: **unsafe manufactured success** because `get_text_model()` silently returns `FakeTextModel`.
- Google TTS with no API key: **unsafe manufactured success** because `get_speech_synthesizer()` silently returns `FakeSpeechSynthesizer`.

The staged refactor removes persisted Fake selection from the production schema. Fake implementations remain available only through explicit test/development construction.

## llama.cpp integration decision

### Evaluated approaches

1. **Bundled llama.cpp subprocess/sidecar**
   - Pros: strong process isolation; easy to inspect/cancel at OS-process granularity.
   - Cons: introduces sidecar packaging/signing/provenance on both macOS architectures; requires a stable private IPC protocol; duplicates lifecycle/supervision complexity; makes bundle verification materially more complicated.

2. **High-level older `llama_cpp` Rust crate**
   - Pros: convenient high-level API and off-thread completion handle.
   - Cons: current published release `0.3.2` is from 2024 and pins an older llama.cpp generation. That is a poor fit for a model catalog intended to support current GGUF families such as Qwen3.

3. **Maintained `llama-cpp-2` Rust bindings** — **selected**
   - Current evaluated release: `0.1.154` (published 2026-08-05).
   - Provides safe wrappers around the current llama.cpp API while staying close enough to upstream to track its fast-moving ABI/API.
   - Downstream `default-features = false` avoids the crate's default OpenMP/common feature set and does not enable CUDA, Vulkan, ROCm, OpenCL, or MKL.
   - **Apple Silicon caveat:** the crate itself has a target-specific dependency that enables its Metal sys feature on macOS arm64 even when downstream default features are disabled. V1 therefore guarantees **zero GPU offload at runtime** (`n_gpu_layers = 0` / equivalent model params), not that Metal support is absent from the Apple Silicon binary.
   - The underlying llama.cpp project supports CPU inference and both Apple arm64 and x86_64; this repository already has CI runners for both macOS architectures.
   - Keeps inference in-process, so the application does not require a separately installed `llama-cli`/`llama-server` executable.

References used for the decision:

- https://docs.rs/llama-cpp-2/0.1.154/llama_cpp_2/
- https://docs.rs/crate/llama-cpp-2/0.1.154/source/Cargo.toml.orig
- https://docs.rs/crate/llama-cpp-sys-2/0.1.154/source/Cargo.toml.orig
- https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md

### Build/package implications

The native binding pulls a C/C++ llama.cpp build into the Rust dependency graph. CI/package work must therefore verify:

- Linux Rust-quality runners have the needed C/C++/CMake/libclang toolchain;
- macOS arm64 and x86_64 bundle jobs compile the binding without relying on a developer-installed llama.cpp;
- Apple Silicon may contain compiled Metal support from the binding's target policy, but Local LLM V1 acceptance must prove model parameters request zero GPU layers/offload;
- no GGUF model weight is embedded in ordinary application bundles;
- llama.cpp/binding license evidence is included in the shipped dependency inventory;
- model licenses are tracked separately because model weights are user-installed artifacts, not ordinary compiled dependencies.

### Synchronization and cancellation policy

Until the selected binding is proven safe for concurrent contexts in this application, Local LLM V1 will use a single runtime manager with serialized generation. Model load/unload/delete/switch operations share the same ownership boundary so an artifact cannot be removed while inference is using it.

Inference is CPU-only in V1: model/context configuration must request zero GPU layers and no GPU offload. CUDA, Vulkan, ROCm, OpenCL, and MKL features are not enabled by this project. On Apple Silicon, Metal code may still be compiled by the binding's target-specific dependency, but it is not selected for model offload in V1. Request output/context sizes are bounded independently of model metadata. Runtime calls that cannot be cooperatively interrupted will be isolated from async executor threads, and application shutdown must have a bounded/non-hanging policy before real-model acceptance closes.

## Staged implementation rule

The schema/fail-closed tranche may land before llama.cpp runtime support as long as:

- existing users migrate to Google text generation;
- new profiles continue using Google until the local installer/runtime/UX is usable;
- a selected-but-unavailable Local provider fails explicitly with a stable provider error;
- Local never falls back to Google or Fake;
- Google without a credential never falls back to Fake;
- no Local option is exposed as a working production choice until model lifecycle/runtime support exists.

The new-profile default can switch to Local later in this TODO once the required model install can be explained and completed from the UI. This staging prevents the schema refactor from breaking fresh installs before the runtime exists.
