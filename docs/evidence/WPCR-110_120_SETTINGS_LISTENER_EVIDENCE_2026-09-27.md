# WPCR-110/120 Settings Listener Remediation Evidence

**Date:** 2026-09-27
**Scope:** WPCR-110 Settings enable/disable listener ownership and WPCR-120 input-device / ASR-mode listener restart behavior.
**Evidence commit:** `bf79159fb389bbe1961c83fcad1918aae43233af`

## Summary

Current `master` contains a shared Settings listener-control path for Wake setting changes. The production boundary is `apply_configured_native_wake_listener_settings_change` in `src-tauri/src/app/wake_word_state.rs`, which delegates to the injectable `apply_configured_native_wake_listener_settings_change_with_control` helper. That helper routes Wake enablement, disablement, input-device changes, and ASR-mode changes through `NativeWakeListenerControl` instead of only mutating the `WakeWordApplicationRuntime` phase.

The implementation keeps Policy B intact: Wake Word V1 remains local-Moonshine-only for command ASR. Unsupported ASR modes stop the listener path, record a fail-closed runtime state, and return the sanitized error `Wake Word V1 requires local Moonshine command ASR` before listener startup can claim a listening state.

## Implemented behavior at evidence commit

- Enabling Wake from disabled calls `NativeWakeListenerControl::RestartForSettings` with the pending Settings payload, rather than relying on stale persisted settings.
- Disabling Wake from a listening runtime calls `NativeWakeListenerControl::Stop` before reporting the runtime as disabled.
- Changing the input device while Wake is enabled stops the existing listener and restarts with the new input-device value when no conversation is active.
- Changing to another supported local Moonshine ASR mode while Wake is enabled stops and restarts the listener with the new mode.
- Changing to an unsupported ASR mode while Wake is enabled fails closed without leaving an active listener.
- Input-device changes during an active conversation stop the current listener and leave the runtime in a pending/loading state, with diagnostics classified as `PendingUntilIdle`.
- Listener restart failure records `WakeWordRuntimePhase::Error` and diagnostics classify the state as `FailedClosed`, avoiding a false listening claim.

## Deterministic test evidence

The following deterministic tests are present in `src-tauri/src/app/wake_word_settings_change_tests.rs` at `bf79159fb389bbe1961c83fcad1918aae43233af`:

- `enable_from_disabled_restarts_listener_with_pending_settings`
- `disable_from_listening_stops_before_reporting_disabled`
- `input_device_change_stops_then_restarts_with_new_device`
- `supported_asr_mode_change_stops_then_restarts_with_new_mode`
- `active_conversation_defers_device_restart_and_reports_pending`
- `listener_restart_failure_fails_closed_without_false_listening_claim`
- `enabled_asr_mode_change_to_unsupported_fails_closed_without_listener`
- `enabled_input_device_change_without_startup_config_enters_loading_without_listener_claim`

Related transactional rollback coverage remains in `src-tauri/src/app/runtime_preferences.rs` through `failed_wake_listener_change_restores_previous_runtime_state`, which verifies a failed Wake listener change restores the previous runtime phase and does not leave a listener active.

## Exact-master validation evidence

At evidence commit `bf79159fb389bbe1961c83fcad1918aae43233af`, the following exact-master GitHub Actions runs passed:

- Ordinary CI: `36347933313`
- Wake Word lifecycle stability: `36347933318`
- Wake Word source security audit: `36347933322`

## Remaining boundaries

This evidence does not close final WPCR-950/960 qualification. It also does not claim real native KWS platform acceptance, production listener performance measurements, or final source/privacy/security audit completion. Those remain owned by the final exact-head and exact-master gates in the post-closeout TODO.
