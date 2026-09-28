# WPCR-110 — Settings UI diagnostics refresh evidence

Date: 2026-09-28
Exact master: `a2abb402a618bd281599d76b288d61dd1cb12654`
Ordinary CI: `36386721536` passed for exact `a2abb402a618bd281599d76b288d61dd1cb12654`

## Scope

This evidence records the frontend Settings UI proof for Wake enable/disable diagnostics refresh after the Settings write completes. It closes the previously open WPCR-110 UI-refresh assertion at the component-test level and complements the backend rollback/status evidence in:

- `docs/evidence/WPCR-110_120_SETTINGS_LISTENER_EVIDENCE_2026-09-27.md`
- `docs/evidence/WPCR-110_SETTINGS_ROLLBACK_STATUS_2026-09-27.md`
- `docs/evidence/WPCR-110_PERSISTENCE_ROLLBACK_2026-09-27.md`

## Production path

`src/components/Settings/WakeWordSettingsPanel.tsx` owns the Wake Word Settings panel. Its toggle handler calls `updateSettingsPatch` with the fixed Wake phrase and requested `wake_word_enabled` state. After that Settings write resolves, it calls `refreshDiagnostics()`, which reads `get_wake_word_diagnostics` through the Tauri bridge and updates the displayed runtime phase, listener ownership status, listener-active flag, and listening flag.

The UI therefore does not rely only on the optimistic patched Settings object after enable/disable. The panel refreshes backend diagnostics after the save boundary before presenting the post-toggle listener/runtime status.

## Test evidence

`src/components/Settings/WakeWordSettingsPanel.test.tsx` now includes deterministic frontend tests for both directions:

- `refreshes diagnostics after enabling Wake Word completes` queues disabled diagnostics for the initial render and active/listening diagnostics after the Settings write. The test toggles Wake on, verifies the persisted fixed phrase/write payload, and then asserts the refreshed UI displays `Runtime: Listening locally`, `Listener: Active locally`, and the enabled/active listener message.
- `refreshes diagnostics after disabling Wake Word completes` queues active/listening diagnostics for the initial render and disabled/stopped diagnostics after the Settings write. The test toggles Wake off, verifies the persisted fixed phrase/write payload, and then asserts the refreshed UI displays `Runtime: Disabled`, `Listener: Stopped`, and stopped listener ownership help text.

These tests use the existing mocked Tauri IPC fixture through `@tauri-apps/api/core` and exercise the real component path rather than replacing the panel with patched frontend-only state.

## Boundaries

This evidence closes the WPCR-110 frontend diagnostics-refresh item and the Settings/listener ordinary-CI proof portion of WPCR-800. It does not by itself close WPCR-900, WPCR-950, or WPCR-960; those still require final audit evidence and exact-head/exact-master Wake-specific gates.
