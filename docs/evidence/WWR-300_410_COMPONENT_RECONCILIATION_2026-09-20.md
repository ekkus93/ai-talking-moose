# WWR-300 / WWR-410 component reconciliation evidence

Date: 2026-09-20
Baseline master: `3810b387d68b0d05d4caae108b0bd4c110f50ce7`

## WWR-300 — production application-state ownership

Current production `AppState` directly owns exactly one `WakeWordApplicationRuntime` as `wake_word_runtime` alongside the existing authoritative `Arc<Mutex<AudioCapture>>` as `audio_capture`. `AppState::new_with_secret_store` constructs the Wake runtime from normalized persisted settings and does not construct a Wake-owned capture stream. `app::wake_word_state::runtime_from_app_state` returns that same AppState-owned runtime, and its regression test proves disabled startup while the authoritative capture is inactive.

This objectively closes the narrow WWR-300 item **Wire Wake Word manager into production application state/composition**. It does **not** claim that continuous production microphone routing into KWS is complete; the remaining WWR-300 capture-routing, ownership-transfer, device-error, reconnect, and repeated-cycle acceptance items remain open.

## WWR-410 — component debounce/trigger semantics

`WakeWordRuntimeManager::accept_trigger` accepts a trigger only from `Listening`, atomically changes the phase to `Triggered`, increments a saturating privacy-safe trigger counter, and records an `Instant` used only to derive bounded trigger age. Additional positive frames while `Triggered` or `SuspendedTalking` return `false`, so no timer/cooldown masks duplicate activation.

The runtime regression `repeated_positive_frames_create_one_trigger_until_interaction_resets` proves one accepted trigger across repeated positives and proves a later trigger can be accepted after `resume_after_interaction`. `CanonicalWakePcmRouter::return_to_wake_listening` clears stale handoff state, resets the KWS stream, then resumes the runtime; its handoff-transfer regression proves the reset occurs exactly at that ownership-return boundary before later listening PCM is accepted.

The already-merged WWR-510 diagnostics evidence establishes that trigger count and last-trigger age are privacy-safe and that raw PCM cannot be represented by the serialized diagnostics surface.

These sources objectively support the following WWR-410 component requirements/tests:

- preserve one wake event → one accepted runtime trigger invariant;
- ignore repeated positive KWS frames after acceptance;
- reset KWS stream at the verified router ownership-return lifecycle point;
- permit a later phrase/trigger after return to `Listening`;
- correctness requires no cooldown and no cooldown is implemented;
- keep trigger count privacy-safe;
- keep last-trigger timing privacy-safe/monotonic via `Instant`-derived age;
- repeated positives yield one accepted trigger;
- a later trigger after resume is accepted;
- trigger diagnostics expose no PCM.

## Truthfulness boundary

This evidence is component-level reconciliation only. It does not close WWR-410 final acceptance because production microphone routing and wake→normal-command activation are not yet integrated end-to-end. In particular, **Debounce behavior is lifecycle-correct rather than timer-masking a state bug** remains an end-to-end acceptance claim until WWR-300/310/400 production integration is complete and qualified.
