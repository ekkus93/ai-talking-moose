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

## P5 runtime implementation

P5 is implemented on branch `ralph/local-llm-p5-runtime-20260831`, based on merged LLM-003 master `448df93af1f5c924dd3b8bc9160a55bdef9ac1cf`.

### Ownership and native-type boundary

`src-tauri/src/ai/local/runtime/` is split into focused `types`, `manager`, `llama`, and test modules. `AppState` owns only `Arc<LocalRuntimeManager>`; llama.cpp backend/model/context/sampler types never cross into `AppState`, Tauri commands, or frontend-facing structures.

The manager owns one lazily initialized `LlamaBackend` and at most one loaded `LlamaModel`. `LlamaContext` and `LlamaSampler` are created and destroyed entirely inside one blocking generation scope because `llama-cpp-2 0.1.154` makes those objects thread-affine (`!Send`/`!Sync`), while the cached model/backend are safe to keep behind the runtime owner.

### Load, switch, delete, and synchronization semantics

Every generation, load/switch, delete, and shutdown transition uses one Tokio operation mutex. Generation is serialized. Model deletion routes through the runtime manager so a loaded model is unloaded before the installer removes its artifact. The runtime never silently substitutes another local model, Google, or Fake.

Loading the already-loaded model identity is idempotent. Switching models unloads the old model before attempting the replacement. If replacement loading fails, the runtime remains unloaded rather than restoring the previous model. An initialized backend with no loaded model is not treated as a loaded state and therefore does not perform a spurious unload before the first real load.

Installed-model resolution is fail-closed: catalog membership is required, installer state must be `Installed`, the artifact is canonicalized under the canonical installer root, metadata must describe a regular file, and the file byte count must match the pinned catalog entry before llama.cpp is asked to open it.

### CPU and request bounds

Model loading explicitly uses `with_n_gpu_layers(0)`. Context construction explicitly disables KQV and operation offload. The default context target is 4096 tokens, capped by catalog context limits. The default thread policy uses half of available parallelism clamped to 1–8 threads.

Requests reject empty prompts, prompts larger than 64 KiB, zero output-token limits, non-finite temperatures, and temperatures outside 0.0–2.0. Output tokens are additionally capped by the catalog recommendation, and tokenized prompt plus requested output must fit within the selected bounded context.

### Cancellation and bounded shutdown limitation

Cancellation is cooperative. The runtime checks a `CancellationToken` before model/context work, between prompt-decode chunks, and between generated-token decode iterations.

The underlying llama.cpp API has an abort callback, but the safe high-level `LlamaContextParams` surface in `llama-cpp-2 0.1.154` does not expose an abort-callback setter. P5 therefore does **not** reach into unsafe raw FFI solely to force interruption. If a native `decode` call is already executing, cancellation cannot interrupt that individual call and is observed after it returns.

Application exit is nevertheless bounded: `ExitRequested` immediately calls `begin_shutdown()` so no new local work is admitted, then async teardown gives `LocalRuntimeManager::shutdown()` at most five seconds. If an in-progress native decode prevents timely unload, the application logs a safe timeout without prompt/output/path data and continues process exit. The OS then reclaims remaining native state. This is the explicit V1 fail-safe for a native call that cannot be cooperatively interrupted.

### Diagnostics and privacy

`LocalRuntimeDiagnostics` records selected model ID, loaded model ID/revision/quantization, loaded state, thread count, context size, generation-in-progress state, safe error category, generation duration, prompt/output token counts, and tokens-per-second. It deliberately contains no prompt or generated text fields. llama.cpp native logging is disabled at backend initialization because native diagnostics may include model filesystem paths; application-visible failures use stable runtime error categories/messages instead.

P5 owns the safe internal runtime telemetry. Extending generated frontend IPC shapes for that telemetry remains P10 work; the existing installer diagnostics command is not silently redefined during P5.

### Validation evidence

CI run `33443626815` passed on P5 head `0331373db1ec998cd6e20ef0599c30f33ed0819a`, including Rust format, Clippy with warnings denied, the complete Rust test suite, frontend/generated-contract gates, dependency audit, local-LLM compile proof, and supported bundle jobs. The bounded-shutdown hardening was added after that green point and therefore requires a fresh normal CI run before PR #18 is mergeable.

## Staged implementation rule

The schema/fail-closed tranche may land before llama.cpp runtime support as long as:

- existing users migrate to Google text generation;
- new profiles continue using Google until the local installer/runtime/UX is usable;
- a selected-but-unavailable Local provider fails explicitly with a stable provider error;
- Local never falls back to Google or Fake;
- Google without a credential never falls back to Fake;
- no Local option is exposed as a working production choice until model lifecycle/runtime support exists.

The new-profile default can switch to Local later in this TODO once the required model install can be explained and completed from the UI. This staging prevents the schema refactor from breaking fresh installs before the runtime exists.
