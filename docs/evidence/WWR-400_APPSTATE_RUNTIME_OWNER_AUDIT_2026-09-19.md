# WWR-400 AppState Wake Word runtime owner audit

Date: 2026-09-19
Baseline master: `abaa8c59b392553b3372936ae181953f340a1e20`

## Purpose

This evidence records the current production-composition gap before mutating the large `AppState` and settings transaction files. It is intentionally narrow: it does not claim WWR-400 is complete.

## Verified current state

`src-tauri/src/app/wake_word_composition.rs` already defines `WakeWordApplicationRuntime`, the intended application-level Wake Word owner. That owner wraps one shared `WakeWordRuntimeManager`, initializes from normalized `AppSettings`, enters `Loading` when persisted Wake Word is enabled, remains `Disabled` when disabled, exposes Talking/TTS suspension and resume semantics, and deliberately owns no microphone stream.

`src-tauri/src/app/state.rs` still does not store a `WakeWordApplicationRuntime` field in `AppState`. Therefore the production application graph does not yet have a single authoritative Wake Word runtime owner even though the composition type exists.

`src-tauri/src/commands/settings.rs` normalizes and persists Wake Word settings, but live `update_settings` does not yet apply `wake_word_enabled` transitions to a production `WakeWordApplicationRuntime` because `AppState` does not yet own one.

The current shared microphone owner remains `AppState::audio_capture`. That must stay true: WWR-300 selected the one-stream strategy, so the AppState Wake Word runtime owner must not open a competing capture stream.

## Required implementation constraints

The next code mutation should:

1. Add exactly one `WakeWordApplicationRuntime` to `AppState`.
2. Construct it in `AppState::new_with_secret_store` after persisted settings are loaded and normalized.
3. Initialize it from the effective `AppSettings`, not from raw persisted JSON.
4. Preserve disabled-by-default behavior.
5. Enter `Loading` rather than falsely claiming `Listening` when persisted settings enable Wake Word before native KWS artifacts are loaded.
6. Preserve the existing `AudioCapture` ownership model; this owner must not start microphone capture.
7. Make live Wake Word enable/disable settings updates apply to the runtime without app restart where safe.
8. Keep settings writes transactional: if a runtime enable/disable operation fails before persistence, persisted settings must remain unchanged; if persistence fails after a reversible runtime transition, the runtime should be restored to the previous enabled state.
9. Add deterministic tests proving AppState clones share one runtime owner, persisted enabled settings initialize the owner to `Loading`, disabled settings initialize it to `Disabled`, and enabling the owner does not start microphone capture.

## Why this remains open

WWR-400 remains incomplete until the source implements the owner field and the settings update path, and exact-head CI validates the change. Later slices still need actual trigger-to-command activation and deeper lifecycle wiring around active ASR, Thinking, Talking, TTS success/cancel/failure, and error recovery.
