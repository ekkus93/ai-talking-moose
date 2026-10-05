# Whisper.cpp Local ASR TODO

Status: Proposed implementation plan. P0 verification gates complete (2026-10-05).
Recorded: 2026-10-03
Spec: `docs/WHISPER_CPP_LOCAL_ASR_SPEC.md`

## P0: Verification gates

- [x] Pin exact whisper.cpp source commit.
- [x] Pin exact `ggml-small.bin` SHA256.
- [x] Pin exact `ggml-small.bin` byte size.
- [x] Record exact license text for `ggml-small.bin`.
- [x] Record exact license text for vendored whisper.cpp source, if vendored.
- [x] Confirm supported target matrix:
  - [x] Linux x86_64 CPU.
  - [x] Linux aarch64 CPU, if supported.
  - [x] macOS behavior, if included.
- [x] Confirm native integration approach:
  - [x] default: `build.rs`/CMake FFI;
  - [x] fallback: pinned Rust crate, only if compatible.
- [x] Update `docs/PRIVACY.md` to cover local Whisper ASR.
- [x] Update `docs/LOCAL_ASR_WHISPER_HANDOFF_2026-10-03.md` with implementation progress.

## P1: Types, manifest, and build

- [ ] Add `AsrMode::WhisperSmall` to `src-tauri/src/asr/types.rs`.
- [ ] Add serialized form `"whisper_small"`.
- [ ] Add a Whisper architecture or model identity type:
  - [ ] e.g. `WhisperModelArchitecture::Small`;
  - [ ] or a broader local ASR engine identity if a cleaner abstraction is needed.
- [ ] Create `src-tauri/src/asr/whisper/mod.rs`.
- [ ] Create `src-tauri/src/asr/whisper/manifest.rs`:
  - [ ] model id `whisper-small-ggml`;
  - [ ] display name;
  - [ ] upstream repository;
  - [ ] pinned source commit;
  - [ ] Hugging Face asset URL;
  - [ ] expected byte size;
  - [ ] SHA256;
  - [ ] license;
  - [ ] runtime compatibility.
- [ ] Add manifest validation:
  - [ ] non-empty id;
  - [ ] non-empty display name;
  - [ ] pinned revision in URL;
  - [ ] HTTPS only;
  - [ ] valid SHA256;
  - [ ] valid byte size;
  - [ ] no path traversal in model file names.
- [ ] Create `src-tauri/src/asr/whisper/installer.rs`:
  - [ ] explicit download only;
  - [ ] bounded staging directory;
  - [ ] exact size verification;
  - [ ] SHA256 verification;
  - [ ] atomic promotion;
  - [ ] cancellation;
  - [ ] disk-space check;
  - [ ] install-state progress.
- [ ] Store models under:
  - [ ] `<app-data>/models/whisper/whisper-small/`.
- [ ] Extend `src-tauri/build.rs`:
  - [ ] whisper.cpp source/CMake build;
  - [ ] `whisper_native_linked` cfg;
  - [ ] rerun on native source changes;
  - [ ] optional `TALKING_MOOSE_WHISPER_LIB_DIR`;
  - [ ] Linux link target for `libwhisper`.
- [ ] Create `src-tauri/src/asr/whisper/ffi.rs`:
  - [ ] safe wrappers;
  - [ ] no public whisper.cpp types;
  - [ ] stable C API only;
  - [ ] 16 kHz mono int16 input.

## P2: Engine and pipeline

- [ ] Create `src-tauri/src/asr/whisper/engine.rs`:
  - [ ] open verified model;
  - [ ] run `whisper_full`;
  - [ ] convert segments to transcript updates;
  - [ ] track utterance state;
  - [ ] emit partial and final events.
- [ ] Create `src-tauri/src/asr/whisper/runtime.rs`:
  - [ ] model lease;
  - [ ] worker ownership;
  - [ ] shutdown behavior.
- [ ] Extend local pipeline:
  - [ ] add `LocalAsrPipeline::start_whisper_small`, or
  - [ ] refactor Moonshine startup behind a local ASR engine trait.
- [ ] Add worker thread name:
  - [ ] `whisper-small-asr`.
- [ ] Preserve bounded queue:
  - [ ] capacity 8 chunks;
  - [ ] drop-newest overload policy;
  - [ ] no CPAL blocking.
- [ ] Implement transcription windowing:
  - [ ] accumulate 100 ms PCM chunks;
  - [ ] bounded recent window for partials;
  - [ ] final flush on endpoint or stop.
- [ ] Map Whisper failure states to:
  - [ ] `AsrErrorKind::ModelNotInstalled`;
  - [ ] `AsrErrorKind::ModelCorrupt`;
  - [ ] `AsrErrorKind::RuntimeUnavailable`;
  - [ ] `AsrErrorKind::ModelLoadFailed`;
  - [ ] `AsrErrorKind::AudioInput`;
  - [ ] `AsrErrorKind::Inference`;
  - [ ] `AsrErrorKind::InvalidState`;
  - [ ] `AsrErrorKind::Cancelled`;
  - [ ] `AsrErrorKind::Internal`.
- [ ] Ensure no fallback to Gemini Live, Moonshine, or fake provider.

## P3: Conversation, wake word, and IPC

- [ ] Update `ConversationManager` local ASR preparation:
  - [ ] verify Whisper model before session open;
  - [ ] fail closed before microphone capture;
  - [ ] attach Whisper pipeline to lifecycle;
  - [ ] stop microphone before stopping Whisper worker.
- [ ] Update local ASR mode classification:
  - [ ] `is_local_mode(AsrMode::WhisperSmall) == true`.
- [ ] Update wake-word support:
  - [ ] confirm whether Whisper mode supports wake word;
  - [ ] update `wake_word_asr_mode_supported` if supported;
  - [ ] preserve fail-closed behavior for unsupported mode.
- [ ] Update `commands/asr_models.rs`:
  - [ ] `get_asr_models` returns Whisper descriptor;
  - [ ] `install_asr_model` supports Whisper;
  - [ ] `delete_asr_model` supports Whisper;
  - [ ] emit `moose://asr/model-progress` for Whisper installs.
- [ ] Update `commands/asr_diagnostics.rs`:
  - [ ] compose diagnostics for `AsrMode::WhisperSmall`;
  - [ ] report engine name;
  - [ ] report model id;
  - [ ] report model revision;
  - [ ] report install state;
  - [ ] preserve queue and dropped-chunk metrics.
- [ ] Update generated backend contract:
  - [ ] run `npm run generate:frontend-contract` when IPC shapes change;
  - [ ] review diff;
  - [ ] commit regenerated contract.

## P4: Frontend and docs

- [ ] Update frontend ASR model list:
  - [ ] show Whisper Small as local;
  - [ ] show install state;
  - [ ] show expected and installed byte counts;
  - [ ] show runtime revision;
  - [ ] disable install while conversation is active.
- [ ] Update settings disclosure:
  - [ ] Whisper ASR is local;
  - [ ] model download is explicit;
  - [ ] no microphone audio is sent to Google;
  - [ ] partial transcript behavior may be batch-based.
- [x] Update `docs/PRIVACY.md`.
- [ ] Update `README.md` if local ASR options are described there.
- [ ] Add or update:
  - [ ] `docs/WHISPER_CPP_LOCAL_ASR_PIPELINE.md`, if runtime behavior needs its own document;
  - [ ] `docs/WHISPER_CPP_CPU_BENCHMARK.md`, if benchmarks are produced.

## P5: Real CPU acceptance and performance

- [x] Create a separate real CPU acceptance workflow:
  - [x] Linux runner;
  - [x] pinned whisper.cpp commit;
  - [x] pinned `ggml-small.bin` SHA256;
  - [x] no real model in ordinary CI;
  - [x] no real model in repository.
- [ ] Record evidence:
  - [x] source SHA;
  - [x] model SHA;
  - [x] artifact bytes;
  - [x] test audio;
  - [x] transcription output;
  - [x] latency;
  - [ ] CPU usage;
  - [ ] RSS.
- [ ] Benchmark on supported Linux hardware:
  - [ ] first partial latency;
  - [ ] final transcript latency;
  - [ ] real-time factor;
  - [ ] process RSS;
  - [ ] dropped chunks under overload.
- [ ] Tune transcription windowing:
  - [ ] partial interval;
  - [ ] endpoint silence threshold;
  - [ ] maximum utterance length;
  - [ ] queue capacity if 800 ms is insufficient.

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
