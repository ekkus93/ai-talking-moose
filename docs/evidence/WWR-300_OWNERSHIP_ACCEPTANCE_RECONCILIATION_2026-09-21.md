# WWR-300 ownership acceptance reconciliation — 2026-09-21

This evidence records the post-remediation state of the authoritative microphone ownership path on master `f3edb50516e2464188f9d40aa3389a03093fd336`.

## Qualified production behavior

The Wake path is composed around the application's existing shared `Arc<parking_lot::Mutex<AudioCapture>>`; `AuthoritativeWakeCaptureOwner` does not construct or retain a second physical `AudioCapture`. Wake start, command handoff, command return, restart, disable, and cancellation all operate on that shared owner.

The following deterministic acceptance behaviors are now represented in source tests and have passed merged-master CI:

- Wake start and disable use the exact shared application capture owner.
- Replacing a Wake epoch reuses that same capture owner.
- Command handoff stops the shared capture before command ASR takes ownership.
- Command return restarts Wake through the same owner.
- Failed command return leaves capture stopped and Wake in `Error`; reconnect through the same owner restores `Listening`.
- Failed/unavailable-device Wake start fails closed: capture is stopped and stale Wake orchestration is cleared.
- Cancellation while Wake remains enabled returns through the same owner; cancellation after Wake is disabled leaves capture idle for manual listen.
- Wake disable releases the shared capture and manual listen can subsequently acquire it.

Merged-master CI `35614688330` passed on exact master SHA `f3edb50516e2464188f9d40aa3389a03093fd336`.

## WWR-300 checklist evidence mapping

These existing behaviors support reconciliation of the following TODO requirements once the TODO itself is updated on a qualified reconciliation PR:

- Ensure Wake Word does not open a competing continuous microphone stream.
- Implement deterministic ownership transfer to command ASR.
- Implement deterministic ownership return to wake listening.
- Handle reconnect.
- Handle unavailable device.
- Handle permission/capture error without spin/deadlock, for the capture-start error path currently exercised.
- Ensure cancellation does not orphan or multiply streams.
- Wake disable tears down/suspends capture according to final policy.
- Manual listen still works with Wake Word disabled.
- Device error leaves deterministic ownership.
- Cancellation leaves deterministic ownership.
- Exactly one authoritative microphone ownership model exists in the implemented Wake composition boundary.
- No simultaneous competing capture opens occur in the implemented handoff/return boundary.

## Items not claimed complete by this evidence

This reconciliation deliberately does **not** claim the remaining end-to-end production-composition and audio-flow requirements merely because lower-level ownership tests pass. In particular, the following still require direct source/acceptance evidence before their TODO boxes should be checked:

- Wake manager wiring into the complete production application lifecycle/composition.
- Canonicalize/resample microphone PCM once where practical.
- Feed ring buffer and KWS from the same chronological canonical stream.
- Preserve live samples immediately after trigger while command ASR initializes.
- Device-disconnect handling distinct from startup/unavailable-device failure.
- Repeated wake→ASR→wake cycles with an explicit stream-count invariant, rather than only pointer identity/active-state assertions.

This separation prevents the TODO from overstating completion while preserving exact evidence for the ownership work already merged.