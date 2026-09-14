# AI Talking Moose — Local TTS Post-Review Hardening TODO

**Date:** 2026-09-14
**Companion spec:** `docs/LOCAL_TTS_POST_REVIEW_HARDENING_SPEC_2026-09-14.md`
**Reviewed baseline:** `05c5d8106cda6febe871ba7a4eb579c818be695a` (`master`)
**Status:** Complete — implementation, qualification, guarded merge, and exact-master verification finished 2026-09-14

## Final closeout evidence

- Implementation base: `542b47b26f7cd86f0d669821e1a5120b58197328`.
- Implementation PR: #118.
- Exact tested PR head: `2e238385ff7f47bd0eff8982be0408773d7742f4`.
- PR-head CI `34872268664` — PASS.
- PR-head production CPU acceptance `34872268709` — PASS.
- PR-head ASR intelligibility smoke `34872268631` — PASS.
- PR-head ASR voice audition `34872268820` — PASS.
- Guarded squash merge produced `master` `cfd32b06f106829a209a053891d155e822e58a10`.
- Post-merge CI `34873555675` — PASS.
- Post-merge production CPU acceptance `34873555672` — PASS on Linux x86_64 and macOS arm64.
- Post-merge ASR intelligibility smoke `34873555704` — PASS.
- Post-merge ASR voice audition `34873555711` — PASS.
- Dedicated pinned-model waveform evidence branch head: `7a776e88c585d700ffdcd6804df8e346726b5909` (disposable evidence branch; not merged).
- Waveform evidence run `34876293975` — PASS on Linux x86_64 and macOS arm64.
- Pinned ONNX output metadata observed on both platforms:
  - `waveform`: `Tensor { ty: Float32, shape: [-1], dimension_symbols: ["num_samples"] }`.
  - `duration`: `Tensor { ty: Int64, shape: [-1], dimension_symbols: ["Castduration_dim_0"] }`.
- Real runtime waveform tensor shape observed: Linux `[76200]`; macOS `[75600]`.
- Path A was therefore selected for LTR-500: production accepts mono `[N]` and single-batch mono `[1, N]`, rejects empty/multi-batch/higher-rank/mismatched shapes, and keeps `duration` presence-only.
- This TODO reconciliation is intentionally non-recursive: the docs-only closeout PR that carries these final checkmarks is not required to record its own merge SHA inside itself.


This TODO is the finite implementation queue created from the 2026-09-14 Local KittenTTS code review. It must not reopen already-qualified Local TTS subsystems unless a task below identifies a concrete reason.

Task IDs use the `LTR-###` prefix (**Local TTS Review Remediation**).

---

## LTR-000 — Freeze scope and reviewed baseline

- [x] Confirm implementation starts from reviewed master `05c5d8106cda6febe871ba7a4eb579c818be695a` or a later verified descendant.
- [x] Record the actual implementation base SHA before the first source change.
- [x] Keep this batch limited to the review findings in the companion spec.
- [x] Do not change KittenTTS model/runtime/G2P pins unless a new defect requires it and is separately documented.
- [x] Do not change provider fallback policy.
- [x] Do not change the Local 2-thread production policy.
- [x] Do not claim subjective listening evidence.
- [x] Do not re-open `KCR-330` / `KTT-805` merely because the V1 decision used owner-approved ASR proxy evidence.

**Acceptance**

- [x] Final PR scope is traceable to this TODO/spec.
- [x] Any newly discovered out-of-scope defect is documented separately instead of being silently folded into this batch.

---

## LTR-100 — Repair the Local voice restart-policy regression test

- [x] Locate the restart-policy test that starts from `AppSettings::default()` and assigns `next.local_tts_voice = "Luna"`.
- [x] Replace the no-op assignment with a valid non-default Local voice such as `Leo`.
- [x] Add an explicit assertion that `previous.local_tts_voice != next.local_tts_voice`.
- [x] Confirm both voice IDs are valid Local KittenTTS catalog values.
- [x] Preserve the intended assertion that changing Local standalone voice alone does not require a conversation restart.
- [x] Ensure the test does not accidentally exercise Google standalone or Gemini Live voice state instead.
- [x] Review adjacent restart-policy tests for similar default-value false positives caused by later default changes.

**Acceptance**

- [x] The test fails if Local voice is accidentally added to conversation-restart criteria.
- [x] The test cannot pass by assigning the current default to itself.
- [x] Complete Rust tests remain green.

---

## LTR-200 — Reconcile the post-closeout hardening SPEC with current Luna/ASR-proxy state

Target:

`docs/LOCAL_TTS_POST_CLOSEOUT_HARDENING_SPEC_2026-09-13.md`

- [x] Change stale status wording such as `Proposed implementation queue` to a truthful completed/historical status.
- [x] Preserve the fact that PR #111 was required to leave `KCR-330` / `KTT-805` open **at that time**.
- [x] Add the later chronology: PR #114 automated ASR audition, explicit owner approval, PR #115 Luna default selection.
- [x] State that `KCR-330` / `KTT-805` is closed for V1 by owner-approved ASR proxy evidence.
- [x] State that no subjective human listening claim is made.
- [x] State that future human listening may override Luna but is not remaining V1 closeout work.
- [x] Remove or contextualize statements that currently imply Bella is still provisional/default.
- [x] Update the completion definition so it no longer requires a currently-open human-only gate.

**Acceptance**

- [x] The spec remains historically intelligible.
- [x] The spec does not contradict current `docs/VOICE_SELECTION.md` or PR #115 behavior.

---

## LTR-210 — Reconcile the post-closeout hardening TODO with current Luna/ASR-proxy state

Target:

`docs/LOCAL_TTS_POST_CLOSEOUT_HARDENING_TODO_2026-09-13.md`

- [x] Preserve completed PR #111 engineering evidence.
- [x] Update the introductory KCR-330/KTT-805 paragraph to explain original deferral plus later V1 ASR-proxy closeout.
- [x] Reconcile LTH-000 wording that says not to mark KCR-330/KTT-805 complete.
- [x] Reconcile LTH-500 from a current open-human-only gate to a historical boundary that PR #111 correctly preserved before later owner approval.
- [x] Update final checklist wording so it no longer says KCR-330/KTT-805 remains open.
- [x] Update intentionally-out-of-scope wording so future subjective audition is described as an optional override/evidence expansion rather than remaining V1 work.
- [x] Cross-link the automated ASR audition and Luna default closeout documents.

**Acceptance**

- [x] Every checkbox continues to describe work that actually occurred.
- [x] Historical LTH acceptance is not retroactively falsified.
- [x] Current-state summary agrees with Luna/ASR-proxy closeout.

---

## LTR-220 — Reconcile remaining current-looking KittenTTS closeout/spec documentation

Known review target:

`docs/KITTENTTS_CLOSEOUT_REMEDIATION_SPEC_2026-09-12.md`

- [x] Search the repository for `KCR-330`.
- [x] Search the repository for `KTT-805`.
- [x] Search the repository for `Bella`.
- [x] Search the repository for `Luna`.
- [x] Search the repository for `human audition`.
- [x] Search the repository for `human-only`.
- [x] Search the repository for `owner-only`.
- [x] Search the repository for `provisional`.
- [x] Search the repository for `remains open`.
- [x] Search the repository for `default voice`.
- [x] Classify every relevant hit as current/accurate, historical/preserved, stale/update-required, or historical/requires-banner.
- [x] Update stale current-state statements.
- [x] Add a superseded/current-state banner where historical text is too easy to mistake for current status.
- [x] Preserve historical evidence and dates rather than rewriting them to pretend the later decision existed earlier.

**Acceptance**

- [x] All current-state docs agree that Luna is the V1 Local default.
- [x] All current-state docs agree KCR-330/KTT-805 is closed for V1 by owner-approved ASR proxy.
- [x] No current-state doc claims subjective human listening occurred.

---

## LTR-300 — Finish ASR-proxy default-voice closeout bookkeeping

Target:

`docs/LOCAL_TTS_ASR_PROXY_DEFAULT_VOICE_CLOSEOUT_2026-09-14.md`

- [x] Record/check PR #115 exact head `4a8bf77e4803b7898c717a3cbe145f9df9600c75`.
- [x] Record/check PR-head CI `34853091162` — PASS.
- [x] Record/check PR-head production CPU acceptance `34853091150` — PASS.
- [x] Record/check PR-head ASR intelligibility smoke `34853091234` — PASS.
- [x] Record/check PR-head ASR voice audition `34853091230` — PASS.
- [x] Record/check guarded merge of PR #115.
- [x] Record/check merged master `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4`.
- [x] Record/check post-merge CI `34854193904` — PASS.
- [x] Record/check post-merge production CPU acceptance `34854193873` — PASS.
- [x] Record/check post-merge ASR intelligibility smoke `34854193869` — PASS.
- [x] Record/check post-merge ASR voice audition `34854193968` — PASS.
- [x] Preserve explicit wording that the decision is ASR-proxy based, not subjective human listening.

**Acceptance**

- [x] No completed evidence item remains falsely unchecked.
- [x] No evidence item is checked without exact supporting evidence.

---

## LTR-400 — Freeze and document persisted Bella versus new Luna default policy

Unless the owner explicitly changes the policy during this task, implement the following:

> Luna is the default for new profiles and for settings where `local_tts_voice` is absent. Existing valid persisted Local voice choices, including Bella, remain preserved as user preference and are not silently migrated.

- [x] Verify current `AppSettings::default()` uses `Luna`.
- [x] Verify current settings load preserves an existing valid persisted `Bella` value.
- [x] Add/update a regression test proving persisted Bella remains Bella after load/normalization.
- [x] Add/update a regression test proving an applicable missing Local voice field receives current default Luna.
- [x] Ensure tests distinguish defaulting from forced migration.
- [x] Do not bump settings schema solely to force Bella → Luna.
- [x] Do not infer whether persisted Bella was explicit user choice versus old default.
- [x] Document the policy in the most appropriate settings/voice documentation.
- [x] Confirm unknown invalid Local voice handling remains fail-safe under existing settings policy.

**Acceptance**

- [x] New/default-on-missing Local voice is Luna.
- [x] Existing valid persisted Bella remains Bella.
- [x] No provider fallback or unrelated migration behavior changes.

---

## LTR-500 — Decide whether to strengthen ONNX waveform rank/shape validation

- [x] Inspect the pinned production KittenTTS ONNX output metadata for `waveform`.
- [x] Inspect the `ort` extraction API shape returned by the existing `try_extract_tensor::<f32>()` path.
- [x] Record the actual real-model waveform rank/shape observed under acceptance.
- [x] Decide whether the expected shape is stable enough to enforce.

Choose one path:

### Path A — Strengthen

- [x] Define a small pure predicate/helper for acceptable waveform tensor shape.
- [x] Validate rank/batch/channel dimensions before flattening samples.
- [x] Preserve `f32`, nonempty, and finite-sample checks.
- [x] Map shape mismatch to sanitized Local TTS inference failure.
- [x] Add ordinary unit tests for accepted/rejected shapes.
- [x] Preserve `duration` as presence-only drift detection unless a separate requirement changes it.
- [x] Run real production CPU acceptance on Linux x86_64 and macOS arm64.

### Path B — Defer with evidence

- [x] N/A — Path A was selected; Record why pinned artifact identity plus current `f32`/nonempty/finite validation is the accepted V1 boundary.
- [x] N/A — Path A was selected; Record why enforcing tensor rank/shape would create a brittle or unsupported dependency.
- [x] N/A — Path A was selected; Add clarifying code/doc comment if needed.
- [x] N/A — Path A was selected; Do not pretend shape validation exists.

**Acceptance**

- [x] Exactly one path is selected and documented.
- [x] The runtime contract is more explicit after this task than before it.
- [x] No waveform/sample-rate semantics are changed accidentally.

---

## LTR-600 — Run final source audit

- [x] Audit `src-tauri/src/app/settings_policy.rs`.
- [x] Audit settings defaults/migration/normalization in `src-tauri/src/app/state.rs`.
- [x] Audit `src-tauri/src/ai/local_tts.rs`.
- [x] Audit `src-tauri/src/ai/local_tts/runtime/engine.rs` if LTR-500 changes it.
- [x] Audit Local TTS tests changed by LTR-100/LTR-400/LTR-500.
- [x] Audit `docs/VOICE_SELECTION.md`.
- [x] Audit the reconciled post-closeout hardening SPEC/TODO.
- [x] Audit the ASR-proxy closeout document.
- [x] Audit the KittenTTS closeout remediation spec/reconciliation/evidence docs.
- [x] Confirm no Local-to-Google fallback.
- [x] Confirm no Google-to-Local fallback.
- [x] Confirm Google/Local/Live voice ownership remains separate.
- [x] Confirm no utterance/audio/credential/raw-path leakage was introduced.
- [x] Confirm no model/runtime/G2P pins changed unintentionally.
- [x] Confirm no Local thread-policy change.
- [x] Confirm no stale current-state Bella/provisional/open-human-gate wording remains in authoritative docs.
- [x] Confirm no false subjective-listening claim.

**Acceptance**

- [x] Final diff contains only intended remediation.
- [x] No new mandatory defect remains unresolved in this scope.

---

## LTR-700 — Exact-head validation

### Always required

- [x] Review exact final branch diff against current `master`.
- [x] Confirm docs/source changes match this TODO.
- [x] Pass exact PR-head ordinary CI.
- [x] Record exact PR-head SHA.
- [x] Record exact PR-head CI run ID.

### Required when Rust source/tests change

- [x] Rust formatting passes.
- [x] Clippy passes.
- [x] Complete Rust tests pass.
- [x] Settings/restart regression tests pass.
- [x] Generated backend contract validation passes if contract-relevant settings/default source changed.

### Required when Local TTS runtime engine changes

- [x] KittenTTS production CPU acceptance passes on exact PR head.
- [x] Linux x86_64 real inference passes.
- [x] macOS arm64 real inference passes.
- [x] ASR intelligibility smoke passes if waveform/output behavior can plausibly change.
- [x] ASR voice audition is rerun only if voice/output/audition behavior is actually affected.

**Acceptance**

- [x] No stale or partially qualified head is eligible for merge.
- [x] Expensive workflows are required by diff scope, not rerun merely for evidence recursion.

---

## LTR-800 — Guarded merge and exact-master verification

- [x] Recheck PR mergeability immediately before merge.
- [x] Merge only with the exact tested PR head SHA.
- [x] Use an allowed guarded merge method.
- [x] Record exact merged `master` SHA.
- [x] Verify ordinary CI on exact merged `master`.
- [x] Verify production CPU acceptance on exact merged `master` if runtime-engine changes required it pre-merge.
- [x] Verify ASR smoke/audition post-merge only when required by the final source diff.
- [x] Update this TODO with final evidence without starting an evidence-only recursion loop.

**Acceptance**

- [x] `master` contains the remediation.
- [x] Exact merged-master validation evidence is recorded.

---

## Final checklist

- [x] Restart-policy Local voice test performs a real voice change.
- [x] Post-closeout hardening SPEC reconciled to current Luna/ASR-proxy state.
- [x] Post-closeout hardening TODO reconciled to current Luna/ASR-proxy state.
- [x] Remaining current-looking KittenTTS closeout/spec docs reconciled or clearly marked historical.
- [x] ASR-proxy default-voice closeout evidence bookkeeping completed.
- [x] Persisted Bella versus new Luna default behavior explicitly documented and tested.
- [x] Waveform shape contract deliberately strengthened or explicitly deferred with evidence.
- [x] Repository-wide stale-wording audit completed.
- [x] Provider separation preserved.
- [x] No-fallback behavior preserved.
- [x] Privacy/logging boundaries preserved.
- [x] Artifact pins/runtime/G2P revisions unchanged unless separately justified.
- [x] Exact PR-head required validation passed.
- [x] Guarded merge completed from exact tested head.
- [x] Exact merged-master required validation passed.
