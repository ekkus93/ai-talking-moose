# WPCR-200 Manual Transfer Terminal Recovery Evidence

**Date:** 2026-09-27
**Remediation item:** WPCR-200 — manual conversation shared-capture transfer
**Exact master:** `6cbf39b6dcafd545fb9da9365f36e377a701924b`

## Scope

This evidence records deterministic coverage for manual command transfer terminal recovery after Wake has yielded command ownership. It complements the active-listener transfer evidence in `docs/evidence/WPCR-200_ACTIVE_LISTENER_TRANSFER_2026-09-27.md` rather than replacing it.

## Implemented coverage

`src-tauri/src/app/wake_word_manual_transfer_tests.rs` adds focused tests for the command-ownership guard used by manual conversation start and cancel/failure terminal paths:

- `manual_start_failure_after_active_wake_guard_does_not_leave_wake_suspended` starts from an enabled/listening Wake runtime, transfers command ownership through `NativeWakeListenerControl::TransferToCommand`, verifies the runtime enters `SuspendedTalking`, then resolves a recoverable start failure and verifies Wake returns to `Listening` without recording a Wake error.
- `cancel_and_recoverable_failure_paths_release_command_suspension` proves both cancellation and recoverable failure release the command guard and return enabled Wake to `Listening` instead of leaving it permanently suspended.
- `disable_during_manual_transfer_wins_over_restart_after_cancel` proves the latest disabled setting wins at terminal cancellation, returning Wake to `Disabled` instead of restarting.

Together with `active_listener_transfer_releases_capture_and_preserves_command_suspension` in `src-tauri/src/app/wake_word_state.rs`, the coverage proves the two relevant halves of the active manual-transfer path: an active native listener is stopped and releases capture before command ownership, and subsequent recoverable start failure/cancel paths release the command suspension according to the latest Wake setting.

## Exact validation

- Exact-master ordinary CI passed: run `36375659976` at `6cbf39b6dcafd545fb9da9365f36e377a701924b`.
- Exact-master Wake source-security audit passed: run `36375659947` at `6cbf39b6dcafd545fb9da9365f36e377a701924b`.

## Privacy

The tests use only deterministic runtime state and listener-control boundaries. They do not persist raw microphone PCM, transcripts, credentials, private paths, or user audio.

## Remaining scope

This evidence does not close the still-open WPCR-110 Settings persistence/UI rollback items, WPCR-310 native-KWS spoken fixture detection, or final WPCR-900/950/960 closeout gates.
