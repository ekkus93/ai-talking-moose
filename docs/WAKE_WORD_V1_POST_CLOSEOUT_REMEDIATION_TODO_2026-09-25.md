# Wake Word V1 Post-Closeout Remediation TODO

**Date:** 2026-09-25
**Final reconciliation:** 2026-09-28
**Status:** Closed on `master`
**Final evidence commit:** `b8e23d7c0fca0fa15035e07548cf1d13b1e52ec2`
**Exact final qualification / exact-master gate head:** `bba5ea44a668f67baf32f0471dff2d75bb72cf22`
**Spec:** `docs/WAKE_WORD_V1_POST_CLOSEOUT_REMEDIATION_SPEC_2026-09-25.md`
**Source review:** post-closeout code review of `master` at `0ea5e03f9012884e2858d7b478fde916f0f163d7`

This document is the reconciled closeout state for the Wake Word V1 post-closeout remediation. The detailed historical checklist remains available in Git history before this final reconciliation commit; this version records the objective final status, exact evidence, and exact-head / exact-master validation used for closeout.

## Final closeout summary

- [x] Post-closeout baseline was reopened, frozen, and documented.
- [x] One native listener control plane owns listener lifecycle transitions.
- [x] Settings enable/disable, rollback, diagnostics, and UI refresh are wired to real listener ownership.
- [x] Input-device and ASR-mode changes while Wake is enabled are handled through the listener control plane.
- [x] Manual conversation shared-capture transfer is reliable and does not strand Wake in error/suspended state.
- [x] Wake V1 command-ASR policy is explicitly local-Moonshine-only.
- [x] Downstream first-command-word acceptance is covered at the deterministic boundary and in the real KWS command fixture.
- [x] Artifact provisioning is documented as developer-prepared and fail-closed for clean installs.
- [x] Production native listener performance evidence exists and distinguishes production-listener reports from standalone KWS-only reports.
- [x] Diagnostics and Settings UI truthfulness are covered, including listener ownership states and privacy boundaries.
- [x] Documentation was reconciled to the post-closeout support state.
- [x] Required CI gates were added/reconciled for reopened issues.
- [x] Final source/privacy/security audit was completed.
- [x] Exact-head final qualification passed.
- [x] Exact-master verification passed on the exact tested `master` head.

## WPCR section reconciliation

| WPCR section | Final status | Primary evidence |
| --- | --- | --- |
| WPCR-000 — Reopen and freeze post-closeout remediation baseline | Closed | `docs/evidence/WPCR-000_POST_CLOSEOUT_BASELINE_2026-09-25.md`; merged on `master` at `362ec673d0195fb360a3ffc32fd22ad616cdb169`; ordinary CI `36230401074`. |
| WPCR-100 — Build one native listener control plane | Closed | `docs/evidence/WPCR-100_LISTENER_SHUTDOWN_2026-09-26.md`; authoritative listener control/status evidence through exact `master` `e4606ee2f5a649d01468ed002d41beb09989da05`; ordinary CI `36341840706`, lifecycle stability `36341840594`, source-security audit `36341840687`. |
| WPCR-110 — Wire Settings enable/disable to real listener ownership | Closed | `docs/evidence/WPCR-110_120_SETTINGS_LISTENER_EVIDENCE_2026-09-27.md`, `docs/evidence/WPCR-110_SETTINGS_ROLLBACK_STATUS_2026-09-27.md`, `docs/evidence/WPCR-110_PERSISTENCE_ROLLBACK_2026-09-27.md`, `docs/evidence/WPCR-110_SETTINGS_UI_REFRESH_2026-09-28.md`; ordinary CI includes `36386721536` and `36387048117` for the final UI-refresh evidence path. |
| WPCR-120 — Handle input-device and ASR-mode settings changes while Wake is enabled | Closed | `docs/evidence/WPCR-110_120_SETTINGS_LISTENER_EVIDENCE_2026-09-27.md`; exact `master` `bf79159fb389bbe1961c83fcad1918aae43233af`; ordinary CI `36347933313`, lifecycle stability `36347933318`, source-security audit `36347933322`. |
| WPCR-200 — Fix manual conversation shared-capture transfer | Closed | `docs/evidence/WPCR-200_ACTIVE_LISTENER_TRANSFER_2026-09-27.md`, `docs/evidence/WPCR-200_MANUAL_TRANSFER_TERMINAL_RECOVERY_2026-09-27.md`; exact ordinary CI `36375659976`, source-security audit `36375659947`, evidence merge CI `36376132191`. |
| WPCR-300 — Resolve Wake command-ASR policy mismatch | Closed | Policy B selected: explicit local-Moonshine-only Wake V1. Enforced in `wake_word_state.rs`, Settings, UI disclosures, and docs. |
| WPCR-310 — Add downstream first-command-word acceptance | Closed | `docs/evidence/WPCR-310_FIRST_COMMAND_WORD_BOUNDARY_2026-09-26.md`, `docs/evidence/WPCR-310_COMMAND_FIXTURE_KWS_2026-09-27.md`; exact `master` `138384250a7133465e8b890701de4cbf3f93eb34`; ordinary CI `36377539641`, required-gates audit `36377539545`, real-KWS acceptance `36377539575`. |
| WPCR-400 — Add clean-install Wake artifact provisioning | Closed | `docs/evidence/WPCR-400_ARTIFACT_PROVISIONING_MODEL_2026-09-25.md`; developer-prepared artifact model, UI/docs disclosure, fail-closed clean app-data behavior, and manifest validation. |
| WPCR-500 — Measure production idle listener performance | Closed | `docs/evidence/WPCR-500_PRODUCTION_LISTENER_PERFORMANCE_2026-09-27.md`; exact `master` `5232ecf4fe7f9d20dd8de85f07526f43e8a48f64`; real KWS acceptance `36338240563`; evidence merge `7ebaf08c3a550801eb497432a7a0bb7f5bf32b86`; ordinary CI `36357695244`. |
| WPCR-600 — Fix diagnostics and Settings UI truthfulness | Closed | Listener ownership diagnostics and Settings status truthfulness merged through PR #469; exact-head ordinary CI, Wake source-security audit, Wake documentation audit, and Wake privacy audit passed at `47b368b23aeac9c4dfd7238d033304984d97e54d`. |
| WPCR-700 — Reconcile Wake documentation | Closed | `docs/evidence/WPCR-700_POST_CLOSEOUT_DOCUMENTATION_2026-09-26.md`; reopened-gate documentation updates through PR #479; exact-master ordinary CI `36303647425`, documentation audit `36303647388`, required-gates audit `36303647268`. |
| WPCR-800 — Add required CI gates for reopened issues | Closed | `docs/evidence/WPCR-800_REQUIRED_GATES_2026-09-26.md`; `docs/wake-word-required-gates.json`; final exact-head required-gates audit `36391804741`. |
| WPCR-900 — Final source/privacy/security audit | Closed | `docs/evidence/WPCR-900_FINAL_SOURCE_PRIVACY_SECURITY_AUDIT_2026-09-28.md`; audited `master` `c56e658bd60bbd187296f82c04da06824fb7649c`; ordinary CI `36389559764`. |
| WPCR-950 — Exact-head final qualification | Closed | `docs/evidence/WPCR-950_960_FINAL_QUALIFICATION_2026-09-28.md`; exact final qualification head `bba5ea44a668f67baf32f0471dff2d75bb72cf22`; all required exact-head gates passed. |
| WPCR-960 — Guarded merge and exact-master verification | Closed | `docs/evidence/WPCR-950_960_FINAL_QUALIFICATION_2026-09-28.md`; direct-to-`master` remediation, so no PR merge step was applicable; exact tested `master` `bba5ea44a668f67baf32f0471dff2d75bb72cf22` passed all required exact-master gates. |

## WPCR-950 exact-head final qualification

**Exact final qualification head:** `bba5ea44a668f67baf32f0471dff2d75bb72cf22`

This head changed only validation trigger comments, one narrow privacy-audit false-positive exception for `format!("{:?}", final_snapshot.phase)` in the acceptance-report path, and a lifecycle trigger note. It did not change Wake product behavior, model/runtime identities, corpus definitions, thresholds, scoring policy, listener behavior, Settings behavior, ASR/TTS behavior, or acceptance criteria.

### WPCR-950 tasks

- [x] Reload latest `master` before final qualification.
- [x] Review final branch diff against current `master`.
- [x] Confirm no unrelated ASR/TTS/settings regression is introduced.
- [x] Record exact final PR/head SHA: `bba5ea44a668f67baf32f0471dff2d75bb72cf22`.
- [x] Run ordinary CI at exact head: `36391804709`.
- [x] Run Settings/listener lifecycle gate at exact head: ordinary CI `36391804709`.
- [x] Run manual conversation shared-capture transfer gate at exact head: ordinary CI `36391804709`.
- [x] Run selected ASR policy acceptance at exact head: ordinary CI `36391804709`.
- [x] Run downstream first-command-word acceptance at exact head: ordinary CI `36391804709` plus real-KWS command fixture in `36391804703`.
- [x] Run clean-install artifact provisioning acceptance at exact head: native packaging `36391804824`, artifact verification `36391804721`.
- [x] Run production idle listener performance gate at exact head: real KWS / production-listener acceptance `36391804703`, performance evidence `36391804722`.
- [x] Run privacy/source-security audit at exact head: privacy audit `36391804693`, source-security audit `36391804817`.
- [x] Run documentation audit at exact head: `36391804809`.
- [x] Run required-gates audit at exact head: `36391804741`.
- [x] Record run IDs and report artifact names.

### WPCR-950 acceptance

- [x] Every required exact-head gate passed.
- [x] No skipped required gate was counted as passing.
- [x] Evidence is bound to the exact final head.

### Exact-head run IDs

| Gate | Run ID |
| --- | ---: |
| Ordinary CI | `36391804709` |
| Wake Artifact Verification | `36391804721` |
| Wake Word corpus validation | `36391804786` |
| Wake Word corpus contract | `36391804740` |
| Wake Word native packaging architecture | `36391804824` |
| Wake Word lifecycle stability | `36391804798` |
| Wake Word performance evidence | `36391804722` |
| Wake Word privacy audit | `36391804693` |
| Wake Word source security audit | `36391804817` |
| Wake Word documentation audit | `36391804809` |
| Wake Word required gates audit | `36391804741` |
| Wake Word real KWS acceptance | `36391804703` |

### Exact-head artifacts

From `Wake Word real KWS acceptance` run `36391804703`:

- `wake-word-v1-corpus` — artifact `10955834039`
- `wake-word-real-kws-linux-x86_64` — artifact `10956920657`
- `wake-word-real-kws-macos-arm64` — artifact `10956920850`

## WPCR-960 guarded merge and exact-master verification

This remediation was executed directly on `master` under the user's direct instruction. Therefore, PR mergeability and guarded merge substeps are not applicable. Exact-master verification is the exact tested `master` head verification recorded above.

**Exact tested master:** `bba5ea44a668f67baf32f0471dff2d75bb72cf22`

### WPCR-960 tasks

- [x] Recheck PR mergeability immediately before merge. Direct-to-`master`; no PR merge applicable.
- [x] Recheck exact head SHA immediately before merge. Direct-to-`master`; exact tested head recorded as `bba5ea44a668f67baf32f0471dff2d75bb72cf22`.
- [x] Merge only the exact tested head using an allowed guarded merge method. Direct-to-`master`; no PR merge applicable.
- [x] Record exact merged/tested master SHA: `bba5ea44a668f67baf32f0471dff2d75bb72cf22`.
- [x] Verify ordinary CI on exact merged/tested master: `36391804709`.
- [x] Verify all required Wake-specific exact-master gates: run IDs listed above.
- [x] Verify clean-install, listener lifecycle, ASR policy, downstream command, performance, privacy, source-security, docs, and required-gates evidence on exact merged/tested master.
- [x] Reconcile this TODO with exact evidence.
- [x] Do not close final status until all reopened code-review findings have implementation and acceptance evidence.

### WPCR-960 acceptance

- [x] `master` contains the complete post-closeout remediation.
- [x] Required exact-master validation evidence passes.
- [x] This TODO is reconciled with exact SHAs and run IDs.
- [x] Final docs truthfully describe Wake Word V1 support state.

## Final validation notes

- The final evidence file `docs/evidence/WPCR-950_960_FINAL_QUALIFICATION_2026-09-28.md` landed at `b8e23d7c0fca0fa15035e07548cf1d13b1e52ec2` and passed ordinary CI `36392549793`.
- This final TODO reconciliation is documentation-only. It records the exact final qualification and exact-master verification already completed on `bba5ea44a668f67baf32f0471dff2d75bb72cf22` and does not change product behavior, validation policy, artifact identity, corpus identity, or acceptance criteria.

## Final status

Wake Word V1 post-closeout remediation is closed. All reopened post-closeout findings are implemented, objectively qualified, reconciled in evidence, and merged to `master`.