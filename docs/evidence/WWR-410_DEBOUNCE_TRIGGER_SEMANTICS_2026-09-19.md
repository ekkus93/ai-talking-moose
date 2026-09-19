# WWR-410 debounce and trigger semantics evidence

Date: 2026-09-19
Baseline master: `806956580df80f2b6ea9083e024049d4e1e0990a`

## Scope

This evidence reconciles the currently implemented Wake Word runtime debounce semantics. It does not claim final integrated production wake-to-command activation is complete; WWR-300/WWR-400/WWR-310 still own that integration work.

## Implemented semantics

`src-tauri/src/asr/wake_word_runtime.rs` implements debounce through lifecycle state rather than through an arbitrary cooldown timer.

`WakeWordRuntimeManager::accept_trigger` accepts a trigger only while the runtime is in `Listening`. The first accepted trigger moves the runtime to `Triggered`, snapshots the chronological pre-roll, increments the privacy-safe trigger counter with saturating arithmetic, and records a monotonic `Instant` for age-only diagnostics.

While the runtime is already `Triggered`, additional positive KWS frames return `Ok(false)` and do not increment `trigger_count`. While `SuspendedTalking`, additional activations also return `Ok(false)`. Disabled, Loading, Error, and ShuttingDown states fail closed rather than creating another command activation.

`WakeWordRuntimeManager::resume_after_interaction` is the verified reset point. It clears stale ring/pre-roll state and returns the runtime to `Listening` only from `Triggered`, `SuspendedTalking`, or already-`Listening` states. A second phrase after this reset can create a new trigger and increments the counter again.

No cooldown timer is implemented in this runtime. The correctness invariant is encoded in the phase machine: `Listening -> Triggered -> resume_after_interaction() -> Listening`. This satisfies the V1 requirement to avoid masking lifecycle bugs with timers unless real acceptance later proves a cooldown is necessary.

## Deterministic test coverage

The test `repeated_positive_frames_create_one_trigger_until_interaction_resets` proves:

- the first positive frame accepts and moves to `Triggered`;
- a second positive frame while `Triggered` returns false;
- `trigger_count` remains `1` for repeated positives before reset;
- `resume_after_interaction` permits a later phrase;
- the later phrase increments `trigger_count` to `2`.

The tests `listening_pcm_is_bounded_and_trigger_snapshot_is_chronological`, `resume_and_disable_clear_stale_audio`, and `talking_suspension_blocks_activation_and_clears_audio_until_resume` additionally prove stale pre-roll is cleared at lifecycle reset/suspension boundaries and cannot be replayed twice.

`WakeWordDiagnostics` exposes only bounded trigger count and `last_trigger_age_ms`; it does not serialize raw PCM, transcripts, credentials, or filesystem paths. This is covered by diagnostics serialization tests in `src-tauri/src/asr/wake_word_diagnostics.rs`.

## Remaining dependencies

WWR-410 should remain tied to later integration until WWR-400 proves exactly one accepted wake event starts exactly one normal command interaction in production application composition. The runtime-level debounce invariant is complete, but production activation semantics still depend on WWR-300/WWR-310/WWR-400 wiring.
