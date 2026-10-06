# Local ASR / Whisper.cpp Handoff — 2026-10-03

This file captures the current state of the Talking Moose local voice project as of the commit being pushed to `master`. It is intended for another developer or a fresh clone to pick up the work without context loss.

## 1. Current Objective

Build a private, local Linux ASR path using `ggml-org/whisper.cpp` with the `whisper-small` ggml weights, replacing or extending the macOS-only Moonshine path after the wake-word handoff.

The project currently has:

- A working sherpa-onnx wake-word KWS path on Linux.
- A local ASR pipeline that currently supports Moonshine Tiny/Small streaming on macOS.
- A cloud-based `GeminiLiveAudio` path, which is not private and is not a suitable Linux local ASR replacement.
- A Tauri desktop shell with a newly added close-window control.
- A whisper.cpp `WhisperSmall` local ASR backend is implemented for final transcription; streaming partials, weight-license verification, and real-CPU acceptance remain open.

The goal is to add a whisper.cpp-backed local ASR engine that can be selected as a Linux local ASR mode, while preserving the existing Moonshine implementation for macOS.

---

## 2. Important Project Constraints

### 2.1 Local ASR must be private on Linux

The local ASR path should not require a cloud API key. It should use local model files and local CPU inference.

Do not fall back silently to Google, Fake, or cloud providers when a local ASR backend fails.

### 2.2 Do not commit model weights

The repository must not contain `.gguf`, `.GGUF`, whisper `.bin` weights, Moonshine `.ort` files, sherpa model weights, or other large model binaries.

Model files must be downloaded, verified, and stored by the app at runtime or through an installer pattern.

### 2.3 Existing packaging policy

There is an existing static packaging policy:

```bash
python3 scripts/check_local_llm_packaging_policy.py
```

That script currently targets local LLM / llama.cpp packaging. It is unclear whether it needs to be extended for whisper.cpp or whether a separate ASR packaging policy is required.

Do not weaken the existing local LLM packaging gate to make whisper.cpp work.

### 2.4 ASR benchmark reporting

There is an existing ASR benchmark report script:

```bash
python3 scripts/render_asr015_benchmark_report.py
```

It expects whisper.cpp-style records, including:

- `tiny_streaming`
- `small_streaming`
- source provenance such as:
  - `source_commit`
  - `source_git_blob_sha1`
  - `source_sha256`
  - `corpus_sha256`

If whisper.cpp local ASR is added, align the local benchmark/reporting surface with this script where possible.

---

## 3. Repository State at Handoff Time

### 3.1 Git state before this handoff commit

Branch:

```bash
master
```

Remote:

```bash
origin git@github.com:ekkus93/ai-talking-moose.git
```

Uncommitted changes at the start of this handoff were:

#### Modified files

- `scripts/prepare-wake-word-artifacts.py`
- `src-tauri/capabilities/default.json`
- `src-tauri/gen/schemas/capabilities.json`
- `src-tauri/src/bin/wake_word_acceptance.rs`
- `src/windows/MooseWindow.tsx`

#### Untracked file

- `tests/__init__.py`

This handoff file is added by this commit.

---

## 4. Completed Work

### 4.1 Close-window behavior

The upper-left Square button in the desktop shell now closes the window.

Relevant files:

- `/home/phil/work/ai-talking-moose/src/windows/MooseWindow.tsx`
- `/home/phil/work/ai-talking-moose/src-tauri/capabilities/default.json`
- `/home/phil/work/ai-talking-moose/src-tauri/gen/schemas/capabilities.json`

Changes:

- `handleCloseWindow()` was added in `src/windows/MooseWindow.tsx`.
- The upper-left Square button now calls:
  ```tsx
  onClick={() => void handleCloseWindow()}
  ```
- The button now has:
  ```tsx
  title="Close Talking Moose"
  aria-label="Close Talking Moose"
  ```
- `src-tauri/capabilities/default.json` now includes:
  ```json
  "core:window:allow-close"
  ```
- `src-tauri/gen/schemas/capabilities.json` was regenerated to include the same capability.

This is intended to preserve the completed close-window / quit behavior.

---

### 4.2 Wake-word artifact script change

`scripts/prepare-wake-word-artifacts.py` was changed:

```diff
-            bundle.extractall(destination, members=members, filter="data")
+            bundle.extractall(destination, members=members)
```

This removes the `filter="data"` argument from the tar extraction path.

Review note:

- This may be intentional if the existing `safe_destination` validation is sufficient for the shipped artifact layout.
- It should be re-reviewed before relying on it for untrusted archives, because `filter="data"` provides extra path-traversal protection in Python's `tarfile` extraction.
- The existing script appears to sanitize member names through `safe_destination`, but the removal of `filter="data"` is worth confirming.

---

### 4.3 Wake-word acceptance binary refactor

`src-tauri/src/bin/wake_word_acceptance.rs` was refactored.

Key changes:

- `--output` parsing now happens before the `--production-listener` branch.
- The production-listener branch now returns early after the production report check:
  ```rust
  return Ok(());
  ```
- The non-production branch writes the real KWS acceptance report to `--output`.

Behavior note:

- `--production-listener` no longer writes the report JSON to `--output`.
- The normal real KWS corpus path still writes to `--output`.

This appears intentional to separate production-listener acceptance from corpus report writing, but it should be confirmed against the intended CI/acceptance usage.

---

### 4.4 `tests/__init__.py`

An empty `tests/__init__.py` was added.

This makes `tests/` a Python package if the project uses `tests/` with Python test discovery.

Review note:

- If Python tests are not intended to be discovered through `tests/__init__.py`, this file can be removed.
- It is currently harmless but may be unnecessary.

---

### 4.5 Previous session verification

The previous session recorded that these gates were green after the close-window and wake-word cleanup work:

```bash
npm run check:frontend
npm run check:rust
npm run check:all
```

The exact changed files in this handoff should still be re-verified on the new machine before relying on them.

---

## 5. Current Local ASR Architecture

### 5.1 Current modes

The local ASR mode enum is in:

```text
/home/phil/work/ai-talking-moose/src-tauri/src/asr/types.rs
```

Current `AsrMode` values:

- `MoonshineTinyStreaming`
- `MoonshineSmallStreaming`
- `WhisperSmall`
- `GeminiLiveAudio`

The Whisper mode is implemented as `AsrMode::WhisperSmall` with serialized form `"whisper_small"`.

---

### 5.2 Current pipeline coupling

Main pipeline file:

```text
/home/phil/work/ai-talking-moose/src-tauri/src/asr/pipeline.rs
```

Important current traits/types:

- `LocalAsrPipelineDiagnostics`
- `LocalAsrPipelineEventCallback`
- `PipelineEngine`
- `impl PipelineEngine for MoonshineTinyEngine`

Current `PipelineEngine::push_pcm` signature is Moonshine-specific:

```rust
fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<MoonshineTinyTranscriptUpdate>, AsrError>
```

The worker loop currently:

1. Receives PCM frames.
2. Calls `engine.push_pcm(&pcm)`.
3. Converts `MoonshineTinyTranscriptUpdate` results using `map_transcript_update`.
4. Emits transcript events.

This means the pipeline is not fully engine-agnostic today.

---

### 5.3 Moonshine module

Moonshine module directory:

```text
/home/phil/work/ai-talking-moose/src-tauri/src/asr/moonshine/
```

Files:

- `engine.rs`
- `engine_tests.rs`
- `ffi.rs`
- `installer/`
- `installer.rs`
- `installer_tests.rs`
- `manifest.rs`
- `mod.rs`
- `runtime.rs`

Important Moonshine details:

- Input rate is 16 kHz mono.
- `MOONSHINE_TINY_INPUT_SAMPLE_RATE_HZ = 16_000`
- `MoonshineTinyTranscriptUpdate` has:
  ```rust
  Partial {
      line_id: u64,
      text: String,
      latency_ms: u32,
  }
  Final {
      line_id: u64,
      text: String,
      latency_ms: u32,
  }
  ```
- `MoonshineModelArchitecture` has:
  ```rust
  TinyStreaming
  SmallStreaming
  ```
- `TranscriberInner` owns:
  ```rust
  Arc<dyn MoonshineApi>
  handle: i32
  runtime_version: i32
  ```
- FFI header version constant:
  ```rust
  MOONSHINE_HEADER_VERSION = 30_000
  ```
- Model architecture constants:
  ```rust
  MOONSHINE_MODEL_ARCH_TINY_STREAMING = 2
  MOONSHINE_MODEL_ARCH_SMALL_STREAMING = 4
  ```

Moonshine installer pattern:

- manifest-based download
- SHA256 verification
- CRC32 support
- disk-space probe
- progress callbacks
- marker file:
  ```text
  .talking-moose-model.json
  ```
- marker schema version:
  ```text
  1
  ```
- verify buffer:
  ```text
  1 MiB
  ```
- disk-space headroom:
  ```text
  8 MiB
  ```

This installer pattern is the best local reference for building a whisper.cpp installer.

---

## 6. whisper.cpp Facts Already Established

### 6.1 Source repository

```text
https://github.com/ggml-org/whisper.cpp
```

Default branch:

```text
master
```

License:

```text
MIT
```

Main API surface:

- C API in:
  ```text
  include/whisper.h
  ```

Observed constants:

```c
WHISPER_SAMPLE_RATE 16000
WHISPER_N_FFT 400
WHISPER_HOP_LENGTH 160
WHISPER_CHUNK_SIZE 30
```

Observed batch functions:

```c
whisper_init_from_file_with_params
whisper_full
whisper_full_n_segments
whisper_full_get_segment_text
whisper_free
```

Important architectural fact:

- The C API is batch-oriented.
- No dedicated streaming C API was observed in the reviewed header portion.
- A streaming UX will likely need to be implemented in Rust by managing the audio buffer and calling `whisper_full` periodically.

---

### 6.2 Streaming example

Relevant example:

```text
examples/stream/stream.cpp
```

Observed parameters:

- `step_ms`
- `length_ms`
- `keep_ms`
- `max_tokens`
- `language`
- `model`

Default example model:

```text
models/ggml-base.en.bin
```

Likely Rust streaming strategy:

1. Maintain an in-memory 16 kHz audio buffer.
2. Every `step_ms`, for example around 300 ms, call `whisper_full`.
3. Keep the tail of the buffer to avoid losing context.
4. Track already-emitted segments.
5. Emit only newly emitted segments.
6. Map segments into partial/final transcript updates.

Exact values for:

- `step_ms`
- `length_ms`
- `keep_ms`
- max audio history
- segment overlap

still need to be decided and benchmarked.

---

## 7. whisper-small Model Source

### 7.1 HF repository

```text
https://huggingface.co/ggerganov/whisper.cpp
```

The download script `models/download-ggml-model.sh` downloads the small model as:

```text
models/ggml-small.bin
```

Pinned model URL:

```text
https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.bin?download=true
```

Earlier wrong-path URL:

```text
https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml/small.bin?download=true
```

returned 404; superseded by the pinned URL above and not used.

The file is flat at the repo root, not nested under `ggml/`.

### 7.2 Known metadata

HEAD metadata for `ggml-small.bin`:

- HTTP status: `302` to a CDN
- `x-linked-size`: `487601967` bytes
- `x-repo-commit`: `5359861c739e955e79d9a303bcbc70fb988958b1`

Approximate size:

```text
487,601,967 bytes
about 466 MiB
```

### 7.3 Missing metadata

Still needed:

- authoritative license statement for the `ggml-small.bin` weights
- independent live download verification of bytes and SHA-256

Pinned in `src-tauri/src/asr/whisper/manifest.rs`:

- file name: `ggml-small.bin`
- model URL:

```text
https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.bin?download=true
```

- bytes: `487601967`
- SHA-256: `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`

Observed sibling files in the HF repo include:

- `ggml-small.bin`
- `ggml-small-q5_1.bin`
- `ggml-small-q8_0.bin`
- `ggml-small.en.bin`
- `ggml-small.en-q5_1.bin`
- related model files

The model file is pinned by exact URL and SHA-256 in a manifest.

---

## 8. Rust Crate Option

A Rust crate exists:

```text
whisper_cpp
```

Known metadata:

- version: `0.2.1`
- updated: `2024-02-06`
- repository: `binedge/whisper_cpp-rs`
- dependencies:
  - `whisper_cpp_sys ^0.2.1`
  - `derive_more ^0.99.17`
  - `thiserror ^1.0.50`
  - `tokio ^1.34.0`
  - `tracing ^0.1.40`

Compatibility with current whisper.cpp master is not confirmed.

The crates.io API check previously failed with a JSON decode error, so the crate should be re-verified manually.

This is still a possible option, but it should not be assumed to work.

---

## 9. Whisper.cpp Integration Options

### Option A: Use the `whisper_cpp` crate

Pros:

- Less FFI code.
- Potentially easier Rust integration.
- Fewer native build complications.

Cons:

- Version is old.
- Compatibility with current whisper.cpp master C API is unknown.
- May require pinning to an older whisper.cpp behavior.
- May not expose the needed streaming control.
- May require patching or forking.

Decision needed:

- Can `whisper_cpp 0.2.1` build and run against the whisper.cpp version it bundles or expects?
- Does it expose enough C API control for the desired streaming pattern?

---

### Option B: Build whisper.cpp from source in `build.rs`

This mirrors the existing Moonshine approach.

Pros:

- More control over the exact whisper.cpp version.
- Can build exactly what the app needs.
- Can pin source commit and verify artifacts.
- Avoids dependence on an outdated Rust crate.

Cons:

- More build complexity.
- Requires CMake.
- Requires FFI declarations.
- Requires careful memory ownership for whisper.cpp objects.
- Build time may be significant.

Likely implementation surface:

```text
src-tauri/src/asr/whisper/
```

Possible files:

- `mod.rs`
- `ffi.rs`
- `engine.rs`
- `engine_tests.rs`
- `runtime.rs`
- `manifest.rs`
- `installer.rs`
- `installer_tests.rs`

Likely `build.rs` additions:

- detect supported Linux targets
- locate whisper.cpp source, either:
  - vendored source
  - git submodule
  - downloaded source tarball
- run CMake
- emit `cargo:rustc-link-search`
- emit `cargo:rustc-link-lib`
- emit `cargo:rustc-cfg=whisper_native_linked`

This option is likely more robust if the existing `whisper_cpp` crate is not current enough.

---

### Option C: Parallel whisper-specific pipeline

Instead of making `LocalAsrPipeline` engine-agnostic, add a separate whisper pipeline path.

Pros:

- Lower risk to existing Moonshine code.
- Easier to isolate failures.
- Simpler to reason about during implementation.

Cons:

- More duplicate code.
- More IPC/UI state to maintain.
- Potentially harder to generalize later.

This may be a reasonable short-term approach, but Option B plus a shared transcript-update type is likely cleaner long-term.

---

## 10. Recommended Architecture

The leading design is:

1. Introduce an engine-agnostic transcript update type.
2. Change `PipelineEngine` to use that type.
3. Keep Moonshine as one implementation.
4. Add whisper as another implementation.

Possible Rust type:

```rust
pub enum TranscriptUpdate {
    Partial {
        segment_id: u64,
        text: String,
        latency_ms: u32,
    },
    Final {
        segment_id: u64,
        text: String,
        latency_ms: u32,
    },
}
```

Then update:

```rust
fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<TranscriptUpdate>, AsrError>
```

This would replace the current Moonshine-specific:

```rust
fn push_pcm(&mut self, pcm: &[f32]) -> Result<Vec<MoonshineTinyTranscriptUpdate>, AsrError>
```

Likely new `AsrMode` value:

```rust
AsrMode::WhisperSmallStreaming
```

Likely new architecture type:

```rust
WhisperModelArchitecture::SmallStreaming
```

or a shared:

```rust
AsrArchitecture
```

This will require updating:

- `src-tauri/src/asr/types.rs`
- `src-tauri/src/asr/pipeline.rs`
- `src-tauri/src/asr/moonshine/engine.rs`
- `src-tauri/src/conversation/session.rs`
- `src-tauri/src/conversation/session/local_asr.rs`
- settings/onboarding UI
- IPC contract
- generated backend contract
- frontend tests

---

## 11. Files to Review Before Implementing Whisper

### 11.1 Must read

- `/home/phil/work/ai-talking-moose/src-tauri/src/asr/types.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/src/asr/pipeline.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/src/asr/mod.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/src/conversation/session.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/src/conversation/session/local_asr.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/src/asr/moonshine/mod.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/src/asr/moonshine/engine.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/src/asr/moonshine/ffi.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/src/asr/moonshine/installer.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/src/asr/moonshine/manifest.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/src/bin/wake_word_acceptance.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/src/app/wake_word_engine.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/src/asr/wake_word_sherpa_manifest.rs`
- `/home/phil/work/ai-talking-moose/src-tauri/Cargo.toml`
- `/home/phil/work/ai-talking-moose/src-tauri/build.rs`

### 11.2 Must inspect for whisper

- `https://github.com/ggml-org/whisper.cpp/tree/master/include`
- `https://github.com/ggml-org/whisper.cpp/tree/master/examples/stream`
- `https://github.com/ggml-org/whisper.cpp/tree/master/CMakeLists.txt`
- `https://github.com/ggml-org/whisper.cpp/tree/master/models/download-ggml-model.sh`
- `https://huggingface.co/ggerganov/whisper.cpp`
- `https://crates.io/crates/whisper_cpp`

### 11.3 Relevant scripts

- `/home/phil/work/ai-talking-moose/scripts/check_local_llm_packaging_policy.py`
- `/home/phil/work/ai-talking-moose/scripts/render_asr015_benchmark_report.py`
- `/home/phil/work/ai-talking-moose/scripts/prepare-wake-word-artifacts.py`
- `/home/phil/work/ai-talking-moose/src-tauri/build.rs`

---

## 12. Exact Next Steps

### Step 1: Pull this handoff

On the other computer:

```bash
git clone git@github.com:ekkus93/ai-talking-moose.git
cd ai-talking-moose
git checkout master
git pull origin master
```

Then read:

```text
docs/LOCAL_ASR_WHISPER_HANDOFF_2026-10-03.md
docs/LOCAL_LLM_ARCHITECTURE.md
docs/MOONSHINE_LOCAL_ASR_PIPELINE.md
docs/WAKE_WORD_V1_ARCHITECTURE_2026-09-15.md
docs/WAKE_WORD_V1.md
```

The local LLM architecture doc is relevant because it defines local model boundary patterns, but whisper.cpp ASR may need its own boundary.

---

### Step 2: Verify the repository

After pulling:

```bash
npm ci
npm run check:all
```

If the full gate is too slow, run the split gates:

```bash
npm run check:frontend
npm run check:rust
npm run check:generated-backend-contract
```

The Rust gate is:

```bash
npm run check:rust
```

which expands to:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

---

### Step 3: Confirm whisper.cpp integration approach

Before writing code, decide between:

1. Use `whisper_cpp` crate.
2. Build whisper.cpp via CMake in `build.rs` and write FFI.
3. Add a parallel whisper-specific pipeline.

The safest default is likely:

```text
Build whisper.cpp via build.rs/CMake + FFI, with an engine-agnostic TranscriptUpdate type.
```

But only proceed after confirming:

- desired whisper.cpp source version
- desired model file
- exact SHA256
- license
- supported targets
- build toolchain requirements
- packaging policy impact

---

### Step 4: Confirm whisper-small file metadata

Check:

```bash
curl -sI "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.bin?download=true"
```

Expected values:

- size: `487601967`
- repo commit: `5359861c739e955e79d9a303bcbc70fb988958b1`

Still need:

- license
- live SHA-256 verification against the pinned bytes

If downloading locally for verification only, use a temporary location outside the repo:

```bash
mkdir -p /tmp/whisper-check
cd /tmp/whisper-check
curl -fL "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.bin?download=true" -o ggml-small.bin
sha256sum ggml-small.bin
```

Do not commit the downloaded file.

---

### Step 5: Implement the new ASR engine surface

Likely new Rust module:

```text
src-tauri/src/asr/whisper/
```

Likely new files:

```text
src-tauri/src/asr/whisper/mod.rs
src-tauri/src/asr/whisper/ffi.rs
src-tauri/src/asr/whisper/engine.rs
src-tauri/src/asr/whisper/engine_tests.rs
src-tauri/src/asr/whisper/runtime.rs
src-tauri/src/asr/whisper/manifest.rs
src-tauri/src/asr/whisper/installer.rs
src-tauri/src/asr/whisper/installer_tests.rs
```

Likely changes:

1. Add:
   ```rust
   pub enum TranscriptUpdate {
       Partial {
           segment_id: u64,
           text: String,
           latency_ms: u32,
       },
       Final {
           segment_id: u64,
           text: String,
           latency_ms: u32,
       },
   }
   ```

2. Update `PipelineEngine` to return:
   ```rust
   Result<Vec<TranscriptUpdate>, AsrError>
   ```

3. Update Moonshine engine to convert:
   ```rust
   MoonshineTinyTranscriptUpdate
   ```
   into:
   ```rust
   TranscriptUpdate
   ```

4. Add:
   ```rust
   AsrMode::WhisperSmallStreaming
   ```

5. Add a whisper model manifest similar to the Moonshine manifest.

6. Add a whisper installer similar to the Moonshine installer.

7. Add a whisper runtime/engine that:
   - loads the model once
   - owns whisper.cpp context memory
   - accepts 16 kHz PCM
   - emits partial/final transcript updates
   - releases model memory deterministically
   - fails explicitly for unsupported templates/versions

8. Wire whisper into:
   - `src-tauri/src/conversation/session.rs`
   - `src-tauri/src/conversation/session/local_asr.rs`
   - local ASR start paths
   - local ASR diagnostics
   - local ASR error reporting

---

### Step 6: Add diagnostics

Extend local ASR diagnostics to include whisper-specific state if useful:

- model path
- model SHA256
- whisper.cpp source commit
- build configuration
- current input sample rate
- queue capacity
- last transcript latency
- last segment id
- last error
- install state

The existing diagnostics store is in:

```text
/home/phil/work/ai-talking-moose/src-tauri/src/conversation/session/local_asr.rs
```

It currently stores:

- `input_sample_rate_hz: 16_000`
- `queue_capacity: LOCAL_ASR_QUEUE_CAPACITY_CHUNKS`
- `last_error`

---

### Step 7: Update settings/onboarding UI

The UI should allow selecting a local ASR mode and should clearly distinguish:

- Moonshine Tiny Streaming
- Moonshine Small Streaming
- Whisper Small Streaming
- Gemini Live Audio

Important UX constraints:

- Do not imply that selecting a local model grants permission to download it.
- Do not silently switch to Google or another model if a local model is missing.
- If a model is missing, show an explicit install/download action.
- If local ASR fails, do not silently fall back to cloud.

---

### Step 8: Update IPC contract

If IPC shapes change, regenerate and review the generated backend contract.

Commands:

```bash
npm run generate:frontend-contract
npm run check:generated-backend-contract
npm run check:frontend-contract-shapes
```

The generated contract file is:

```text
src/generated/backendContract.json
```

Review the diff carefully. Do not blindly commit generated drift.

---

### Step 9: Add tests

Likely test areas:

- manifest parsing
- SHA256 verification
- installer failure modes
- model missing
- model corruption
- sample-rate rejection
- unsupported architecture rejection
- transcript update mapping from Moonshine to `TranscriptUpdate`
- whisper engine memory release
- local ASR diagnostics after whisper failure
- IPC contract shape stability

Use injected fakes or test doubles where possible. Do not require real whisper model weights for ordinary unit/integration tests.

---

### Step 10: Benchmark and evidence

If whisper.cpp local ASR is implemented, produce evidence:

- source commit
- model file SHA256
- corpus SHA256
- latency
- accuracy
- memory usage
- CPU usage
- failure behavior

Align with:

```text
scripts/render_asr015_benchmark_report.py
```

Record evidence in `docs/` if it becomes part of the project record.

---

## 13. Commands Useful on the New Machine

### 13.1 Repository setup

```bash
git checkout master
git pull origin master
npm ci
```

### 13.2 Verification

```bash
npm run check:all
```

Or split:

```bash
npm run check:frontend
npm run check:rust
npm run check:generated-backend-contract
```

### 13.3 Rust-only checks

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features
```

### 13.4 Frontend-only checks

```bash
npm run typecheck
npm run lint
npm run format:check
npm run test
npm run build
```

### 13.5 Contract checks

```bash
npm run check:tauri-command-contract
npm run check:frontend-contract-shapes
npm run check:generated-backend-contract
```

### 13.6 Packaging policy checks

```bash
python3 scripts/check_local_llm_packaging_policy.py
python3 scripts/render_asr015_benchmark_report.py
```

The first is currently local-LLM/llama.cpp specific and may need ASR extension.

---

## 14. Open Questions

1. **Which whisper.cpp source version should be pinned?**
   - current `master`?
   - a specific release tag?
   - a commit hash?

2. **Which whisper-small file should be used?**
   - `ggml-small.bin`
   - `ggml-small-q5_1.bin`
   - `ggml-small-q8_0.bin`
   - `ggml-small.en.bin`

3. **What is the exact SHA256 of the chosen file?**

4. **What is the exact license for the chosen file?**

5. **Should whisper.cpp be built via CMake in `build.rs` or used through the `whisper_cpp` crate?**

6. **Should `LocalAsrPipeline` become engine-agnostic now, or should whisper use a parallel pipeline first?**

7. **What streaming parameters should be used?**
   - `step_ms`
   - `length_ms`
   - `keep_ms`
   - max history
   - overlap strategy

8. **Does `scripts/check_local_llm_packaging_policy.py` need to be extended for ASR models?**

9. **Should the empty `tests/__init__.py` be kept or removed?**

10. **Should the removal of `filter="data"` in `scripts/prepare-wake-word-artifacts.py` be confirmed or reverted?**

---

## 15. Do Not Do

- Do not commit whisper model weights.
- Do not commit `.gguf`, `.GGUF`, `.bin`, `.ort`, or other model binary files.
- Do not add real model downloads to ordinary CI.
- Do not weaken `scripts/check_local_llm_packaging_policy.py` to make whisper work.
- Do not silently fall back to Google when local ASR fails.
- Do not duplicate Google model identifiers outside `src-tauri/src/ai/google/config.rs`.
- Do not replace `npm run check:rust` with weaker Rust checks when validating Rust changes.
- Do not commit generated `node_modules/` or `dist/` content.

---

## 16. High-Level Status Summary

### Working

- Wake-word detection using sherpa-onnx on Linux.
- Moonshine local ASR on macOS.
- Cloud Gemini Live Audio path.
- Desktop shell close-window behavior.
- Existing local LLM architecture.
- Existing frontend/backend IPC contract gates.
- Whisper.cpp `WhisperSmall` local ASR core: manifest, installer, Linux build integration, final transcript dispatch, diagnostics, and frontend selection.

### Partial

- Local ASR pipeline still has Moonshine-shaped components.
- Whisper.cpp engine emits only final transcript updates after the 300 ms batch threshold; partials/windowing are not complete.
- Pinned `ggml-small.bin` URL, source commit, size, and SHA-256 are in `src-tauri/src/asr/whisper/manifest.rs`, but live download verification is still pending.
- `ggml-small.bin` weight license is pending.
- `src-tauri/src/asr/whisper/runtime.rs` is not created; lease/worker logic is still distributed across installer/engine/pipeline.
- P5 real-CPU acceptance has not been run.

### Not Working

- Fully streamed Whisper.cpp partial transcription.
- Release-complete local Linux ASR until weight-license verification, live download verification, P2 partial/windowing, and real-CPU acceptance are finished.

### Next Major Milestone

Finish P0/P2/P5 for Whisper.cpp: verify the `ggml-small.bin` weight license and live download bytes/SHA-256, add partial/windowing support in `src-tauri/src/asr/whisper/engine.rs`, and run P5 real-CPU acceptance.

---

## 17. Files Changed in This Handoff Commit

The handoff commit adds:

```text
docs/LOCAL_ASR_WHISPER_HANDOFF_2026-10-03.md
```

It also commits the previously uncommitted working-tree changes listed in section 4.

Files changed before this handoff:

```text
scripts/prepare-wake-word-artifacts.py
src-tauri/capabilities/default.json
src-tauri/gen/schemas/capabilities.json
src-tauri/src/bin/wake_word_acceptance.rs
src/windows/MooseWindow.tsx
tests/__init__.py
```

The new file:

```text
docs/LOCAL_ASR_WHISPER_HANDOFF_2026-10-03.md
```

---

## 18. Suggested First Commit After Pulling on the New Machine

Once on the new machine and after re-running checks, the next implementation commit should probably be one of:

```text
docs: record whisper.cpp ASR integration decision
feat(asr): add engine-agnostic TranscriptUpdate type
feat(asr): add WhisperSmallStreaming mode stub
chore(asr): build whisper.cpp via build.rs
```

Do not mix unrelated UI or wake-word changes into the whisper implementation commit unless necessary.

---

## 19. Final Handoff Note

The most important current gap is finishing Whisper.cpp V1 acceptance: verify the `ggml-small.bin` weight license and live download bytes/SHA-256, complete P2 partial/windowing in `src-tauri/src/asr/whisper/engine.rs`, and run P5 real-CPU acceptance. The codebase already has a strong local ASR and installer pattern based on Moonshine, and the Whisper `WhisperSmall` path is now wired end-to-end for final transcription.

The preferred path forward is:

1. Confirm whisper.cpp build/integration approach.
2. Confirm live `ggml-small.bin` SHA-256 verification and license.
3. Introduce an engine-agnostic `TranscriptUpdate` type.
4. Add a whisper.cpp module under `src-tauri/src/asr/whisper/`.
5. Wire it into `LocalAsrPipeline` and `AsrMode`.
6. Add installer, diagnostics, UI, tests, and packaging policy coverage.
7. Record benchmark evidence in `docs/`.
