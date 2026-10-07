# Large Code Files: Split and Refactor TODO

This plan covers the ten longest code files in the repository at the time it was created. Line counts are a baseline and may drift; the goal for every resulting source and test file is fewer than 800 lines. Keep public APIs stable through facade modules and preserve existing behavior and test coverage while extracting code.

## 1. `src-tauri/src/app/wake_word_engine.rs` — 1,463 lines

- [x] Separate the public wake-word types and error/configuration definitions from the native implementation. Keep the existing module as a facade that re-exports the supported API.
- [x] Move the Sherpa ONNX C API declarations, dynamic-library loading, symbol checks, and native resource ownership into a native-runtime/FFI module.
- [x] Move model/runtime artifact identity, platform selection, architecture checks, and file verification into an artifact-validation module.
- [x] Keep PCM validation and keyword-result interpretation with the engine/session boundary, and add module-level tests for the extracted boundaries.
- [x] Target: each production module and test module below 800 lines.

## 2. `src-tauri/src/ai/local/installer.rs` — 1,446 lines

- [x] Keep install-facing types and `LocalModelInstaller` as the small public facade.
- [x] Extract HTTP/download transport and response-to-safe-error mapping into a transport module.
- [x] Extract model-root layout checks, symlink/path defenses, artifact verification, and marker validation into a storage/verification module.
- [x] Extract staging cleanup, atomic promotion, and install-marker writes into a promotion module.
- [x] Split unit and adversarial tests by transport, verification/path defenses, and promotion/cancellation behavior; keep each test file below 800 lines.
- [x] Preserve exact size/hash checks, cancellation, atomic promotion, sanitized errors, and no implicit model substitution.

## 3. `src-tauri/src/app/state.rs` — 1,426 lines

- [x] Move the large inline test module into `app/state/tests/` and split settings migration/default tests from application-state/runtime initialization tests.
- [x] Keep `AppSettings` defaults and normalization together; move versioned settings migration and legacy-key migration into a dedicated settings-migration module if that production section remains over 800 lines after test extraction.
- [x] Keep `AppState` construction and ownership wiring in the main module, with narrow helper modules for model-root paths and secure-secret migration.
- [x] Retain tests for persisted defaults, forward-version fail-closed behavior, secret migration, and shared runtime ownership.
- [x] Target: `state.rs` and every extracted test or helper file below 800 lines.

## 4. `src-tauri/src/conversation/session/tests.rs` — 1,203 lines

- [x] Move reusable fake providers, sessions, and builders into a compact `tests/support.rs` module.
- [x] Split test cases into focused files: lifecycle/startup/shutdown; local ASR and wake-command ownership; stale generations, cancellation, and bounded provider operations; tools/audio/transcript event dispatch.
- [x] Keep assertions close to the behavior they cover and avoid broad shared fixtures that hide test setup.
- [x] Target: each test file below 800 lines, with no reduction in concurrency, cancellation, or stale-generation coverage.

## 5. `src-tauri/src/ai/local_tts/runtime/engine.rs` — 943 lines

- [x] Extract tar parsing and bounded runtime-library extraction helpers into a private archive module.
- [x] Keep Kitten TTS engine creation, voice resolution, inference, and output-contract validation in the engine module.
- [x] Keep normalization helpers in the existing normalization module and move tests with the behavior they exercise.
- [x] Preserve archive bounds, artifact identity checks, waveform shape validation, and sanitized runtime errors.
- [x] Target: engine and archive modules below 800 lines.

## 6. `src-tauri/src/conversation/session.rs` — 934 lines

- [x] Keep `ConversationManager` as the lifecycle facade and separate start reservation, shutdown, and generation ownership into a lifecycle module if the implementation remains over 800 lines.
- [x] Move provider timeout/bounded-operation helpers and provisional-session cleanup into a provider-operations module.
- [x] Keep microphone forwarding, transcript acceptance, and callback/event translation in focused modules with explicit inputs and outputs.
- [x] Reuse the existing `event_loop` and `local_asr` module boundaries rather than duplicating their responsibilities.
- [x] Target: the facade and each implementation module below 800 lines; keep lifecycle tests exercising the same serialized shutdown and cancellation guarantees.

## 7. `src-tauri/src/asr/moonshine/installer.rs` — 875 lines

- [x] Keep the installer API, install outcomes, and verified-model lease facade in `installer.rs`.
- [x] Review existing `delete`, `disk`, `integrity`, `progress`, and `transport` submodules; move remaining orchestration or staging-directory ownership into focused modules instead of growing the facade.
- [x] Keep install-lock ownership and staging cleanup in one clearly documented lifecycle boundary.
- [x] Target: reduce `installer.rs` below 800 lines while preserving the existing helper modules and all install/delete synchronization guarantees.

## 8. `src-tauri/src/asr/moonshine/installer_tests.rs` — 843 lines

- [x] Move fake transport, fake responses, and disk-space probes into a shared installer-test support module.
- [x] Split test cases into download/manifest/integrity failures; cancellation/concurrency/locking; and filesystem repair/delete/lease behavior.
- [x] Keep fixture bytes and manifest constants close to the tests that use them, or centralize them in support if shared.
- [x] Target: every test file below 800 lines without dropping symlink-swap, interrupted-download, or verified-lease coverage.

## 9. `src-tauri/src/app/wake_word_acceptance.rs` — 817 lines

- [x] Keep corpus loading, fixture acceptance criteria, and the simple real-keyword acceptance flow in the primary acceptance module.
- [x] Move production-listener performance measurement types, synthetic ingress/starter adapters, repeated-cycle execution, and percentile calculations into a dedicated performance-acceptance module.
- [x] Move OS-specific CPU and peak-memory measurement helpers behind a small platform-metrics module or the performance module.
- [x] Keep report schemas and command-facing entry points stable through re-exports.
- [x] Target: all acceptance modules below 800 lines, with fixture path safety and report semantics unchanged.

## 10. `src-tauri/src/ai/local_tts/runtime.rs` — 815 lines

- [x] Move runtime cancellation token state and operations into a small private cancellation module, re-exporting the existing type from the runtime facade if needed.
- [x] Move runtime identity/platform resolution and global artifact verification into a focused identity/verification module.
- [x] Keep runtime manager lifecycle and serialized inference ownership in the runtime module; retain the existing engine, NPZ, tokenization, and telemetry test boundaries.
- [x] Target: bring `runtime.rs` below 800 lines without weakening artifact validation or cancellation behavior.

## Completion checklist

- [x] Keep each resulting source and test file below 800 lines.
- [x] Preserve module-level public APIs with explicit re-exports where callers depend on current paths.
- [x] Add or move tests with each extraction; do not delete coverage to meet the line target.
- [x] Run the repository Rust and frontend quality gates, generated-contract checks when IPC shapes change, and the relevant packaging/privacy checks when those boundaries are touched.
- [x] Recount the ten files and update this plan when each item is completed.

Verification note: Rust Clippy (`-D warnings`), all Rust tests, targeted Rust formatting checks, and every frontend gate passed. The repository-wide Rust formatting check still reports two formatting-only differences in the pre-existing modified `src-tauri/src/asr/whisper/acceptance.rs`, which was left untouched. IPC shapes and packaging/privacy behavior were not changed, so their conditional checks were not needed.

Post-refactor sizes for the ten baseline files: `wake_word_engine.rs` 28, `installer.rs` (Local LLM) 732, `state.rs` 588, `conversation/session/tests/mod.rs` 315, `local_tts/runtime/engine.rs` 426, `conversation/session.rs` 419, `asr/moonshine/installer.rs` 682, `asr/moonshine/installer_tests.rs` 194, `wake_word_acceptance.rs` 194, and `local_tts/runtime.rs` 681 lines. Every extracted source and test file is also below 800 lines.
