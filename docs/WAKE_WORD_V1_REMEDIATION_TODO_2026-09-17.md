# AI Talking Moose — Wake Word V1 Remediation TODO

**Date:** 2026-09-17
**Final reconciliation:** 2026-09-25
**Status:** Closed on `master`
**Final master:** `99e216e13f78c1a0605684801d89dbd6b33ca7c9`
**Specification:** `docs/WAKE_WORD_V1_REMEDIATION_SPEC_2026-09-17.md`
**Original TODO:** `docs/WAKE_WORD_V1_TODO_2026-09-14.md`
**Review baseline:** `ai-talking-moose-wake-word-v1-code-review-2026-09-17.md`

This document is the reconciled closeout state for Wake Word V1 remediation. The detailed historical checklist remains available in Git history before the final closeout reconciliation commit; this version records the objective final status, exact evidence, and final PR-head / merged-master gate evidence.

## Final closeout summary

- [x] Wake Word V1 implementation is merged to `master`.
- [x] The production KWS engine is sherpa-onnx KWS with pinned model/runtime identities.
- [x] The fixed phrase is `Hey, Moose` / keyword `HEY MOOSE`.
- [x] Wake Word defaults disabled and remains user-toggleable from Settings.
- [x] Wake disabled behavior preserves manual interaction behavior.
- [x] Wake Word uses one authoritative runtime manager and one authoritative shared capture ownership model.
- [x] Wake Word does not open a competing full-time ASR stream and does not perform idle transcription.
- [x] Wake detection, pre-roll retention, command handoff, Talking suspension, resume paths, debounce, and lifecycle stability are covered by source tests and exact-head gates.
- [x] Raw Wake PCM remains bounded and memory-only; diagnostics and logs are privacy-safe.
- [x] Linux x86_64 and macOS arm64 real native KWS acceptance passed.
- [x] WWR-950 exact-head final qualification passed on PR #460 head `007859746292206734922ee2a1345d036f4f58fe`.
- [x] WWR-960 guarded merge and exact-master verification passed on merged master `99e216e13f78c1a0605684801d89dbd6b33ca7c9`.

## WWR section reconciliation

| WWR section | Final status | Primary evidence |
| --- | --- | --- |
| WWR-000 — Freeze remediation baseline and preserve known-good behavior | Closed | `docs/evidence/WWR-000_REMEDIATION_BASELINE_2026-09-17.md`. |
| WWR-010 — Fix live Wake Word settings validation defect | Closed | PR #169, exact CI `35254726268`; canonical validation through `WakeWordSettings::from_app_settings_fields`. |
| WWR-020 — Consolidate duplicate Wake Word module architecture | Closed | `docs/evidence/WWR-020_WAKE_WORD_ARCHITECTURE_CONSOLIDATION_2026-09-17.md`; PR #172 exact CI `35268110856`. |
| WWR-030 — Freeze one authoritative V1 KWS policy | Closed | `docs/evidence/WWR-030_CANONICAL_KWS_POLICY_2026-09-17.md`; exact CI `35281663768`; KittenTTS CPU acceptance `35281663889`. |
| WWR-100 — Complete model artifact identities and provenance | Closed | `docs/evidence/WWR-100_MODEL_IDENTITY_2026-09-17.md`; exact ordinary CI `35289861261`, artifact verification `35289861279`, model identity freeze `35289861291`. |
| WWR-110 — Complete sherpa native runtime identities and packaging | Closed | `docs/evidence/WWR-110_SHERPA_RUNTIME_IDENTITY_2026-09-17.md`; exact ordinary CI `35296408130`, artifact verification `35296408236`, model/runtime identity gates `35296408238` and `35296408308`. |
| WWR-120 — Expand artifact CI coverage | Closed | `docs/evidence/WWR-120_ARTIFACT_CI_COVERAGE_2026-09-17.md`; exact ordinary CI `35298008399`; Wake Artifact Verification `35298008347`. |
| WWR-200 — Implement the real native sherpa KWS session | Closed | `src-tauri/src/app/wake_word_engine.rs`; WWR-610/620 real KWS acceptance; source-security and final real-KWS gates. |
| WWR-210 — Fix PCM validation ordering | Closed | PR #187 exact CI `35303924318`; PR #189 exact CI `35317569091`; regression coverage for invalid PCM before retention/feed. |
| WWR-300 — Integrate one authoritative microphone routing path | Closed | `docs/evidence/WWR-300_AUDIO_CAPTURE_OWNERSHIP_AUDIT_2026-09-19.md`, `docs/evidence/WWR-300_APP_STATE_CAPTURE_COMPOSITION_2026-09-22.md`, `docs/evidence/WWR-300_CAPTURE_RECOVERY_CYCLES_2026-09-22.md`; final source-security gate `36177173925`. |
| WWR-310 — Complete production wake→ASR pre-roll/live handoff | Closed | `docs/evidence/WWR-310_HANDOFF_BOUNDARY_2026-09-22.md`; command activation and single-use handoff tests; final lifecycle gate `36177175140`. |
| WWR-400 — Integrate Wake Word with application lifecycle | Closed | WWR-400 source/tests, WWR-640 lifecycle evidence, WWR-900 audit, final lifecycle gate `36177175140`, final source-security gate `36177173925`. |
| WWR-410 — Finalize debounce/trigger semantics | Closed | `docs/evidence/WWR-410_DEBOUNCE_TRIGGER_SEMANTICS_2026-09-22.md`; final source-security gate `36177173925`. |
| WWR-500 — Implement Wake Word Settings UI | Closed | Settings UI/source tests, WWR-500 remediation evidence, documentation audit `36177173812`. |
| WWR-510 — Complete privacy-safe diagnostics | Closed | WWR-510 evidence files; privacy audit `36177173642`; source-security audit `36177173925`. |
| WWR-600 — Add deterministic Wake Word corpus and harness | Closed | `docs/evidence/WWR-600_DETERMINISTIC_CORPUS_ACCEPTANCE_2026-09-24.md`; final corpus gates `36177173861` and `36177173853`; final real-KWS gate `36177173829`. |
| WWR-610 — Add real Linux x86_64 sherpa KWS acceptance | Closed | `docs/evidence/WWR-610_620_REAL_KWS_ACCEPTANCE_2026-09-24.md`; final real-KWS gate `36177173829`. |
| WWR-620 — Add real macOS arm64 sherpa KWS acceptance | Closed | `docs/evidence/WWR-610_620_REAL_KWS_ACCEPTANCE_2026-09-24.md`; final real-KWS gate `36177173829`. |
| WWR-630 — Add performance evidence | Closed | `docs/wake-word-performance-evidence.json`; final performance gate `36177173634`. |
| WWR-640 — Add integrated lifecycle stability acceptance | Closed | PR #435 evidence, exact lifecycle `35999823097`, 100-cycle resource evidence `36090294124`, final lifecycle gate `36177175140`. |
| WWR-700 — Correct and complete documentation | Closed | `docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md`, `docs/WAKE_WORD_V1_CI_GATES.md`; final documentation audit `36177173812`. |
| WWR-800 — Add specialized Wake CI gates | Closed | `docs/wake-word-required-gates.json`, `docs/WAKE_WORD_V1_CI_GATES.md`; final required-gates audit `36177173738`. |
| WWR-900 — Final source/privacy/security audit | Closed | `docs/evidence/WWR-900_FINAL_SOURCE_PRIVACY_SECURITY_AUDIT_2026-09-25.md`; final source-security audit `36177173925`; final privacy audit `36177173642`; final documentation audit `36177173812`. |
| WWR-910 — Reconcile original 314-item TODO | Closed | `docs/evidence/WWR-910_ORIGINAL_TODO_RECONCILIATION_MATRIX_2026-09-25.md`; this final TODO reconciliation; WW-960/970 final evidence below. |
| WWR-950 — Exact-head final qualification | Closed | PR #460 head `007859746292206734922ee2a1345d036f4f58fe`; run IDs below. |
| WWR-960 — Guarded merge and exact-master verification | Closed | Guarded squash merge of PR #460 to `99e216e13f78c1a0605684801d89dbd6b33ca7c9`; exact-master run IDs below. |

## WWR-950 exact-head final qualification evidence

**Final PR:** #460

**Exact PR head:** `007859746292206734922ee2a1345d036f4f58fe`

### Diff / review

- [x] Reloaded latest `master` before final qualification branch creation: `201dabffcef346ec4dd85baafa67d8b37f531ba9`.
- [x] Reviewed final branch scope: evidence scaffold plus workflow-comment trigger updates only.
- [x] Confirmed no production runtime, ASR, TTS, manifest, corpus, model/runtime identity, threshold, or acceptance-criteria change in PR #460.
- [x] Confirmed duplicate Wake Word stacks are guarded by the exact-head source-security audit.
- [x] Confirmed no unrelated ASR/TTS regression was introduced by the final qualification slice.
- [x] Recorded exact final PR head SHA: `007859746292206734922ee2a1345d036f4f58fe`.

### Ordinary gates

- [x] Rust formatting, Clippy/lint, Rust tests, frontend checks, generated settings/backend contract, and artifact-related ordinary CI checks passed through ordinary CI `36176391100`.
- [x] Artifact Python tests passed through Wake Artifact Verification `36176391455`.

### Wake-specific gates

- [x] Deterministic corpus manifest gate passed: `36176391230`.
- [x] Corpus contract gate passed: `36176391062`.
- [x] Linux x86_64 and macOS arm64 real KWS gate passed: `36176391158`.
- [x] Native package/architecture gate passed: `36176391173`.
- [x] Integrated lifecycle stability gate passed: `36176390990`.
- [x] Performance evidence gate passed: `36176391188`.
- [x] Privacy audit passed: `36176391219`.
- [x] Source-security audit passed: `36176391127`.
- [x] Documentation audit passed: `36176391243`.
- [x] Required-gates audit passed: `36176391095`.

### Evidence details

- [x] Exact ordinary CI run ID recorded: `36176391100`.
- [x] Exact Wake-specific run IDs recorded above.
- [x] Exact artifact manifest revision unchanged by the final qualification PR; artifact verification passed at the exact PR head.
- [x] Exact corpus version unchanged by the final qualification PR; corpus and real-KWS gates passed at the exact PR head.
- [x] Acceptance platform details were recorded by real-KWS and native-packaging workflows at the exact PR head.

## WWR-960 guarded merge and exact-master verification evidence

**Guarded merge:** PR #460 squash-merged through Ralph Bridge with expected head `007859746292206734922ee2a1345d036f4f58fe`.

**Merged master:** `99e216e13f78c1a0605684801d89dbd6b33ca7c9`

- [x] Rechecked exact head SHA immediately before merge through the guarded merge call.
- [x] Merged only the exact tested PR head.
- [x] Used an allowed guarded squash merge method.
- [x] Recorded exact merged master SHA: `99e216e13f78c1a0605684801d89dbd6b33ca7c9`.
- [x] Verified ordinary CI on exact merged master: `36177173876`.
- [x] Verified exact-master Wake Artifact Verification: `36177173934`.
- [x] Verified exact-master deterministic corpus manifest gate: `36177173861`.
- [x] Verified exact-master corpus contract gate: `36177173853`.
- [x] Verified exact-master real Linux/macOS KWS acceptance: `36177173829`.
- [x] Verified exact-master native packaging/architecture gate: `36177173665`.
- [x] Verified exact-master integrated lifecycle stability gate: `36177175140`.
- [x] Verified exact-master performance evidence gate: `36177173634`.
- [x] Verified exact-master privacy audit: `36177173642`.
- [x] Verified exact-master source-security audit: `36177173925`.
- [x] Verified exact-master documentation audit: `36177173812`.
- [x] Verified exact-master required-gates audit: `36177173738`.
- [x] Verified final qualification PR did not change `wake-word-artifacts.json`; final artifact verification passed on both PR head and merged master.
- [x] Verified final qualification PR did not change `docs/wake-word-corpus.json`; final corpus and real-KWS gates passed on both PR head and merged master.
- [x] Original WW-960/WW-970 reconciliation is complete via `docs/evidence/WWR-910_ORIGINAL_TODO_RECONCILIATION_MATRIX_2026-09-25.md` plus this WWR-950/960 closeout evidence.
- [x] Final docs remain truthful; README promotion is intentionally deferred unless/until the project chooses to advertise Wake Word as user-ready.
- [x] Final closeout evidence is recorded without opening a new evidence-only implementation loop.

## Original TODO reconciliation

The original `docs/WAKE_WORD_V1_TODO_2026-09-14.md` is reconciled by `docs/evidence/WWR-910_ORIGINAL_TODO_RECONCILIATION_MATRIX_2026-09-25.md` and this final closeout section. Original WW-960 and WW-970 were intentionally left pending in the matrix until final exact-head and exact-master evidence existed; they are now closed by WWR-950/960 above.

## Final remediation checklist

### Architecture

- [x] One authoritative Wake Word subsystem exists.
- [x] One authoritative `WakeWordRuntimeManager` exists.
- [x] One authoritative KWS config/engine policy exists.
- [x] Duplicate legacy Wake Word stacks are removed / guarded against reintroduction.

### Settings/UI

- [x] Live settings validation cannot persist an invalid phrase.
- [x] Wake defaults disabled.
- [x] Phrase is fixed to `Hey, Moose`.
- [x] Settings UI can enable/disable Wake Word.
- [x] UI discloses local/offline KWS and active microphone behavior.
- [x] Manual behavior is preserved when Wake Word is disabled.

### Artifacts/engine

- [x] Model archive/files have immutable identities.
- [x] Native runtimes have immutable identities.
- [x] Model/runtime licenses and notices are verified.
- [x] Real native sherpa session loads exact verified inputs.
- [x] KWS uses 16 kHz mono, one thread, score 1.0, threshold 0.25.
- [x] KWS is local/offline during idle inference.

### Audio/handoff/lifecycle

- [x] One authoritative microphone ownership path exists.
- [x] PCM is validated before retention.
- [x] Ring buffer and KWS share one chronological canonical stream.
- [x] Wake→ASR pre-roll/live handoff is continuous within deterministic acceptance.
- [x] First command word is not clipped in deterministic acceptance.
- [x] One wake event creates one command interaction.
- [x] Wake is suspended while Moose talks.
- [x] Wake resumes after TTS success/cancellation/recoverable failure.
- [x] No V1 barge-in exists.

### Privacy/quality

- [x] Raw Wake PCM remains memory-only.
- [x] Diagnostics are privacy-safe.
- [x] No silent cloud/full-ASR fallback exists.
- [x] Corpus acceptance passes.
- [x] Linux real KWS acceptance passes.
- [x] macOS arm64 real KWS acceptance passes.
- [x] Lifecycle stability passes.
- [x] Performance baseline is recorded.
- [x] Documentation is truthful.

### Closeout

- [x] Artifact/freezer CI coverage is complete.
- [x] Exact PR-head required gates pass.
- [x] Guarded merge uses exact tested head.
- [x] Exact merged-master required gates pass.
- [x] Original 314-item TODO is reconciled.
- [x] No mandatory code-review finding remains open in the Wake Word V1 remediation scope.

## Final status

Wake Word V1 remediation is complete on `master` at `99e216e13f78c1a0605684801d89dbd6b33ca7c9` with exact PR-head and exact merged-master qualification evidence recorded above.
