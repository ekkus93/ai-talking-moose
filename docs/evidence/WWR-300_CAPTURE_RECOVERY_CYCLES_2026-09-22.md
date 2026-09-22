# WWR-300 — capture recovery and repeated-cycle evidence

**Date:** 2026-09-22

## Post-trigger live PCM

`CanonicalWakePcmRouter` switches from KWS feeding to the existing `WakeAsrHandoff` immediately after an accepted trigger. Subsequent canonical PCM is appended to that handoff and is not re-fed to KWS, preserving the live samples produced while command ASR is starting.

## Runtime disconnect handling

`AudioCapture` marks its authoritative capture state inactive when CPAL reports a runtime stream failure. A runtime error does not guarantee that the PCM sender is dropped immediately, so queue closure alone is insufficient evidence of device-disconnect handling.

`AuthoritativeWakeCaptureOwner::route_next` now performs a bounded 250 ms capture-health poll while waiting for the next PCM chunk. If the shared `AppState::audio_capture` becomes inactive, Wake clears pending handoff audio, transitions the shared Wake runtime to `Error`, and returns `CaptureClosed` rather than remaining indefinitely blocked on an otherwise-open queue.

Reconnect remains serialized through `restart_wake` / `WakeCaptureOrchestrator::restart_after_capture_error`. That path reopens capture through the same authoritative `AudioCapture`, resets KWS stream state, clears stale handoff state, and only then returns the runtime to `Listening`.

## Repeated ownership-cycle regression

`repeated_wake_command_cycles_reuse_exact_shared_capture_owner` executes 100 wake→command→wake ownership cycles against the explicit mock microphone. Every cycle verifies:

- the Wake owner still points at the same shared `Arc<Mutex<AudioCapture>>`;
- transfer to command ownership stops that capture;
- return to Wake reopens that same capture;
- the Wake runtime returns to `Listening`;
- final disable leaves the shared capture stopped.

Because `AudioCapture::start` begins by stopping/replacing its existing stream, these repeated cycles cannot accumulate simultaneously active streams behind the authoritative owner.

`inactive_authoritative_capture_fails_wake_closed_without_queue_close` separately proves that an inactive authoritative capture is observed as a Wake error without relying on receiver closure.

## Scope

This evidence closes the WWR-300 implementation items for post-trigger live retention, disconnect handling, reconnect handling, and repeated shared-owner cycles. It does not claim real-hardware disconnect/reconnect acceptance, real KWS corpus acceptance, or production application-start lifecycle wiring; those remain covered by later open tasks.
