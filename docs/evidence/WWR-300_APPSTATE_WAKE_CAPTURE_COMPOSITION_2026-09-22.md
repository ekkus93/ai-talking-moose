# WWR-300 / WWR-400 — AppState Wake Capture Composition Evidence

**Date:** 2026-09-22
**Scope:** Add a source-level production composition boundary that ties the authoritative Wake runtime manager to the application's existing microphone owner.

## Change

`WakeWordApplicationRuntime` now exposes `capture_consumer(engine)`, which builds a `WakeCapturePcmConsumer` from the same `WakeWordRuntimeManager` clone owned by `AppState::wake_word_runtime`.

This does not open a microphone stream. The physical stream remains owned by `AppState::audio_capture` and is still started only through `AuthoritativeWakeCaptureOwner::from_shared_capture`.

## Regression coverage

`wake_word_appstate_composition_tests::app_state_composes_shared_capture_with_authoritative_wake_runtime` verifies that:

- the Wake PCM consumer mutates the same runtime manager observed through `AppState::wake_word_runtime`;
- the capture owner uses the exact `AppState::audio_capture` object;
- starting Wake capture uses the canonical 16 kHz capture rate;
- disabling Wake releases the shared capture owner and returns the authoritative Wake runtime to `Disabled`.

## Acceptance boundary

This advances the WWR-300/WWR-400 composition wiring. It does **not** claim real microphone, real KWS, real wake→ASR, or integrated lifecycle acceptance; those remain governed by the real-audio WWR-300/310/400/640 tasks and the specialized acceptance gates.
