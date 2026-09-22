# WWR-300 native startup boundary — 2026-09-22

## Scope

This evidence records the current production-composition startup boundary for Wake Word V1 on `master`.

## Source evidence

`src-tauri/src/app/wake_word_state.rs` now exposes `start_native_wake_from_app_state`. The function:

- reads the authoritative persisted settings from `AppState`;
- leaves Wake disabled and does not construct native KWS artifacts when `wake_word_enabled` is false;
- applies the enabled setting before native startup when `wake_word_enabled` is true;
- constructs the real verified `NativeKwsSession` through `WakeWordApplicationRuntime::native_capture_consumer`;
- marks the shared Wake runtime loaded only after native session construction succeeds;
- starts capture through `capture_owner_from_app_state`, which uses the existing `AppState::audio_capture` rather than allocating a Wake-owned microphone path;
- records a recoverable runtime/capture error and leaves the shared capture owner inactive when native artifact or capture startup fails.

## Regression coverage

`disabled_startup_does_not_load_native_or_open_capture` proves disabled startup returns without loading missing native artifacts and without opening capture.

`enabled_startup_fails_closed_before_capture_when_native_artifacts_missing` proves enabled startup with absent verified artifacts fails before capture opens and transitions the Wake runtime to `Error`.

Exact merged-master validation for `9f2e665698738989b24e0aa8ee47c3ee01b7473c` passed:

- ordinary CI `35755461757`;
- Wake Word lifecycle stability `35755461837`;
- Wake Word source security audit `35755461895`.

## Non-claims

This does not claim full application lifecycle activation is complete. The current startup boundary is present in source, but later work must still own the durable listener task, settings-driven stop/restart, trigger-to-command activation, Talking suspension/resume integration, device reconnect acceptance, repeated wake→ASR→wake soak, and real KWS corpus acceptance.

This also does not claim real positive/negative audio fixture inference. WWR-600, WWR-610, and WWR-620 remain open.
