# WWR-400 lifecycle boundary evidence — 2026-09-22

## Scope

This evidence records the deterministic lifecycle boundaries currently implemented for Wake Word V1. It does not claim the full durable production listener task or real-audio acceptance is complete.

## Source evidence

`src-tauri/src/app/wake_word_composition.rs` owns the application-level `WakeWordApplicationRuntime`. It initializes from persisted settings, stays `Disabled` when the setting is false, enters `Loading` for enabled startup, and moves to `Listening` only after explicit load completion.

`src-tauri/src/app/wake_word_command_lifecycle.rs` provides the command-interaction guard:

- `suspend_for_command_interaction` suspends Wake from `Listening` or `Triggered` while command ASR/Thinking/Talking owns interaction flow;
- `complete_command_interaction` routes success, cancellation, and recoverable failure through one terminal policy;
- `resume_after_command_interaction` honors the latest persisted enable state, so disabling Wake during an interaction wins over resume.

`src-tauri/src/commands/speech.rs` uses Wake suspension/resume around standalone/TTS playback boundaries so Moose speech does not feed the Wake runtime while V1 barge-in remains disabled.

`src-tauri/src/app/runtime_preferences.rs` applies Wake enable/disable setting changes immediately to the shared runtime manager and rolls them back if a later runtime preference application fails.

## Regression coverage

Existing tests cover:

- persisted disabled startup remains Disabled/manual behavior;
- persisted enabled startup enters Loading and does not falsely claim Listening;
- command interaction suspends Wake until terminal resume;
- success, cancellation, and recoverable failure share the same resume policy;
- disabling Wake during command/Talking leaves the runtime Disabled after terminal completion;
- Wake error does not block the manual command guard;
- repeated terminal resolution does not leave Wake permanently suspended;
- settings toggle on moves Wake to Loading and toggle off moves Wake to Disabled;
- speech/TTS suspension resumes enabled Wake and leaves disabled Wake disabled.

Representative test locations include `src-tauri/src/app/wake_word_composition.rs`, `src-tauri/src/app/wake_word_command_lifecycle.rs`, `src-tauri/src/commands/conversation/wake_word_lifecycle_tests.rs`, `src-tauri/src/commands/speech.rs`, and `src-tauri/src/app/runtime_preferences.rs`.

## Validation

Exact merged-master validation for `ef35cff7682b18500fabf937a372e6bafe817a3c` passed ordinary CI `35763272132`.

## Non-claims

This evidence does not claim a durable background Wake listener task has been fully wired into Tauri startup/settings ownership, nor that one real Wake trigger starts exactly one normal production command interaction. WWR-300 and WWR-400 production activation items remain open until that integration exists.

This evidence also does not claim repeated integrated wake→ASR→Thinking→Talking→wake soak, real KWS fixture acceptance, or first-word downstream ASR transcription acceptance. WWR-600 through WWR-640 remain open.