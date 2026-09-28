# WPCR-110 Settings Rollback and Status Evidence

**Date:** 2026-09-27
**Remediation item:** WPCR-110 — Settings enable/disable listener ownership
**Exact master:** `1976d2ad3edb514ce0fea72c6b34dc700ab5e58c`

## Scope

This evidence records focused deterministic coverage for the Settings/listener control-plane rollback and backend status classification boundaries. It does not claim final closure of Settings persistence failure rollback or frontend UI assertions.

## Implemented coverage

`src-tauri/src/app/wake_word_settings_rollback_tests.rs` adds three tests:

- `listener_rollback_restores_previous_runtime_and_listener_settings` proves a failed listener restart can be followed by rollback to the previous Wake listener settings, restoring the runtime to `Listening` and classifying the previous listener as `Active`.
- `unsupported_asr_settings_failure_is_sanitized_and_actionable` proves the unsupported-ASR Settings failure remains the actionable sanitized string `Wake Word V1 requires local Moonshine command ASR` and does not contain raw paths, secret markers, or audio-content wording.
- `settings_enable_disable_refreshes_backend_status_after_completion` proves the backend status classifier reports `Active` after successful enable completion and `Stopped` after successful disable completion.

These tests use the shared Settings/listener control-plane helper instead of frontend-only patched Settings state.

## Exact validation

- Exact-master ordinary CI passed: run `36376930414` at `1976d2ad3edb514ce0fea72c6b34dc700ab5e58c`.
- Exact-master Wake source-security audit passed: run `36376930415` at `1976d2ad3edb514ce0fea72c6b34dc700ab5e58c`.

## Remaining scope

The following WPCR-110 work remains open and should not be claimed by this evidence:

- persisted Settings rollback when a later Settings persistence/runtime preference operation fails;
- frontend Settings UI tests that assert real backend state after enable/disable;
- final user-level enable/disable acceptance across the full Settings UI flow.

## Privacy

The tests use deterministic Settings/runtime state and approved fixed error strings only. They do not persist raw microphone PCM, private transcripts, credentials, user paths, or audio content.
