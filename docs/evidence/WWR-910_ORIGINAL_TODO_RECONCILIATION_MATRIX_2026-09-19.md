# WWR-910 Original TODO Reconciliation Matrix

**Reconciliation head:** `669ee23ff6e364908dd05296770db85698c80b94`

This matrix reloads the original `docs/WAKE_WORD_V1_TODO_2026-09-14.md` and maps each original section to the remediation track. It is deliberately conservative: a section is not called complete when real corpus/platform/lifecycle/performance evidence is still missing.

| Original section | Remediation source/evidence | Status on this head |
| --- | --- | --- |
| WW-000 scope/baseline | WWR-000 evidence | Complete under remediation baseline; original baseline remains traceable. |
| WW-100 model/runtime artifacts | WWR-100 + WWR-110 evidence | Immutable model and runtime identities/provenance implemented. |
| WW-110 runtime packaging | WWR-110 + WWR-120 evidence | Deterministic preparation, architecture/hash checks and artifact CI implemented; final packaged real-platform acceptance remains part of WWR-610/620/800. |
| WW-200 PCM ring buffer | preserved by WWR-000; WWR-210 validation ordering | Component implemented/tested and retained. |
| WW-300 Sherpa KWS engine | WWR-030 + WWR-100/110 + WWR-200 | Real native adapter and frozen policy implemented; real positive/negative fixture acceptance remains open. |
| WW-310 WakeWordRuntimeManager | WWR-020 + WWR-400/410 | One authoritative manager and deterministic lifecycle/debounce tests exist; full production acceptance remains open. |
| WW-400 microphone routing | WWR-300 audit | Ownership strategy audited; production one-stream/handoff acceptance remains open. |
| WW-410 pre-roll handoff | WWR-310 | Component handoff and clearing semantics exist; real continuous utterance/first-word acceptance remains open. |
| WW-500 persisted settings | WWR-010 + WWR-030 | Canonical validation, disabled default, fixed phrase and live update defect remediation implemented. |
| WW-510 Settings UI | WWR-500 evidence/current-behavior doc | UI exists with fixed phrase, toggle, disclosures and runtime status; final TODO checkbox reconciliation still pending. |
| WW-600 conversation lifecycle | WWR-400 | Deterministic lifecycle behavior is implemented/tested for key states; full integrated production lifecycle acceptance remains open. |
| WW-610 debounce | WWR-410 | One-trigger state-machine behavior and privacy-safe trigger diagnostics implemented/tested; final integrated acceptance remains open. |
| WW-700 diagnostics | WWR-510 + `docs/evidence/WWR-900_SOURCE_PRIVACY_AUDIT_2026-09-19.md` | Privacy-safe diagnostic type covers required state/identity/counters/timing without representable raw PCM. |
| WW-710 performance | WWR-630 | Open: measured representative performance baseline is not yet recorded. |
| WW-800 corpus | WWR-600 + deterministic corpus workflow | Manifest/schema/version/privacy/provenance gate exists; real positive/negative/near-miss fixtures and calibrated thresholds remain open. |
| WW-810 real platform acceptance | WWR-610 + WWR-620 | Open: Linux x86_64 and macOS arm64 real KWS acceptance still required. |
| WW-820 lifecycle stability | WWR-640 | Open: repeated integrated/soak/resource acceptance still required. |
| WW-900 docs | WWR-700 + `docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md` | Current behavior and limitations are documented truthfully; README/user-ready closeout remains deferred until feature acceptance. |
| WW-950 final audit | WWR-900 | Source/privacy audit recorded, but final acceptance remains open pending real platform/lifecycle/performance work. |
| WW-960 exact-head qualification | WWR-950 | Open until all required Wake-specific gates exist and pass on one final head. |
| WW-970 guarded merge | WWR-960 | Open until final qualification head exists. |

## Supersession rules

The remediation specification/TODO supersedes contradictory or underspecified implementation details in the original queue. In particular:

- the canonical V1 phrase is fixed to `Hey, Moose` and arbitrary phrase editing is not a V1 feature;
- score/threshold/thread/sample policy is frozen by WWR-030 rather than exposed as a generic sensitivity surface;
- one authoritative Wake subsystem replaces the duplicate-stack shape found during review;
- documentation/component tests are never substituted for real positive/negative KWS acceptance;
- ordinary CI is necessary but insufficient for final Wake qualification.

## Evidence pointers

Primary remediation evidence already merged includes:

- `docs/evidence/WWR-000_REMEDIATION_BASELINE_2026-09-17.md`
- `docs/evidence/WWR-020_WAKE_WORD_ARCHITECTURE_CONSOLIDATION_2026-09-17.md`
- `docs/evidence/WWR-030_CANONICAL_KWS_POLICY_2026-09-17.md`
- `docs/evidence/WWR-100_MODEL_IDENTITY_2026-09-17.md`
- `docs/evidence/WWR-110_SHERPA_RUNTIME_IDENTITY_2026-09-17.md`
- `docs/evidence/WWR-120_ARTIFACT_CI_COVERAGE_2026-09-17.md`
- `docs/evidence/WWR-300_AUDIO_CAPTURE_OWNERSHIP_AUDIT_2026-09-19.md`
- `docs/evidence/WWR-700_800_RECONCILIATION_2026-09-19.md`
- `docs/evidence/WWR-900_SOURCE_PRIVACY_AUDIT_2026-09-19.md`
- `docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md`
- `docs/WAKE_WORD_V1_CI_GATES.md`

## Remaining original-TODO closeout

The original TODO itself remains intentionally unchecked rather than being mass-marked from design documentation. Final WWR-910 closeout must update original checkboxes only after the corresponding implementation/acceptance evidence is complete, especially WW-400/410, WW-710, WW-800/810/820, WW-950, WW-960, and WW-970.
