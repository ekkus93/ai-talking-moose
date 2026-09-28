# WPCR-900 — Post-closeout audit working notes

Date: 2026-09-28
Inspected master: `5cc56a598b28697340575a2d4b263a1a9b4d65bb`
Ordinary CI: `36387048117` passed for exact `5cc56a598b28697340575a2d4b263a1a9b4d65bb`

## Scope

This is a WPCR-900 working audit note, not final WPCR-900 closeout evidence. It records the current source/privacy/security audit state after the Settings UI diagnostics-refresh proof landed. WPCR-900, WPCR-950, and WPCR-960 still require final exact-head and exact-master Wake-specific gates before closeout.

## Reopened finding coverage reviewed so far

| Reopened area | Current implementation/evidence state | Audit status |
| --- | --- | --- |
| Listener ownership and Settings transitions | Listener control plane and Settings/listener changes are covered by `docs/evidence/WPCR-110_120_SETTINGS_LISTENER_EVIDENCE_2026-09-27.md`, `docs/evidence/WPCR-110_SETTINGS_ROLLBACK_STATUS_2026-09-27.md`, `docs/evidence/WPCR-110_PERSISTENCE_ROLLBACK_2026-09-27.md`, and `docs/evidence/WPCR-110_SETTINGS_UI_REFRESH_2026-09-28.md`. | Covered for WPCR-110 implementation/test scope; still subject to final exact-head/exact-master audit gates. |
| Manual conversation transfer and restart paths | `docs/evidence/WPCR-200_ACTIVE_LISTENER_TRANSFER_2026-09-27.md` and `docs/evidence/WPCR-200_MANUAL_TRANSFER_TERMINAL_RECOVERY_2026-09-27.md` record exact-master ordinary CI plus Wake source-security evidence for transfer/recovery behavior. | Covered for current WPCR-200 checklist. |
| Wake-triggered command handoff and selected ASR policy | `docs/evidence/WPCR-310_FIRST_COMMAND_WORD_BOUNDARY_2026-09-26.md` and `docs/evidence/WPCR-310_COMMAND_FIXTURE_KWS_2026-09-27.md` record deterministic downstream boundary receipt and real-KWS command-fixture coverage. WPCR-300 policy B remains local-Moonshine-only. | Covered within the selected Policy B scope. |
| Clean-install artifact provisioning | `docs/evidence/WPCR-400_ARTIFACT_PROVISIONING_MODEL_2026-09-25.md` records the developer-prepared artifact model and clean-app-data fail-closed behavior. | Covered for the selected developer-prepared model; bundled/installer alternatives remain intentionally unselected. |
| Diagnostics and logs for microphone state truthfulness | WPCR-100/WPCR-600 evidence records listener-status classification and UI status truthfulness; WPCR-110 rollback/status evidence covers backend enable/disable status classification; WPCR-110 UI-refresh evidence covers Settings panel refresh after enable/disable completion. | Covered for implementation/test scope; still subject to final exact-head/exact-master audit gates. |
| Logs/errors/metrics privacy | WPCR-110 sanitized-error evidence, WPCR-310 report-scope evidence, WPCR-500 performance evidence, and prior privacy/source-security gates all preserve the no raw PCM/transcript/credential boundary. | Covered by existing evidence, subject to final exact-head/exact-master audit gates. |
| Performance report claim scope | `docs/evidence/WPCR-500_PRODUCTION_LISTENER_PERFORMANCE_2026-09-27.md` distinguishes production-listener, KWS-only, and continuous-ASR comparison scope. | Covered, subject to final exact-head/exact-master performance gate. |
| Documentation truthfulness | WPCR-700 evidence and documentation/required-gates audits record non-user-ready, developer-prepared, local-ASR-only status. | Covered, subject to final exact-head/exact-master docs and required-gates audits. |

## Settings UI refresh proof added

The previous mandatory open item was WPCR-110's frontend/UI diagnostics-refresh assertion after enable/disable completion. That gap is now covered by `docs/evidence/WPCR-110_SETTINGS_UI_REFRESH_2026-09-28.md`:

- `src/components/Settings/WakeWordSettingsPanel.test.tsx` now proves that enabling Wake refreshes diagnostics after the Settings write and displays `Runtime: Listening locally` plus `Listener: Active locally`.
- The same test file proves that disabling Wake refreshes diagnostics after the Settings write and displays `Runtime: Disabled` plus `Listener: Stopped`.
- Exact ordinary CI `36386721536` passed for the implementation/test commit `a2abb402a618bd281599d76b288d61dd1cb12654`.
- Exact ordinary CI `36387048117` passed for the evidence-note commit `5cc56a598b28697340575a2d4b263a1a9b4d65bb`.

## Remaining closeout work

The known implementation/test gaps identified by WPCR-110 and WPCR-800 are now covered by merged evidence, but final closeout is still not complete. Before WPCR-900 can be marked complete, the final audit evidence must be written and bound to final exact-head/exact-master gates. WPCR-950 and WPCR-960 still need the required ordinary CI, Settings/listener lifecycle coverage, manual transfer, ASR policy, downstream command, clean-install, performance, privacy/source-security, documentation, and required-gates evidence.

## WPCR-900 closeout boundary

Do not mark WPCR-900 complete from this working note alone. Final WPCR-900 evidence must list this working-note audit, WPCR-110 UI-refresh evidence, final source/privacy/security audits, final documentation audit, final required-gates audit, and all exact run IDs.
