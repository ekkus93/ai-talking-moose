# Whisper.cpp Local ASR TODO

Status: Complete; implementation tasks and subtasks were re-audited. Real-CPU evidence and exact-master closeout are recorded in `docs/WHISPER_CPP_LOCAL_ASR_POST_REVIEW_REMEDIATION_TODO_2026-10-07.md`.
Recorded: 2026-10-03
Spec: `docs/WHISPER_CPP_LOCAL_ASR_SPEC.md`

## P0: Verification gates

- [x] Pin exact whisper.cpp source commit.
- [x] Pin exact `ggml-small.bin` SHA256.
- [x] Pin exact `ggml-small.bin` byte size.
- [x] Record exact license text for `ggml-small.bin`.
- [x] Record exact license text for vendored whisper.cpp source, if vendored.
- [x] Confirm supported target matrix:
  - [x] Linux x86_64 CPU (real-CPU qualified on the recorded GitHub runner).
  - [x] Linux aarch64 CPU, if supported (not currently claimed or real-CPU qualified).
  - [x] macOS behavior, if included (Whisper remains unavailable on non-Linux targets; build fails closed).
- [x] Confirm native integration approach:
  - [x] default: `build.rs`/CMake FFI;
  - [x] fallback: pinned Rust crate, only if compatible.
- [x] Update `docs/PRIVACY.md` to cover local Whisper ASR.
- [x] Update `docs/LOCAL_ASR_WHISPER_HANDOFF_2026-10-03.md` with implementation progress.

## P1: Types, manifest, and build

- [x] Add `AsrMode::WhisperSmall` to `src-tauri/src/asr/types.rs`.
- [x] Add serialized form `"whisper_small"`.
- [x] Add a Whisper architecture or model identity type:
  - [x] e.g. `WhisperModelArchitecture::Small`;
  - [x] or a broader local ASR engine identity if a cleaner abstraction is needed.
- [x] Create `src-tauri/src/asr/whisper/mod.rs`.
- [x] Create `src-tauri/src/asr/whisper/manifest.rs`:
  - [x] model id `whisper-small-ggml`;
  - [x] display name;
  - [x] upstream repository;
  - [x] pinned source commit;
  - [x] Hugging Face asset URL;
  - [x] expected byte size;
  - [x] SHA256;
  - [x] license;
  - [x] runtime compatibility.
- [x] Add manifest validation:
  - [x] non-empty id;
  - [x] non-empty display name;
  - [x] pinned revision in URL;
  - [x] HTTPS only;
  - [x] valid SHA256;
  - [x] valid byte size;
  - [x] no path traversal in model file names.
- [x] Create `src-tauri/src/asr/whisper/installer.rs`:
  - [x] explicit download only;
  - [x] bounded staging directory;
  - [x] exact size verification;
  - [x] SHA256 verification;
  - [x] atomic promotion;
  - [x] cancellation;
  - [x] disk-space check;
  - [x] install-state progress.
- [x] Store models under:
  - [x] `<app-data>/models/whisper/whisper-small/`.
- [x] Extend `src-tauri/build.rs`:
  - [x] whisper.cpp source/CMake build;
  - [x] `whisper_native_linked` cfg;
  - [x] rerun on native source changes;
  - [x] optional `TALKING_MOOSE_WHISPER_LIB_DIR`;
  - [x] Linux link target for `libwhisper`.
- [x] Create `src-tauri/src/asr/whisper/ffi.rs`:
  - [x] safe wrappers;
  - [x] no public whisper.cpp types;
  - [x] stable C API only;
  - [x] 16 kHz mono int16 input.

## P2: Engine and pipeline

- [x] Create `src-tauri/src/asr/whisper/engine.rs`:
  - [x] open verified model;
  - [x] run `whisper_full`;
  - [x] convert segments to transcript updates;
  - [x] track utterance state;
  - [x] emit partial and final events.
- [x] Create `src-tauri/src/asr/whisper/runtime.rs`:
  - [x] model lease;
  - [x] worker ownership;
  - [x] shutdown behavior.

  *Implementation note:* `runtime.rs` was consolidated into `asr/pipeline.rs`
  (worker ownership, bounded queue, `stop_and_join`, per-mode diagnostics
  snapshots), `asr/lifecycle.rs` (model lease and startup lease), and
  `conversation/session/local_asr.rs` (`stop_local_asr_for_shutdown` /
  `stop_provisional_local_asr`). No separate `runtime.rs` file is required.
- [x] Extend local pipeline:
  - [x] add `LocalAsrPipeline::start_whisper_small`, or
  - [x] refactor Moonshine startup behind a local ASR engine trait.

  *Implementation note:* whisper startup goes through `LocalAsrPipeline::
  start_with_factory` with a `LocalAsrArchitecture::WhisperSmall` factory that
  constructs `WhisperEngine` from a leased verified model (Moonshine keeps
  its existing factories, so there is no Moonshine-side refactor).
- [x] Add worker thread name:
  - [x] `whisper-small-asr`.
- [x] Preserve bounded queue:
  - [x] Moonshine capacity remains 8 chunks; Whisper uses its own 56-chunk queue after exact-source benchmarks showed multi-second synchronous inference;
  - [x] drop-newest overload policy;
  - [x] no CPAL blocking.
- [x] Implement transcription windowing:
  - [x] accumulate 100 ms PCM chunks;
  - [x] bounded recent window for partials;
  - [x] final flush on endpoint or stop.

  *Implementation note:* the original `4_800`-sample (300 ms) partial interval
  was measured and shown to drop 87 nominal-load chunks. Production uses
  `WHISPER_PARTIAL_INTERVAL_SAMPLES = 80_000` (5 s), verified by the exact-source
  acceptance run recorded in the remediation evidence. `stop()` flushes the
  leftover buffer, and the
  producer emits ~100 ms PCM chunks into the bounded `Vec<u8>` channel.
- [x] Map Whisper failure states to:
  - [x] `AsrErrorKind::ModelNotInstalled`;
  - [x] `AsrErrorKind::ModelCorrupt`;
  - [x] `AsrErrorKind::RuntimeUnavailable`;
  - [x] `AsrErrorKind::ModelLoadFailed`;
  - [x] `AsrErrorKind::AudioInput`;
  - [x] `AsrErrorKind::Inference`;
  - [x] `AsrErrorKind::InvalidState`;
  - [x] `AsrErrorKind::Internal`.

  *Implementation note:* transcribe FFI failures (both `push_pcm` and `stop`)
  map to `Inference` (retryable) to match Moonshine's transcribe-stream
  convention. `AsrErrorKind::Cancelled` is not emitted by the Whisper engine:
  cancellation applies to the user-initiated model download and remains in the
  installer error domain. Whisper ASR session startup, inference, and shutdown
  do not expose a cancellation operation. The shared public enum retains the
  kind for other ASR implementations.
- [x] Ensure no fallback to Gemini Live, Moonshine, or fake provider.

## P3: Conversation, wake word, and IPC

- [x] Update `ConversationManager` local ASR preparation:
  - [x] verify Whisper model before session open;
  - [x] fail closed before microphone capture;
  - [x] attach Whisper pipeline to lifecycle;
  - [x] stop microphone before stopping Whisper worker.
- [x] Update local ASR mode classification:
  - [x] `is_local_mode(AsrMode::WhisperSmall) == true`.
- [x] Update wake-word support:
  - [x] confirm whether Whisper mode supports wake word;
  - [x] update `wake_word_asr_mode_supported` if supported;
  - [x] preserve fail-closed behavior for unsupported mode.
- [x] Update `commands/asr_models.rs`:
  - [x] `get_asr_models` returns Whisper descriptor;
  - [x] `install_asr_model` supports Whisper;
  - [x] `delete_asr_model` supports Whisper;
  - [x] emit `moose://asr/model-progress` for Whisper installs.
- [x] Update `commands/asr_diagnostics.rs`:
  - [x] compose diagnostics for `AsrMode::WhisperSmall`;
  - [x] report engine name;
  - [x] report model id;
  - [x] report model revision;
  - [x] report install state;
  - [x] preserve queue and dropped-chunk metrics.
- [x] Update generated backend contract:
  - [x] run `npm run generate:frontend-contract` when IPC shapes change;
  - [x] review diff;
  - [x] commit regenerated contract.

P3 complete:
- `ConversationManager` preparation (`prepare_local_asr` in `conversation/session/local_asr.rs`) runs `LocalAsrPipeline::start_whisper` (via `start_architecture`) before the provider connects and before mic capture; `engine::open` calls `installer.acquire_verified_model_lease()` (size/SHA-256/magic) and returns `ModelNotInstalled`/`ModelLoadFailed` before the worker is ready. Failure returns early with lifecycle `Failed` and no capture start. The pipeline is attached via `self.local_asr.attach(generation, ...)` after capture; teardown (`begin_shutdown_locked`) calls `capture.lock().stop()` before `stop_local_asr_for_shutdown` -> `local_asr.stop_and_clear()`.
- `is_local_mode(AsrMode::WhisperSmall)` is `true` (`conversation/session/local_asr.rs`).
- `wake_word_asr_mode_supported` includes `AsrMode::WhisperSmall`; `ensure_wake_word_asr_mode_supported` fails closed for unsupported modes (`app/wake_word_state.rs`), test asserts whisper support.
- `commands/asr_models.rs`: `whisper_descriptor`, `install_whisper`, whisper delete path, and `moose://asr/model-progress` emission are all present.
- `commands/asr_diagnostics.rs`: whisper branch composes `AsrDiagnostics` with engine name, model id/revision, install state, and queue/dropped-chunk metrics.
- `npm run check:generated-backend-contract` passed (no diff in regenerated contract).

## P4: Frontend and docs

- [x] Update frontend ASR model list:
  - [x] show Whisper Small as local;
  - [x] show install state;
  - [x] show expected and installed byte counts;
  - [x] show runtime revision;
  - [x] disable install/delete while the active conversation makes mutation unsafe.
- [x] Update settings disclosure:
  - [x] Whisper ASR is local;
  - [x] model download is explicit;
  - [x] no microphone audio is sent to Google;
  - [x] partial transcript behavior reflects batched partial refreshes and utterance finality.
- [x] Update `docs/PRIVACY.md`.
- [x] Update `README.md` where local ASR options are described.
- [x] Add or update:
  - [x] `docs/WHISPER_CPP_LOCAL_ASR_PIPELINE.md`, documenting current runtime behavior;
  - [x] `docs/WHISPER_CPP_CPU_BENCHMARK.md`, with baseline and final qualification values clearly labeled.

## P5: Real CPU acceptance and performance

- [x] Create a separate real CPU acceptance workflow:
  - [x] Linux runner;
  - [x] pinned whisper.cpp commit;
  - [x] pinned `ggml-small.bin` SHA256;
  - [x] no real model in ordinary CI;
  - [x] no real model in repository.
- [x] Record exact-source run evidence in `docs/evidence/WHISPER_CPP_LOCAL_ASR_QUALIFICATION_2026-10-08.md`:
  - [x] repository/source SHA;
  - [x] actual model SHA;
  - [x] installed artifact bytes;
  - [x] test audio identity;
  - [x] transcription output;
  - [x] transcript latency;
  - [x] CPU usage;
  - [x] RSS.
- [x] Benchmark on supported Linux hardware and record:
  - [x] first partial latency;
  - [x] final transcript latency;
  - [x] real-time factor;
  - [x] process RSS;
  - [x] dropped chunks under nominal and deliberate overload.
- [x] Verify the updated sparse partial cadence and repeat exact-source streaming acceptance; prior 300 ms partial cadence dropped 87 nominal-load chunks:
  - [x] partial interval is five seconds (80,000 samples), verified by exact-source CPU acceptance;
  - [x] endpoint silence threshold (measured setting: 500 ms);
  - [x] maximum utterance length (measured bound: 30 s);
  - [x] Whisper queue is 56 x ~100 ms (5.6 seconds); exact-source acceptance retained all nominal corpus chunks; Moonshine remains at 8 chunks.

## Verification gates

Before declaring the Whisper path implemented, run:

- [x] `npm run check:frontend`
- [x] `npm run check:rust`
- [x] `npm run check:all`
- [x] `npm run check:generated-trees`
- [x] `npm run check:generated-backend-contract`
- [x] `npm run check:tauri-command-contract`
- [x] `npm run check:frontend-contract-shapes`
- [ ] `python3 scripts/check_local_llm_packaging_policy.py`, if packaging policy is touched.

Do not add real Whisper model downloads to ordinary CI.
