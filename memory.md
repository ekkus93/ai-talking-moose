# Whisper Local ASR Integration — 2026-10-04

## Objective
- Add local Whisper.cpp `whisper-small` GGML ASR to `/home/phil/work/ai-talking-moose` while preserving Moonshine local ASR, Gemini Live cloud ASR, and fail-closed local-ASR behavior.
- Bring `cargo check` and `cargo test --lib` to a fully passing compile state for the Whisper installer/engine integration.

## Important Details
- Project root: `/home/phil/work/ai-talking-moose`; Rust backend: `src-tauri/`; frontend: `src/`.
- Whisper.cpp vendored at `third_party/whisper.cpp`, pinned to commit `5359861c739e955e79d9a303bcbc70fb988958b1`.
- `build.rs` builds Whisper.cpp via CMake/static-link; supports `TALKING_MOOSE_WHISPER_LIB_DIR`.
- Whisper is local-only, batch/non-streaming; no cloud fallback.
- Shared local ASR constants: `LOCAL_ASR_QUEUE_CAPACITY_CHUNKS = 8`, `LOCAL_ASR_INPUT_SAMPLE_RATE_HZ = 16_000`, `WORKER_POLL_INTERVAL = 25ms`, `PRODUCTION_WORKER_STARTUP_TIMEOUT = 30s`, test timeout `250ms`.
- Pinned Whisper model manifest: id `whisper-small-ggml`, source commit `5359861c739e955e79d9a303bcbc70fb988958b1`, URL `https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.bin?download=true`, SHA-256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`, bytes `487601967`.
- Async mutex: `tokio::sync::Mutex` aliased as `AsyncMutex`; lease sync pattern uses `blocking_lock_owned()`.
- `AsrError` fields: `kind` (`AsrErrorKind`), `message`, `retryable`.
- Current compile state: **`cargo check` passes with 0 errors** (69 warnings, mostly dead code from unwired code).
- Current test state: **`cargo test --lib` passes 854 tests, 0 failed, 3 ignored**.

## Work State
### Completed
- Whisper.cpp submodule pinned; `build.rs` updated for CMake/static-link.
- Whisper foundation skeleton written: `AsrMode::WhisperSmall`, `mod.rs`, `ffi.rs`, `manifest.rs`, `engine.rs`.
- `mod.rs` declares `engine` and re-exports `WhisperModelInstaller`.
- `pipeline.rs`: `PipelineEngine: Send` made public; `start_whisper` accepts `Arc<WhisperModelInstaller>`.
- `installer.rs` written with per-model lock, staging, verification, rollback.
- `installer/integrity.rs`, `progress.rs`, `disk.rs`, `delete.rs` all written.
- `verify_installed` changed to sync.
- `acquire_verified_model_lease` changed to sync using `blocking_lock_owned()`.
- `E0716` temporary-borrow in `delete()` and `install()` fixed by binding `Arc` to local.
- `engine.rs` rewritten as **free function** `pub fn open(installer: Arc<WhisperModelInstaller>) -> Result<WhisperEngine, AsrError>`:
  - handles `acquire_verified_model_lease()` via explicit `match` on `Result<Option<WhisperVerifiedModelLease>, WhisperModelInstallError>`.
  - converts `WhisperModelInstallError` to `AsrError` with `retryable` field.
  - returns fail-closed error for `Ok(None)` with message "The Whisper Small model is not installed..."
  - stores `Arc<WhisperModelInstaller>` (not the lease) in `WhisperEngine`.
  - implements `PipelineEngine` (sample rate 16000, push_pcm/stop stubs returning empty/Ok).
- `local_asr.rs` Whisper branch fail-closed with `AsrErrorKind::RuntimeUnavailable`.
- Warnings cleaned: removed unused `manifest` import from `pipeline.rs`; changed `error` → `_error` in `installer.rs:420`.
- **`cargo check` passes with 0 errors.**
- **Pre-existing test build failures fixed:**
  - `pipeline_tests.rs`: `MoonshineModelArchitecture` → `LocalAsrArchitecture` in all usages (E0425/E0433/E0308 fixed).
  - `integrity.rs:83`: `size_of::<Variant>` → `bytes.len() * 2` (E0573 + assertion fixed).
  - **`cargo test --lib` now passes: 854 tests, 0 failures, 3 ignored.**

### Active
- **Open unknowns (unchanged):**
  - exact license text for `ggml-small.bin` missing.
  - exact license text for vendored Whisper.cpp source missing.
  - supported target matrix only partially confirmed.
  - pinned SHA-256 not yet verified against actual download.

### Blocked
- No blockers. All compile and test gates pass.

## Next Move
1. **Implement real Whisper batch transcription in `engine.rs`** — replace the stub `PipelineEngine` impl (`push_pcm`/`stop`) with actual whisper.cpp calls.
2. **Wire `WhisperModelInstaller` into `src-tauri/src/conversation/session/local_asr.rs`** — replace fail-closed P1 branch with real Whisper pipeline startup.
3. **Update install/delete/status commands** (`commands/asr_models.rs`), diagnostics, frontend settings/types, `PRIVACY.md`, `README.md`, and handoff docs.
4. Run CPU acceptance workflow, benchmark, tune Whisper windowing.

## Relevant Files
- `src-tauri/src/asr/whisper/engine.rs`: free-function `open`; currently stub `PipelineEngine` impl; stores `Arc<WhisperModelInstaller>`.
- `src-tauri/src/asr/whisper/installer.rs`: full installer with sync lease, `E0716` fixed; `let mut result` warning at line 406.
- `src-tauri/src/asr/whisper/installer/integrity.rs`: fixed assertion (`bytes.len() * 2` instead of 64).
- `src-tauri/src/asr/whisper/installer/progress.rs`: fixed `size_of::<Variant>` → `size_of::<WhisperModelInstallPhase>()`.
- `src-tauri/src/asr/pipeline_tests.rs`: fixed `MoonshineModelArchitecture` → `LocalAsrArchitecture` in all 6 usages; removed unused import.
- `src-tauri/src/asr/pipeline.rs`: `start_whisper` calls `engine::open(Arc<...>)`.
- `src-tauri/src/conversation/session/local_asr.rs`: Whisper branch fail-closed (P1).
- `src-tauri/src/asr/whisper/mod.rs`: declares `ffi`, `engine`, `installer`, `manifest`.
- `src-tauri/src/asr/whisper/ffi.rs`: C FFI bindings.
- `src-tauri/src/asr/whisper/manifest.rs`: pinned model metadata.
- `src-tauri/src/asr/whisper/installer/disk.rs`: `DiskIntegrity` trait.
- `src-tauri/src/asr/whisper/installer/delete.rs`: delete implementation.
- `src-tauri/src/asr/whisper/installer/transport.rs`: download transport.
- `src/tauri/src/asr/whisper/installer/transport.rs`: download transport.
- `src-tauri/src/commands/asr_models.rs`: needs Whisper install/delete/status.
- `src-tauri/src/commands/asr_diagnostics.rs`: needs Whisper diagnostics.
- `src-tauri/src/asr/types.rs`: `AsrMode`, `LocalAsrArchitecture`, `AsrErrorKind`, `AsrError`.
- `src-tauri/src/asr/mod.rs`: module exports.
- `docs/WHISPER_CPP_LOCAL_ASR_TODO.md`: master task list.
- `docs/WHISPER_CPP_LOCAL_ASR_SPEC.md`: V1 scope.
- `docs/WHISPER_CPP_LOCAL_ASR_HANDOFF_2026-10-03.md`: handoff doc; needs update.
- `docs/PRIVACY.md`: needs Whisper update.
- `README.md`: top-level docs.
- `package.json`: check scripts.
- `src-tauri/Cargo.toml`: dependencies.
- `src-tauri/build.rs`: Whisper CMake/static-link build.
- `third_party/whisper.cpp`: vendored source.
- `src/types/moose.ts`: frontend ASR types.
- `src/components/Settings/AsrSettingsPanel.tsx`: Whisper UI.
