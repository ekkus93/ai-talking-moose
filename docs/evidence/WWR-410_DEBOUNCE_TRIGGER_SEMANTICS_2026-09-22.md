# WWR-410 — debounce and trigger semantics evidence

**Date:** 2026-09-22

## Source behavior

Wake Word V1 uses lifecycle state, not a timer cooldown, to debounce detections.

- `WakeWordRuntimeManager::accept_trigger` accepts a trigger only while `Listening`, atomically transitions to `Triggered`, increments the privacy-safe trigger count, and records an `Instant` used only to expose trigger age.
- Additional detections while `Triggered` or `SuspendedTalking` return `false`; they cannot increment the trigger count or create another accepted activation.
- `CanonicalWakePcmRouter` stops feeding KWS after the first accepted trigger and retains subsequent canonical PCM only for the wake-to-command handoff.
- `CanonicalWakePcmRouter::return_to_wake_listening` clears stale handoff audio, resets the KWS stream, and only then returns the runtime to `Listening`.
- No cooldown or wall-clock debounce interval exists in the V1 path. A later phrase is eligible immediately after lifecycle ownership returns to `Listening`.
- `WakeWordRuntimeSnapshot` exposes only trigger count and monotonic trigger age plus bounded state/count metadata; it contains no raw PCM field.

## Regression coverage

The authoritative runtime/router tests prove:

- repeated positive frames produce one accepted trigger while the interaction owns the runtime;
- a second phrase after return to `Listening` produces a second accepted trigger without a cooldown;
- return to listening resets the KWS stream exactly once;
- post-trigger positive-looking PCM is not re-fed to KWS and is retained only for command handoff;
- diagnostics/debug output contains counts rather than retained PCM payloads.

Relevant tests are in:

- `src-tauri/src/asr/wake_word_runtime.rs`
- `src-tauri/src/app/wake_word_pcm_router.rs`
- `src-tauri/src/app/wake_word_command_activation.rs`

## Scope

This closes WWR-410 debounce/trigger semantics only. It does not claim that the production native listener is fully started from application lifecycle, that real-corpus KWS acceptance has passed, or that integrated WWR-400/WWR-640 acceptance is complete.
