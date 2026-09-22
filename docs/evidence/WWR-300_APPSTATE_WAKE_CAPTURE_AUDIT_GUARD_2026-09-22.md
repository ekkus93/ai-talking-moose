# WWR-300 / WWR-400 — AppState Wake Capture Audit Guard Evidence

**Date:** 2026-09-22
**Scope:** Guard the source-level production composition boundary that connects AppState Wake runtime ownership to the shared microphone capture owner.

## Change

`check_wake_word_source_security_audit.mjs` now requires:

- `WakeWordApplicationRuntime::capture_consumer` to remain present;
- that the capture consumer is built through `CanonicalWakePcmRouter::new(self.manager.clone(), engine)`;
- the pre-existing shared `AppState::audio_capture`, `AuthoritativeWakeCaptureOwner::from_shared_capture`, and handoff stop boundaries to remain present.

## Acceptance boundary

This is a deterministic source guard for the WWR-300/400 composition boundary. It prevents regressions back to duplicate Wake runtime/capture ownership, but it does not claim real microphone, real KWS inference, real wake→ASR handoff, or integrated lifecycle acceptance.
