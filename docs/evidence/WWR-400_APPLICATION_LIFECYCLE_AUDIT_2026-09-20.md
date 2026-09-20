# WWR-400 — Application lifecycle integration audit

Evidence head reviewed: `6d64096178eda56fab59fcae785bddf16fe9f656`.

This audit separates lifecycle-manager behavior that is already implemented from the production audio/conversation wiring that remains mandatory.

## Implemented and source-backed

- `src-tauri/src/lib.rs` calls `app::wake_word_state::initialize_from_app_state(&app_state)` during production setup after persisted settings are loaded.
- `src-tauri/src/app/wake_word_state.rs` seeds the single process-wide `WakeWordApplicationRuntime` from normalized `AppState` settings and leaves physical microphone ownership with `AppState::audio_capture`.
- Disabled persisted settings construct the runtime in `Disabled`; enabled persisted settings construct it in `Loading`, not falsely in `Listening`.
- `WakeWordApplicationRuntime` provides explicit Talking suspension, clears retained pre-roll through the manager transition, honors the latest enabled setting on resume, fails closed on capture/runtime error, and enters shutdown explicitly.
- Live settings changes are applied to the authoritative runtime through `runtime_preferences::apply_changed_runtime_preferences`, including rollback if a later reversible preference side effect fails.
- Unit tests cover disabled/enabled construction, shared runtime-manager state, Talking suspension and ring clearing, terminal resume, disable-during-Talking precedence, recoverable runtime error, and capture-error fail-closed behavior.

These facts support the WWR-400 implementation substance for: one authoritative application composition owner, initialization from persisted settings, disabled-by-default runtime behavior, Talking suspension/ring clearing, disable-during-interaction precedence, and manager-level resume/error behavior.

## Still open — production end-to-end lifecycle

The following must not be inferred from manager/component tests:

- enabled startup does not yet prove that the verified native KWS engine is loaded and the one physical microphone stream is started/routed into it;
- an accepted native wake trigger is not yet wired to start exactly one normal command interaction;
- command-ASR/Thinking/Talking transitions are not yet all connected to the Wake runtime boundary in the production conversation graph;
- TTS success/cancellation/recoverable-failure callbacks are not yet proven to resume the production Wake capture/KWS path;
- production KWS reset at resume is not yet proven;
- WWR-300 one-stream microphone routing and WWR-310 continuous pre-roll/live handoff remain prerequisites for end-to-end WWR-400 acceptance.

Therefore WWR-400 acceptance (`Moose cannot wake itself from its own TTS` and `One integrated lifecycle controls Wake Word end-to-end`) remains open until those production paths are wired and qualified.

## Next implementation boundary

The next code slice should connect the existing `AppState::audio_capture` owner to the Wake runtime/engine without opening a second capture stream, then route accepted triggers through the existing normal conversation-start path. That slice must preserve manual start when Wake Word is disabled and must add repeated-cycle stream-count/ownership tests before any WWR-400 end-to-end acceptance checkbox is closed.
