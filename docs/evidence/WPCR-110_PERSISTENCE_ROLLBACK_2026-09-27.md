# WPCR-110 — Settings persistence rollback evidence

Date: 2026-09-27
Evidence baseline: `4538ca0f9fb0feca694d98c73cc59954bdb5d472`

## Scope

This evidence records WPCR-110 Settings rollback behavior present on current `master`. It covers persisted Settings rollback, listener/runtime rollback, and frontend save completion ordering after rollback. It does not claim the remaining frontend diagnostics-refresh assertion for enable/disable; that item remains separate unless stable UI test evidence is merged.

## Production path

`src-tauri/src/commands/settings.rs` applies runtime preferences before persistence, then calls `persist_and_apply_settings`. If persistence fails, it calls `apply_changed_runtime_preferences(&app, &new_settings, &previous)` to restore runtime side effects before returning the persistence error.

`src-tauri/src/app/runtime_preferences.rs` routes Wake changes through `apply_wake_listener_change_transactionally`, which uses `wake_word_state::apply_configured_native_wake_listener_settings_change` for forward application and rollback from rejected settings back to previous settings.

`src-tauri/src/app/settings_policy.rs` persists serialized `AppSettings` before mutating in-memory runtime settings or behavior-engine configuration, so a persistence failure leaves the authoritative settings snapshot unchanged.

`src/stores/mooseStore.ts` now awaits `reconcileSettingsAfterWriteFailure()` before resolving the queued settings write. This prevents the Settings UI caller from treating a failed save as complete before the frontend has reloaded or rebuilt the authoritative post-rollback settings view.

## Test evidence

The following deterministic tests cover the backend rollback boundary on current `master`:

- `wake_word_settings_rollback_tests::listener_rollback_restores_previous_runtime_and_listener_settings` proves that a failed Wake listener restart rolls back from the rejected input device to the previous listener settings and returns runtime phase to `Listening`.
- `wake_word_settings_rollback_tests::unsupported_asr_settings_failure_is_sanitized_and_actionable` proves the user-visible unsupported-ASR error stays actionable and does not expose raw paths, secrets, or audio content.
- `wake_word_settings_rollback_tests::settings_enable_disable_refreshes_backend_status_after_completion` proves backend status classification reports active listener ownership after enable and stopped listener ownership after disable completion.
- `settings_policy::tests::failed_persistence_leaves_runtime_settings_and_behavior_unchanged` proves persistence failure leaves in-memory runtime settings and behavior-engine configuration unchanged.

## Exact validation

Exact `master` `13fd0a30cd13f7576fd7192e7af58f931127407c` passed ordinary CI run `36379891781` for the backend rollback/status tests already present at that head.

Exact `master` `4538ca0f9fb0feca694d98c73cc59954bdb5d472` passed ordinary CI run `36381028445` for the frontend settings rollback completion-order fix.

## Boundaries

This evidence supports the backend/persistence rollback items in WPCR-110 and Settings/listener lifecycle ordinary-CI coverage in WPCR-800. It does not close final WPCR-900/950/960 and does not by itself prove a stable frontend diagnostics-refresh assertion after enable/disable.
