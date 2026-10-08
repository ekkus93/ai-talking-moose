# Whisper.cpp Local ASR Specification

Status: Implemented and qualified on the recorded Linux x86_64 CPU runner; see the exact-source evidence and limitations in the remediation TODO.
Recorded: 2026-10-03
Primary target: private local Linux.
Related documents:

- `docs/LOCAL_ASR_WHISPER_HANDOFF_2026-10-03.md`
- `docs/WHISPER_CPP_LOCAL_ASR_PIPELINE.md` (current implementation behavior)
- `docs/WHISPER_CPP_LOCAL_ASR_POST_REVIEW_REMEDIATION_TODO_2026-10-07.md` (authoritative remediation and qualification status)
- `docs/MOONSHINE_LOCAL_ASR_PIPELINE.md`
- `docs/LOCAL_LLM_ARCHITECTURE.md`
- `docs/PRIVACY.md`

## Purpose

Add a local, private ASR path to Talking Moose using [`ggml-org/whisper.cpp`](https://github.com/ggml-org/whisper.cpp) and the `whisper-small` GGML model.

The implementation must keep microphone audio on the local machine, preserve the existing Moonshine local ASR path, preserve Gemini Live cloud-audio behavior, and fail closed when local ASR is unavailable.

## Scope

V1 scope includes:

- A new local ASR mode for Whisper.cpp `whisper-small`.
- A pinned local model manifest for `ggml-small.bin`.
- A model installer that downloads only after explicit user action.
- A Rust FFI boundary around Whisper.cpp.
- A dedicated local-ASR inference worker.
- Integration with the existing bounded microphone-to-ASR pipeline.
- Integration with `ConversationManager` session lifecycle.
- IPC descriptors and diagnostics for the new local model.
- Settings/UI surface for install, active state, and diagnostics.
- Unit and integration tests that do not require the real model.
- Privacy, packaging, and documentation updates.

## Non-goals

V1 explicitly does not include:

- Cloud ASR fallback from Whisper.cpp.
- Automatic model download on app startup.
- Committing model weights, GGUF files, whisper `.bin` files, Moonshine `.ort` files, or sherpa-onnx weights to the repository.
- Real streaming ASR with Whisper.cpp. Whisper.cpp is treated as a batch endpoint engine.
- Replacing Moonshine Tiny/Small.
- Changing the single authoritative microphone owner, `AudioCapture`.
- Adding a second CPAL stream for local ASR.

## Existing architecture this must preserve

Talking Moose currently has:

- `AudioCapture` as the only microphone owner.
- Capture downmixes to mono and resamples to 16 kHz.
- Capture emits 100 ms chunks of signed 16-bit little-endian mono PCM.
- Local Moonshine uses a bounded local-ASR queue with capacity:

  ```rust
  LOCAL_ASR_QUEUE_CAPACITY_CHUNKS: usize = 8;
  ```

  This is approximately 800 ms of queued audio.

- The local-ASR inference worker owns the bounded channel and runs native inference on a dedicated OS thread, not on the Tokio async runtime.
- Local ASR failures are surfaced through `AsrEvent::Error` and `LocalAsrRuntimeDiagnostics`.
- `ConversationManager` fails closed before opening a cloud session if local-ASR prerequisites are invalid.
- Existing local modes are:

  ```rust
  AsrMode::MoonshineTinyStreaming
  AsrMode::MoonshineSmallStreaming
  ```

  and the cloud mode is:

  ```rust
  AsrMode::GeminiLiveAudio
  ```

Whisper.cpp must plug into the same provider-neutral local-ASR contract.

## Proposed ASR mode

Add a new `AsrMode`:

```rust
pub enum AsrMode {
    #[default]
    MoonshineTinyStreaming,
    MoonshineSmallStreaming,
    GeminiLiveAudio,
    WhisperSmall,
}
```

Serialized name:

```json
"whisper_small"
```

Display name:

```text
Whisper Small (local)
```

The new mode must:

- be classified as a local ASR mode;
- not create the Gemini Live microphone-audio upload queue;
- not fall back to `GeminiLiveAudio`;
- expose the same `AsrEvent` contract;
- expose the same `AsrModelDescriptor` and diagnostics contract where applicable.

## Model definition

Use the Whisper.cpp GGML `small` model.

Proposed application-owned identity:

```text
id: whisper-small-ggml
display_name: Whisper Small (local)
engine: whisper.cpp
model_file: ggml-small.bin
sample_rate_hz: 16000
format: mono int16 PCM
```

Pinned upstream location:

```text
https://github.com/ggml-org/whisper.cpp/tree/60c0be6ac8fa71b1a2ae2dd938a31a34a508e774
https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.bin?download=true
```

Known metadata from the pinned Hugging Face asset:

```text
x-linked-size: 487601967 bytes
x-repo-commit: 5359861c739e955e79d9a303bcbc70fb988958b1
sha256: 1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b
```

Before implementation, the manifest must pin:

- exact `ggml-small.bin` SHA256;
- exact byte size after download;
- exact license text for the model file;
- exact whisper.cpp source commit;
- exact C header version or ABI revision if one is available;
- exact supported OS/architecture matrix.

The model must not be downloaded by ordinary CI or by the application unless the user explicitly selects local Whisper ASR and starts installation.

## Model storage

Proposed local storage root:

```text
<app-data>/models/whisper/whisper-small/
```

Inside that directory:

```text
ggml-small.bin
manifest.json
install.lock
verification.state
```

The installer should follow the same general safety pattern used by Moonshine and local TTS:

- explicit user-initiated download;
- bounded staging directory;
- exact size verification;
- exact SHA256 verification;
- atomic promotion;
- cancellation support;
- path-traversal protection;
- no silent provider substitution;
- no fallback to another ASR engine when the selected model is missing or corrupt.

The model directory should be excluded from repository, resource, and bundle packaging.

## Native build strategy

Preferred default:

- build Whisper.cpp from pinned source inside the Rust build;
- expose only the stable C API through Rust FFI;
- link a native `libwhisper` library on Linux;
- keep all whisper.cpp types private to the `asr/whisper/` module tree.

Proposed module layout:

```text
src-tauri/src/asr/whisper/
├── mod.rs
├── manifest.rs
├── installer.rs
├── ffi.rs
├── engine.rs
├── runtime.rs
└── tests/
    ├── ffi_tests.rs
    ├── engine_tests.rs
    ├── installer_tests.rs
    └── manifest_tests.rs
```

Build integration:

- extend `src-tauri/build.rs`;
- add a `whisper_native_linked` cfg;
- add rerun conditions for whisper.cpp native source, CMake output, and pinned build commit;
- support an explicit development escape hatch such as:

  ```text
  TALKING_MOOSE_WHISPER_LIB_DIR
  ```

- do not use a prebuilt third-party binary in the repository;
- do not commit generated native libraries to the source tree.

Alternate approach:

- evaluate the `whisper_cpp` Rust crate only if it can be pinned with the same privacy, build, and packaging guarantees.
- Until then, default to direct CMake/build.rs FFI.

## FFI contract

The V1 FFI should use only stable Whisper.cpp C functions, for example:

- `whisper_init_from_file_with_params`
- `whisper_full`
- `whisper_full_n_segments`
- `whisper_full_get_segment_text`
- `whisper_free`

The engine boundary should accept:

```text
mono int16 PCM at 16,000 Hz
```

Whisper.cpp is batch-oriented. The engine must not expose raw Whisper.cpp C types to the rest of the application.

## Audio contract

Whisper.cpp must consume the same local audio contract as Moonshine:

- 16 kHz;
- mono;
- 16-bit little-endian PCM;
- 100 ms chunks from `AudioCapture`;
- bounded queue; Moonshine retains 8 chunks and Whisper uses its separately measured 56-chunk capacity.

If the existing local pipeline queue is currently represented internally as `f32` for Moonshine, the Whisper engine may either:

- consume the underlying 16-bit PCM chunks before conversion, or
- convert from `f32` to `int16` inside the Whisper worker.

In either case, the engine boundary remains local-only and never sends audio to a cloud provider.

## Inference worker

The Whisper worker should:

- be owned by `LocalAsrPipeline` or an equivalent local-ASR engine abstraction;
- run on a dedicated OS thread named, for example:

  ```text
  whisper-small-asr
  ```

- poll the bounded local-ASR channel;
- accumulate PCM into a transcription buffer;
- invoke `whisper_full` on the buffer;
- emit `AsrEvent` values through the existing callback contract;
- never block the CPAL callback thread.

Overload policy should remain the same as the existing local pipeline:

- if the queue is full, drop the newest chunk;
- preserve older queued speech when possible;
- retain `AudioCaptureDiagnostics.dropped_chunks` as the authoritative capture-side counter;
- report pipeline queue depth from the bounded sender.

## Transcription strategy

Whisper.cpp does not provide the same line-streaming model as Moonshine. The current implementation re-runs batch inference over one bounded utterance window and emits local partial updates on a fixed cadence. The named values are owned by `src-tauri/src/asr/whisper/engine.rs`:

- partial refresh every 80,000 samples (5 seconds at 16 kHz), selected after the original 300 ms cadence dropped most audio at normal input rate;
- local RMS endpoint after 8,000 quiet samples (500 ms);
- forced finalization at 480,000 samples (30 seconds);
- local speech RMS threshold 0.008.

These thresholds are grouped in `WhisperEngineConfig`, so focused acceptance/tests can override them independently. Production currently uses the defaults above; real-CPU acceptance must still confirm that the wider cadence and queue prevent nominal-load drops. The partial cadence is not an utterance boundary. Endpoint, maximum duration, and graceful stop produce a final update. The pipeline applies and emits that update before acknowledging delivery to the engine; only then does the engine clear its bounded PCM window and advance the segment identity. No cloud VAD is involved.

## Partial and final transcript semantics

Preserve the existing `AsrEvent` semantics:

```rust
AsrEvent::SpeechStarted
AsrEvent::PartialTranscript
AsrEvent::FinalTranscript
AsrEvent::SpeechEnded
AsrEvent::Error
```

For Whisper.cpp:

- `PartialTranscript` should represent the latest partial estimate for the current utterance.
- `FinalTranscript` should be emitted once per utterance when the utterance is considered complete.
- Partial results must not be treated as final by the conversation layer.
- The transcript state machine should not infer identity from text alone; it should track utterance or segment identity where available.

## Conversation integration

`ConversationManager` should treat `AsrMode::WhisperSmall` the same as the existing local Moonshine modes:

1. Verify the Whisper model before opening a cloud or local session.
2. Fail closed before microphone capture if the model is missing, corrupt, or incompatible.
3. Start the one authoritative `AudioCapture` on the local ASR pipeline.
4. Attach the Whisper pipeline to the conversation lifecycle.
5. Publish `Listening` only after microphone startup and local-ASR readiness succeed.
6. Route final local transcripts through the existing local ASR event handling path.
7. Stop microphone capture before stopping the local ASR worker during shutdown.
8. Preserve barge-in suppression behavior without disconnecting the local recognizer unless the conversation is stopped.

The Whisper path must never create the Gemini Live microphone-audio upload queue.

## IPC and diagnostics

Existing relevant IPC surface:

- `get_asr_models`
- `install_asr_model`
- `delete_asr_model`
- `get_asr_diagnostics`
- event: `moose://asr/model-progress`

The Whisper implementation should:

- include the Whisper descriptor in `get_asr_models`;
- support installation through the same progress event;
- support deletion;
- support diagnostics through the same `AsrDiagnostics` contract.

Existing descriptors expose:

```text
id
display_name
mode
install_state
revision
runtime_release
installed_bytes
expected_bytes
active
error_message
```

For Whisper, proposed values include:

```text
id: whisper-small-ggml
display_name: Whisper Small (local)
revision: whisper.cpp commit + model asset revision
runtime_release: whisper.cpp C header or build commit
```

Diagnostics should reuse the existing local runtime fields:

```text
input_sample_rate_hz
streaming
queue_depth
queue_capacity
dropped_chunks
last_error
first_partial_latency_ms
first_final_latency_ms
last_transcription_latency_ms
processed_audio_ms
inference_wall_time_ms
real_time_factor
process_cpu_time_ms
average_cpu_utilization_percent
baseline_resident_memory_bytes
resident_memory_bytes
peak_resident_memory_bytes
```

Current RSS is a process snapshot. Peak RSS uses the operating system high-water metric (`VmHWM` on Linux and `resident_size_max` on macOS); it must not be described as a sparse-sampling peak.

## Settings and UI

The Settings/UI surface should:

- list Whisper Small as a local ASR option;
- show install state;
- show expected and installed byte counts;
- show runtime revision;
- allow installation only when the user explicitly chooses it;
- show diagnostics when active;
- make clear that selecting Whisper does not automatically download the model;
- make clear that Whisper ASR is local-only;
- make clear that local ASR does not send microphone audio to Google.

## Failure behavior

Failure handling must be explicit and fail-closed.

Map installer/engine failures to the existing `AsrErrorKind` values:

| Failure | `AsrErrorKind` |
|---|---|
| Model file missing | `ModelNotInstalled` |
| SHA256 or size mismatch | `ModelCorrupt` |
| Unsupported platform or native library | `RuntimeUnavailable` |
| Model load failure after verification | `ModelLoadFailed` |
| Invalid PCM or audio input | `AudioInput` |
| Whisper inference failure | `Inference` |
| Invalid pipeline state | `InvalidState` |
| User cancelled the explicit model download | `WhisperModelInstallErrorKind::Cancelled` (installer API, not an ASR engine error) |
| Invalid manifest or internal bug | `Internal` |

Required behavior:

- Do not fall back to Gemini Live.
- Do not fall back to Moonshine unless the user changes the selected ASR mode.
- Do not fall back to a fake provider.
- Do not send microphone audio to any cloud endpoint when local ASR is selected.
- Keep microphone capture stopped when local ASR cannot start.

## Privacy

Whisper ASR must be local-only:

- microphone audio stays on the device;
- no ASR audio is sent to Google;
- no model is downloaded except through explicit user action;
- no transcript is sent to a cloud endpoint by the ASR path;
- model files are stored only under app-owned model directories;
- installer URLs are pinned and validated;
- model provenance and license are documented;
- local model deletion removes local files but leaves the selected mode selected and not installed.

## Packaging

The packaging policy must remain weight-free:

- no `ggml-small.bin` in the repository;
- no `ggml-small.bin` in resources;
- no whisper model weights in normal CI;
- no Moonshine, sherpa-onnx, or local LLM weights bundled with the app;
- local model acquisition is runtime/user-initiated;
- the app must still build and pass ordinary CI without downloading model weights.

If whisper.cpp native source is vendored, the source code may be committed under its license, but model weights must not be committed.

## Testing strategy

Unit and integration tests must not require the real Whisper model.

Required test categories:

- `AsrMode` serialization tests;
- Whisper manifest validation tests;
- installer state-machine tests;
- FFI boundary tests using mocked native functions;
- engine batching and transcript state tests using fake PCM;
- pipeline bounded queue behavior tests;
- conversation lifecycle tests with a fake local ASR engine;
- diagnostics composition tests;
- IPC contract regeneration checks.

Real model acceptance should be separate from ordinary CI:

- download a pinned model on a Linux CPU runner;
- run transcription against synthetic or recorded test audio;
- record evidence to `docs/`;
- never add real model downloads to ordinary `check:all`.

## Current decisions and qualification gaps

- The model artifact revision, URL, expected byte count, SHA-256, and license are recorded in the runtime manifest and reconciled in the license/notice documents. The real-CPU workflow independently hashes the installed bytes; final-source evidence is recorded in `docs/evidence/WHISPER_CPP_LOCAL_ASR_QUALIFICATION_2026-10-08.md`.
- Native whisper.cpp is built from the tracked `third_party/whisper.cpp` submodule at the pinned source revision. The provenance check compares the manifest, gitlink, and checked-out source independently.
- Partial cadence, local RMS endpointing, maximum utterance duration, and bounded queue policy are implemented and documented. Their production CPU/latency/RSS impact was measured on the exact accepted source checkout; the evidence qualifies the recorded corpus and runner, not all hardware or utterance lengths.
- Cloud VAD is out of scope; the local RMS endpoint is deterministic and testable.
- Real CPU qualification covers Linux x86_64 only. Linux aarch64 is not claimed as CPU-qualified; Whisper fails closed on macOS and other non-Linux targets.
