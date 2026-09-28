# WPCR-900 — Post-closeout audit working notes

Date: 2026-09-28
Inspected master: `50e8b540749ab119de7afddaaab2eeccf0ed7453`
Ordinary CI: `36386091810` passed for exact `50e8b540749ab119de7afddaaab2eeccf0ed7453`

## Scope

This is a WPCR-900 working audit note, not final WPCR-900 closeout evidence. It records the current source/privacy/security audit state and the remaining mandatory gap that still prevents WPCR-900, WPCR-950, and WPCR-960 from closing.

## Reopened finding coverage reviewed so far

| Reopened area | Current implementation/evidence state | Audit status |
| --- | --- | --- |
| Listener ownership and Settings transitions | Listener control plane and Settings/listener changes are covered by `docs/evidence/WPCR-110_120_SETTINGS_LISTENER_EVIDENCE_2026-09-27.md`, `docs/evidence/WPCR-110_SETTINGS_ROLLBACK_STATUS_2026-09-27.md`, and `docs/evidence/WPCR-110_PERSISTENCE_ROLLBACK_2026-09-27.md`. | Partially covered; frontend diagnostics-refresh/UI assertion remains open. |
| Manual conversation transfer and restart paths | `docs/evidence/WPCR-200_ACTIVE_LISTENER_TRANSFER_2026-09-27.md` and `docs/evidence/WPCR-200_MANUAL_TRANSFER_TERMINAL_RECOVERY_2026-09-27.md` record exact-master ordinary CI plus Wake source-security evidence for transfer/recovery behavior. | Covered for current WPCR-200 checklist. |
| Wake-triggered command handoff and selected ASR policy | `docs/evidence/WPCR-310_FIRST_COMMAND_WORD_BOUNDARY_2026-09-26.md` and `docs/evidence/WPCR-310_COMMAND_FIXTURE_KWS_2026-09-27.md` record deterministic downstream boundary receipt and real-KWS command-fixture coverage. WPCR-300 policy B remains local-Moonshine-only. | Covered within the selected Policy B scope. |
| Clean-install artifact provisioning | `docs/evidence/WPCR-400_ARTIFACT_PROVISIONING_MODEL_2026-09-25.md` records the developer-prepared artifact model and clean-app-data fail-closed behavior. | Covered for the selected developer-prepared model; bundled/installer alternatives remain intentionally unselected. |
| Diagnostics and logs for microphone state truthfulness | WPCR-100/WPCR-600 evidence records listener-status classification and UI status truthfulness; WPCR-110 rollback/status evidence covers backend enable/disable status classification. | Partially covered; remaining frontend diagnostics refresh after enable/disable must be proven before final audit closure. |
| Logs/errors/metrics privacy | WPCR-110 sanitized-error evidence, WPCR-310 report-scope evidence, WPCR-500 performance evidence, and prior privacy/source-security gates all preserve the no raw PCM/transcript/credential boundary. | Covered by existing evidence, subject to final exact-head/exact-master audit gates. |
| Performance report claim scope | `docs/evidence/WPCR-500_PRODUCTION_LISTENER_PERFORMANCE_2026-09-27.md` distinguishes production-listener, KWS-only, and continuous-ASR comparison scope. | Covered, subject to final exact-head/exact-master performance gate. |
| Documentation truthfulness | WPCR-700 evidence and documentation/required-gates audits record non-user-ready, developer-prepared, local-ASR-only status. | Covered, subject to final exact-head/exact-master docs and required-gates audits. |

## Current mandatory open item

The remaining product-level gap is WPCR-110's frontend/UI diagnostics-refresh assertion after enable/disable completion:

- `UI refreshes diagnostics/status after enable/disable completes` remains open.
- `Existing Settings UI tests are updated to assert real backend state, not just patched frontend settings` remains open.
- WPCR-800's Settings/listener lifecycle coverage remains open to the extent it depends on that WPCR-110 UI proof.

A frontend rollback timing assertion was attempted on `master` but repeatedly failed the opaque `Frontend quality` gate with redacted job evidence. The known-green scaffold was restored at `50e8b540749ab119de7afddaaab2eeccf0ed7453`, and exact ordinary CI `36386091810` passed. This note therefore does not claim closure of the frontend UI assertion.

## WPCR-900 closeout boundary

Do not mark WPCR-900 complete until the remaining WPCR-110/WPCR-800 Settings UI proof is merged and exact-head/exact-master final gates pass. Final WPCR-900 evidence must list this working-note audit, the final UI proof, final source/privacy/security audits, final documentation audit, final required-gates audit, and all exact run IDs.
