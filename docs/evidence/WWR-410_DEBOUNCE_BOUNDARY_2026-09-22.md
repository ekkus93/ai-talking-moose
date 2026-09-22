# WWR-410 debounce boundary evidence — 2026-09-22

## Scope

This evidence records the current deterministic debounce behavior for Wake Word V1. It is based on source-level router/runtime tests and does not claim real-audio acceptance.

## Source evidence

`src-tauri/src/app/wake_word_pcm_router.rs` routes all accepted trigger handling through `CanonicalWakePcmRouter::route`. After a KWS detection is accepted, the shared runtime leaves `Listening` and enters `Triggered`; subsequent chunks are appended to the handoff buffer rather than re-fed to KWS. This makes repeated positive KWS frames after one accepted phrase unable to create duplicate trigger activations.

`CanonicalWakePcmRouter::return_to_wake_listening` clears stale handoff state, resets the KWS stream, and moves the runtime back to `Listening`, which allows a later phrase to become a later independent trigger without using a timer cooldown.

Trigger diagnostics remain privacy-safe because the runtime exposes counters/timing state but no PCM payloads; raw audio is held only in memory-bound pre-roll/handoff buffers.

## Regression coverage

Existing deterministic tests cover:

- one phrase with repeated positive frames yields one accepted trigger and one handoff payload;
- repeated positive frames after trigger are not fed back into KWS and do not increment trigger count;
- return to Listening resets KWS stream state;
- a later phrase after return to Listening yields a second trigger;
- no cooldown is needed for those correctness tests;
- trigger count remains a numeric diagnostic and does not expose PCM.

Relevant test names include `repeated_positive_frames_after_trigger_do_not_duplicate_command_activation` and `later_phrase_after_return_to_listening_yields_second_trigger_without_cooldown` in `src-tauri/src/app/wake_word_pcm_router.rs`.

## Validation

Exact merged-master validation for `b52b38537665946e6ee6ce16692e3abaa762c2b5` passed ordinary CI `35762260906`.

## Non-claims

This does not claim full application lifecycle activation or a production command interaction. WWR-400 remains open for one wake trigger starting exactly one normal command interaction, and WWR-640 remains open for repeated integrated wake→ASR→Thinking→Talking→wake stability acceptance.

This also does not claim corpus or real-audio false-trigger performance. WWR-600, WWR-610, WWR-620, and WWR-630 remain open.
