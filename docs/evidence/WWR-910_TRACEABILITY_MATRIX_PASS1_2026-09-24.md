# WWR-910 — original Wake Word TODO traceability matrix, pass 1

Date: 2026-09-24
Base master: `7b538933b963e3cf8d1a8d3c2dc53de56470f6f6`
Canonical remediation TODO: `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`
Original TODO under reconciliation: `docs/WAKE_WORD_V1_TODO_2026-09-14.md`

## Purpose

This pass is a mechanical traceability matrix, not final closeout. It records which original Wake Word V1 areas already have objective merged evidence and which areas must remain open until production lifecycle wiring, real corpus/native KWS acceptance, performance evidence, specialized gate enforcement, and final exact-head/exact-master closeout are complete.

No original TODO item should be marked complete merely because it appears in this matrix. Completion still requires the source/evidence/run IDs named below and, for acceptance sections, passing the exact required gates on the final qualified head.

## Current exact-master baseline

- `master` SHA: `7b538933b963e3cf8d1a8d3c2dc53de56470f6f6`.
- Exact-master ordinary CI after WWR-900 audit pass: run `35985079161`, passed.
- Exact-master WWR-800 gate-coverage merge at `145bf21d9baeb2b5f58fb50ae079e3094d11f4c6` passed ordinary CI run `35983938327`, Wake lifecycle stability run `35983938243`, Wake source-security audit run `35983938252`, and Wake required-gates audit run `35983938148`.

## Traceability matrix

| Original area | Current remediation mapping | Objective evidence | Status for WWR-910 |
| --- | --- | --- | --- |
| WW-000 baseline / scope preservation | WWR-000 | `docs/evidence/WWR-000_REMEDIATION_BASELINE_2026-09-17.md` | Reconciled for baseline evidence. |
| Settings validation and fixed phrase | WWR-010, WWR-030, WWR-500 | PR #169 / CI `35254726268`; WWR-030 evidence; Settings UI checks in WWR-500 | Reconciled for live-write defect and V1 fixed phrase behavior. |
| Duplicate runtime/module consolidation | WWR-020 | `docs/evidence/WWR-020_WAKE_WORD_ARCHITECTURE_CONSOLIDATION_2026-09-17.md`, PR #172 / CI `35268110856` | Reconciled for architecture consolidation; final WWR-900 still audits no reintroduction. |
| Canonical V1 KWS policy constants | WWR-030 | `docs/evidence/WWR-030_CANONICAL_KWS_POLICY_2026-09-17.md`, PR #174 / CI `35281663768` | Reconciled for static policy and deterministic fake/session validation. |
| Model artifact identities and provenance | WWR-100 | `docs/evidence/WWR-100_MODEL_IDENTITY_2026-09-17.md`, artifact/model freeze runs `35289861279` and `35289861291` | Reconciled for identities/provenance/license; real inference acceptance remains under WWR-610/620. |
| Sherpa runtime identities and packaging | WWR-110 | `docs/evidence/WWR-110_SHERPA_RUNTIME_IDENTITY_2026-09-17.md`, runtime identity run `35294590820`, PR #184 head gates | Reconciled for pinned runtime identity/architecture/package layout; real inference acceptance remains under WWR-610/620. |
| Artifact CI coverage | WWR-120 | `docs/evidence/WWR-120_ARTIFACT_CI_COVERAGE_2026-09-17.md`, runs `35298008399`, `35298008347`, `35298008322`, `35298008340` | Reconciled for artifact/freezer coverage. |
| Native KWS session implementation | WWR-200 | `src-tauri/src/app/wake_word_engine.rs` implementation evidence in remediation TODO | Partially reconciled: implementation exists, but real positive/negative fixture acceptance and production composition remain open. |
| PCM validation ordering | WWR-210 | PR #187 / CI `35303924318`, PR #189 / CI `35317569091` | Reconciled for validation-before-retention/feed behavior. |
| Microphone ownership / routing | WWR-300 | `docs/evidence/WWR-300_AUDIO_CAPTURE_OWNERSHIP_AUDIT_2026-09-19.md`, `docs/evidence/WWR-300_APP_STATE_CAPTURE_COMPOSITION_2026-09-22.md`, `docs/evidence/WWR-300_CAPTURE_RECOVERY_CYCLES_2026-09-22.md` | Partially reconciled: single-owner infrastructure and deterministic tests exist; production startup wiring and real acceptance remain open. |
| Wake to command-ASR handoff | WWR-310 | `docs/evidence/WWR-310_HANDOFF_BOUNDARY_2026-09-22.md`, PR #377 / CI `35759517089` | Partially reconciled: sample-order and one-shot handoff evidence exists; full normal command-ASR activation and real/reproducible audio acceptance remain open. |
| Lifecycle integration and no barge-in | WWR-400, WWR-410 | `docs/evidence/WWR-410_DEBOUNCE_TRIGGER_SEMANTICS_2026-09-22.md`, deterministic lifecycle stability evidence, conversation/standalone speech suspension tests | Partially reconciled: deterministic boundaries exist; production enabled-start, trigger-to-command, Talking/TTS end-to-end acceptance remain open. |
| Settings UI and diagnostics | WWR-500, WWR-510 | WWR-500 checks in remediation TODO; WWR-510 evidence files for privacy-safe diagnostics and error/log audit | Reconciled for UI/disclosure/diagnostic schema; WWR-630 still owns accepted measured performance values. |
| Corpus and acceptance harness | WWR-600 | PR #395 real KWS acceptance branch evidence exists but is not merged; exact-head real KWS acceptance passed on `1399dca15cd30073e13957b021b72d5ee2b7f0d5` while ordinary CI remains blocked by the rustls lockfile audit failure | Open until the corpus/harness slice is merged and reconciled on master. |
| Linux x86_64 real KWS acceptance | WWR-610 | PR #395 exact-head Wake Word real KWS acceptance passed on `1399dca15cd30073e13957b021b72d5ee2b7f0d5` | Open until the qualified branch is mergeable and merged; do not infer master support from an unmerged branch. |
| macOS arm64 real KWS acceptance | WWR-620 | PR #395 exact-head Wake Word real KWS acceptance passed on `1399dca15cd30073e13957b021b72d5ee2b7f0d5` with the platform-specific gate output | Open until the qualified branch is mergeable and merged; do not infer master support from an unmerged branch. |
| Performance evidence | WWR-630 | Diagnostics placeholders only in WWR-510 evidence; PR #395 acceptance output can inform later performance evidence after merge | Open. Placeholder schema is not performance acceptance. |
| Integrated lifecycle stability acceptance | WWR-640 | `docs/evidence/WWR-640_DETERMINISTIC_RUNTIME_CYCLE_STABILITY_2026-09-22.md`; Wake lifecycle stability gate passed on exact master `145bf21d9baeb2b5f58fb50ae079e3094d11f4c6` after WWR-800 path-filter expansion | Partially reconciled: deterministic layer and gate path are qualified; real integrated acceptance/soak/resource accounting remains open. |
| Documentation | WWR-700 | `docs/WAKE_WORD_V1.md`, `docs/evidence/WWR-700_DOCUMENTATION_AUDIT_2026-09-23.md` | Partially reconciled: current docs avoid overclaiming; README/user-facing feature-available claims must wait for production and native acceptance. |
| Specialized Wake gates | WWR-800 | `docs/evidence/WWR-800_WAKE_GATE_INVENTORY_2026-09-22.md`, `docs/evidence/WWR-800_CONVERSATION_PATH_GATE_COVERAGE_2026-09-24.md`, PR #423 exact-head and exact-master CI/gates | Partially reconciled: conversation path-filter hole closed; actual missing gate definitions/enforcement remain open. |
| Final source/privacy/security audit | WWR-900 | `docs/evidence/WWR-900_SOURCE_PRIVACY_AUDIT_PASS1_2026-09-24.md` | Partially reconciled: pass 1 exists; final audit must be rerun after production/corpus/gate changes. |
| Final exact-head and exact-master qualification | WWR-950/960 | Ordinary exact-head/exact-master CI evidence across merged slices | Open. Final qualification cannot close until all required Wake gates exist and pass on the final head and merged master. |

## Original TODO areas that must remain explicitly open

The following original TODO areas cannot be marked complete in the original file during this pass:

1. Any item that requires real KWS positive/negative fixture behavior on merged master.
2. Any item that requires Linux x86_64 or macOS arm64 support claims from merged real inference evidence.
3. Any item that requires deterministic corpus recall/false-trigger measurement.
4. Any item that requires performance claims, idle CPU/memory baselines, or KWS-vs-full-ASR comparison.
5. Any item that requires end-to-end production startup/listening when Wake Word is enabled.
6. Any item that requires full wake trigger to normal command-ASR activation and downstream ASR acceptance.
7. Any item that requires final required Wake gates to exist, run, and be enforced for merge eligibility.
8. Any item that requires final exact-head/exact-master closeout.

## Reconciliation rules for the final pass

Final WWR-910 reconciliation should:

- reload `docs/WAKE_WORD_V1_TODO_2026-09-14.md` from the final feature head;
- update original checkboxes only where the exact source/evidence/run IDs prove the requirement;
- annotate superseded items by pointing to the remediation spec rather than silently checking them;
- avoid using this traceability matrix as recursive evidence for implementation completion;
- keep WW-960/WW-970 final closeout evidence tied to the final guarded merge and exact-master validation.
