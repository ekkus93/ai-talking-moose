# WWR-400 TTS lifecycle integration audit — 2026-09-19

Base commit: `7c7542e3f7e320d537581fac132f709a96afd326`

## Verified production state

`src-tauri/src/app/wake_word_composition.rs` now provides the process-wide authoritative `WakeWordApplicationRuntime`. It is initialized from normalized persisted settings and exposes explicit `suspend_for_talking()` and `resume_after_interaction(wake_word_enabled)` boundaries. Suspension clears retained ring/pre-roll through `WakeWordRuntimeManager`; resume honors a disable that raced with Talking.

The runtime still deliberately owns no microphone stream. `AppState::audio_capture` remains the existing capture owner, preserving the WWR-300 one-stream direction.

## Remaining TTS wiring gap

The production standalone TTS path in `src-tauri/src/commands/speech.rs` transitions `CharacterState` to `Talking` in `surface_standalone_playback`, but it does not call the Wake Word application runtime before that transition. `schedule_standalone_completion` returns the character to Idle after successful playback but does not resume Wake Word. Its cancellation path returns early and likewise does not resume Wake Word.

Repository-wide inspection found no production caller of `WakeWordApplicationRuntime::suspend_for_talking` or `resume_after_interaction`; current uses are confined to composition/runtime tests. Therefore the WWR-400 Talking/TTS checklist must remain open despite the runtime primitives being implemented.

## Required implementation slice

The production speech path must:

1. Suspend Wake Word before publishing `CharacterState::Talking`, while treating Disabled/Loading/Error as non-listening states rather than breaking ordinary/manual speech.
2. If surfacing Talking fails after suspension, restore Wake Word according to the latest `wake_word_enabled` setting.
3. Keep Wake Word suspended for the entire playback lifetime.
4. Resume on successful completion using the latest persisted/in-memory setting.
5. Resume after cancellation when the cancelled playback still owns the foreground speech slot; a superseded playback must not resume Wake Word underneath its replacement.
6. Preserve `Disabled` when the user disables Wake Word during Talking.
7. Add regression tests for success, cancellation, surfacing failure, and disable-during-Talking.

## Acceptance impact

This audit does **not** claim WWR-400 complete. It identifies the exact missing production call sites and the ownership condition needed to avoid a cancellation/supersession race. Moose cannot yet be claimed unable to wake itself from its own TTS until these call sites are wired and qualified.
