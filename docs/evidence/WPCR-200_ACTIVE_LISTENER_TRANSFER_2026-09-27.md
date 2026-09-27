# WPCR-200 Active Listener Transfer Evidence

**Date:** 2026-09-27
**Scope:** WPCR-200 manual conversation shared-capture transfer while the native Wake listener is actively owning capture.
**Implementation commit:** `b040fa1cd63917f91963f1b3fd323c153dab0662`

## Summary

Current `master` fixes the active-listener manual-transfer boundary in `NativeWakeListenerControl::TransferToCommand`. Before the fix, an active listener shutdown could disable the Wake runtime before the command-suspension guard ran. The fixed boundary restores the runtime to a listening state after intentional active-listener shutdown, then runs the normal command-interaction suspension path so the manual command owns capture while Wake remains intentionally `SuspendedTalking` instead of being stranded disabled or error.

## Implemented behavior

- Manual transfer first detects whether a native listener handle is active.
- The active listener is stopped through the shared listener control plane.
- Listener shutdown releases the authoritative `AudioCapture` before command ASR can start.
- When a listener was active, the runtime is restored to the command-suspendable listening boundary and then transitioned through `suspend_for_command_interaction`.
- The transfer records no Wake error for intentional shutdown.
- The listener slot is empty after transfer, preventing duplicate microphone ownership.

## Deterministic test evidence

The test `active_listener_transfer_releases_capture_and_preserves_command_suspension` in `src-tauri/src/app/wake_word_state.rs` starts the local listener thread with a deterministic in-process KWS test engine, verifies capture is active, transfers ownership with `NativeWakeListenerControl::TransferToCommand`, and asserts:

- the transfer reports that an active listener was present;
- the native listener slot is cleared;
- a `Stopped` listener event is emitted;
- application capture is no longer active;
- Wake runtime phase is `SuspendedTalking`;
- no Wake runtime error is recorded.

## Exact validation evidence

PR #494 exact head `614e0458a4123648681ebcbc4b07b753aa273545` passed:

- Ordinary CI: `36349867506`
- Wake Word lifecycle stability: `36349867516`
- Wake Word source security audit: `36349867526`

Merged `master` commit `b040fa1cd63917f91963f1b3fd323c153dab0662` passed:

- Ordinary CI: `36350297078`
- Wake Word lifecycle stability: `36350297048`
- Wake Word source security audit: `36350297103`

## Remaining boundaries

This evidence closes the active-listener transfer regression and duplicate-capture ownership risk for manual transfer. Final WPCR-900/950/960 closeout remains open, and broader product-level final qualification still must re-run every required final gate at the final exact head and final exact merged master.
