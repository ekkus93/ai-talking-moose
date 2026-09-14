# AI Talking Moose — Local TTS Post-Review Hardening TODO

**Date:** 2026-09-14
**Companion spec:** `docs/LOCAL_TTS_POST_REVIEW_HARDENING_SPEC_2026-09-14.md`
**Reviewed baseline:** `05c5d8106cda6febe871ba7a4eb579c818be695a` (`master`)
**Status:** Planned remediation queue

This TODO is the finite implementation queue created from the 2026-09-14 Local KittenTTS code review. It must not reopen already-qualified Local TTS subsystems unless a task below identifies a concrete reason.

Task IDs use the `LTR-###` prefix (**Local TTS Review Remediation**).

---

## LTR-000 — Freeze scope and reviewed baseline

- [ ] Confirm implementation starts from reviewed master `05c5d8106cda6febe871ba7a4eb579c818be695a` or a later verified descendant.
- [ ] Record the actual implementation base SHA before the first source change.
- [ ] Keep this batch limited to the review findings in the companion spec.
- [ ] Do not change KittenTTS model/runtime/G2P pins unless a new defect requires it and is separately documented.
- [ ] Do not change provider fallback policy.
- [ ] Do not change the Local 2-thread production policy.
- [ ] Do not claim subjective listening evidence.
- [ ] Do not re-open `KCR-330` / `KTT-805` merely because the V1 decision used owner-approved ASR proxy evidence.

**Acceptance**

- [ ] Final PR scope is traceable to this TODO/spec.
- [ ] Any newly discovered out-of-scope defect is documented separately instead of being silently folded into this batch.

---

## LTR-100 — Repair the Local voice restart-policy regression test

- [ ] Locate the restart-policy test that starts from `AppSettings::default()` and assigns `next.local_tts_voice = "Luna"`.
- [ ] Replace the no-op assignment with a valid non-default Local voice such as `Leo`.
- [ ] Add an explicit assertion that `previous.local_tts_voice != next.local_tts_voice`.
- [ ] Confirm both voice IDs are valid Local KittenTTS catalog values.
- [ ] Preserve the intended assertion that changing Local standalone voice alone does not require a conversation restart.
- [ ] Ensure the test does not accidentally exercise Google standalone or Gemini Live voice state instead.
- [ ] Review adjacent restart-policy tests for similar default-value false positives caused by later default changes.

**Acceptance**

- [ ] The test fails if Local voice is accidentally added to conversation-restart criteria.
- [ ] The test cannot pass by assigning the current default to itself.
- [ ] Complete Rust tests remain green.

---

## LTR-200 — Reconcile the post-closeout hardening SPEC with current Luna/ASR-proxy state

Target:

`docs/LOCAL_TTS_POST_CLOSEOUT_HARDENING_SPEC_2026-09-13.md`

- [ ] Change stale status wording such as `Proposed implementation queue` to a truthful completed/historical status.
- [ ] Preserve the fact that PR #111 was required to leave `KCR-330` / `KTT-805` open **at that time**.
- [ ] Add the later chronology: PR #114 automated ASR audition, explicit owner approval, PR #115 Luna default selection.
- [ ] State that `KCR-330` / `KTT-805` is closed for V1 by owner-approved ASR proxy evidence.
- [ ] State that no subjective human listening claim is made.
- [ ] State that future human listening may override Luna but is not remaining V1 closeout work.
- [ ] Remove or contextualize statements that currently imply Bella is still provisional/default.
- [ ] Update the completion definition so it no longer requires a currently-open human-only gate.

**Acceptance**

- [ ] The spec remains historically intelligible.
- [ ] The spec does not contradict current `docs/VOICE_SELECTION.md` or PR #115 behavior.

---

## LTR-210 — Reconcile the post-closeout hardening TODO with current Luna/ASR-proxy state

Target:

`docs/LOCAL_TTS_POST_CLOSEOUT_HARDENING_TODO_2026-09-13.md`

- [ ] Preserve completed PR #111 engineering evidence.
- [ ] Update the introductory KCR-330/KTT-805 paragraph to explain original deferral plus later V1 ASR-proxy closeout.
- [ ] Reconcile LTH-000 wording that says not to mark KCR-330/KTT-805 complete.
- [ ] Reconcile LTH-500 from a current open-human-only gate to a historical boundary that PR #111 correctly preserved before later owner approval.
- [ ] Update final checklist wording so it no longer says KCR-330/KTT-805 remains open.
- [ ] Update intentionally-out-of-scope wording so future subjective audition is described as an optional override/evidence expansion rather than remaining V1 work.
- [ ] Cross-link the automated ASR audition and Luna default closeout documents.

**Acceptance**

- [ ] Every checkbox continues to describe work that actually occurred.
- [ ] Historical LTH acceptance is not retroactively falsified.
- [ ] Current-state summary agrees with Luna/ASR-proxy closeout.

---

## LTR-220 — Reconcile remaining current-looking KittenTTS closeout/spec documentation

Known review target:

`docs/KITTENTTS_CLOSEOUT_REMEDIATION_SPEC_2026-09-12.md`

- [ ] Search the repository for `KCR-330`.
- [ ] Search the repository for `KTT-805`.
- [ ] Search the repository for `Bella`.
- [ ] Search the repository for `Luna`.
- [ ] Search the repository for `human audition`.
- [ ] Search the repository for `human-only`.
- [ ] Search the repository for `owner-only`.
- [ ] Search the repository for `provisional`.
- [ ] Search the repository for `remains open`.
- [ ] Search the repository for `default voice`.
- [ ] Classify every relevant hit as current/accurate, historical/preserved, stale/update-required, or historical/requires-banner.
- [ ] Update stale current-state statements.
- [ ] Add a superseded/current-state banner where historical text is too easy to mistake for current status.
- [ ] Preserve historical evidence and dates rather than rewriting them to pretend the later decision existed earlier.

**Acceptance**

- [ ] All current-state docs agree that Luna is the V1 Local default.
- [ ] All current-state docs agree KCR-330/KTT-805 is closed for V1 by owner-approved ASR proxy.
- [ ] No current-state doc claims subjective human listening occurred.

---

## LTR-300 — Finish ASR-proxy default-voice closeout bookkeeping

Target:

`docs/LOCAL_TTS_ASR_PROXY_DEFAULT_VOICE_CLOSEOUT_2026-09-14.md`

- [ ] Record/check PR #115 exact head `4a8bf77e4803b7898c717a3cbe145f9df9600c75`.
- [ ] Record/check PR-head CI `34853091162` — PASS.
- [ ] Record/check PR-head production CPU acceptance `34853091150` — PASS.
- [ ] Record/check PR-head ASR intelligibility smoke `34853091234` — PASS.
- [ ] Record/check PR-head ASR voice audition `34853091230` — PASS.
- [ ] Record/check guarded merge of PR #115.
- [ ] Record/check merged master `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4`.
- [ ] Record/check post-merge CI `34854193904` — PASS.
- [ ] Record/check post-merge production CPU acceptance `34854193873` — PASS.
- [ ] Record/check post-merge ASR intelligibility smoke `34854193869` — PASS.
- [ ] Record/check post-merge ASR voice audition `34854193968` — PASS.
- [ ] Preserve explicit wording that the decision is ASR-proxy based, not subjective human listening.

**Acceptance**

- [ ] No completed evidence item remains falsely unchecked.
- [ ] No evidence item is checked without exact supporting evidence.

---

## LTR-400 — Freeze and document persisted Bella versus new Luna default policy

Unless the owner explicitly changes the policy during this task, implement the following:

> Luna is the default for new profiles and for settings where `local_tts_voice` is absent. Existing valid persisted Local voice choices, including Bella, remain preserved as user preference and are not silently migrated.

- [ ] Verify current `AppSettings::default()` uses `Luna`.
- [ ] Verify current settings load preserves an existing valid persisted `Bella` value.
- [ ] Add/update a regression test proving persisted Bella remains Bella after load/normalization.
- [ ] Add/update a regression test proving an applicable missing Local voice field receives current default Luna.
- [ ] Ensure tests distinguish defaulting from forced migration.
- [ ] Do not bump settings schema solely to force Bella → Luna.
- [ ] Do not infer whether persisted Bella was explicit user choice versus old default.
- [ ] Document the policy in the most appropriate settings/voice documentation.
- [ ] Confirm unknown invalid Local voice handling remains fail-safe under existing settings policy.

**Acceptance**

- [ ] New/default-on-missing Local voice is Luna.
- [ ] Existing valid persisted Bella remains Bella.
- [ ] No provider fallback or unrelated migration behavior changes.

---

## LTR-500 — Decide whether to strengthen ONNX waveform rank/shape validation

- [ ] Inspect the pinned production KittenTTS ONNX output metadata for `waveform`.
- [ ] Inspect the `ort` extraction API shape returned by the existing `try_extract_tensor::<f32>()` path.
- [ ] Record the actual real-model waveform rank/shape observed under acceptance.
- [ ] Decide whether the expected shape is stable enough to enforce.

Choose one path:

### Path A — Strengthen

- [ ] Define a small pure predicate/helper for acceptable waveform tensor shape.
- [ ] Validate rank/batch/channel dimensions before flattening samples.
- [ ] Preserve `f32`, nonempty, and finite-sample checks.
- [ ] Map shape mismatch to sanitized Local TTS inference failure.
- [ ] Add ordinary unit tests for accepted/rejected shapes.
- [ ] Preserve `duration` as presence-only drift detection unless a separate requirement changes it.
- [ ] Run real production CPU acceptance on Linux x86_64 and macOS arm64.

### Path B — Defer with evidence

- [ ] Record why pinned artifact identity plus current `f32`/nonempty/finite validation is the accepted V1 boundary.
- [ ] Record why enforcing tensor rank/shape would create a brittle or unsupported dependency.
- [ ] Add clarifying code/doc comment if needed.
- [ ] Do not pretend shape validation exists.

**Acceptance**

- [ ] Exactly one path is selected and documented.
- [ ] The runtime contract is more explicit after this task than before it.
- [ ] No waveform/sample-rate semantics are changed accidentally.

---

## LTR-600 — Run final source audit

- [ ] Audit `src-tauri/src/app/settings_policy.rs`.
- [ ] Audit settings defaults/migration/normalization in `src-tauri/src/app/state.rs`.
- [ ] Audit `src-tauri/src/ai/local_tts.rs`.
- [ ] Audit `src-tauri/src/ai/local_tts/runtime/engine.rs` if LTR-500 changes it.
- [ ] Audit Local TTS tests changed by LTR-100/LTR-400/LTR-500.
- [ ] Audit `docs/VOICE_SELECTION.md`.
- [ ] Audit the reconciled post-closeout hardening SPEC/TODO.
- [ ] Audit the ASR-proxy closeout document.
- [ ] Audit the KittenTTS closeout remediation spec/reconciliation/evidence docs.
- [ ] Confirm no Local-to-Google fallback.
- [ ] Confirm no Google-to-Local fallback.
- [ ] Confirm Google/Local/Live voice ownership remains separate.
- [ ] Confirm no utterance/audio/credential/raw-path leakage was introduced.
- [ ] Confirm no model/runtime/G2P pins changed unintentionally.
- [ ] Confirm no Local thread-policy change.
- [ ] Confirm no stale current-state Bella/provisional/open-human-gate wording remains in authoritative docs.
- [ ] Confirm no false subjective-listening claim.

**Acceptance**

- [ ] Final diff contains only intended remediation.
- [ ] No new mandatory defect remains unresolved in this scope.

---

## LTR-700 — Exact-head validation

### Always required

- [ ] Review exact final branch diff against current `master`.
- [ ] Confirm docs/source changes match this TODO.
- [ ] Pass exact PR-head ordinary CI.
- [ ] Record exact PR-head SHA.
- [ ] Record exact PR-head CI run ID.

### Required when Rust source/tests change

- [ ] Rust formatting passes.
- [ ] Clippy passes.
- [ ] Complete Rust tests pass.
- [ ] Settings/restart regression tests pass.
- [ ] Generated backend contract validation passes if contract-relevant settings/default source changed.

### Required when Local TTS runtime engine changes

- [ ] KittenTTS production CPU acceptance passes on exact PR head.
- [ ] Linux x86_64 real inference passes.
- [ ] macOS arm64 real inference passes.
- [ ] ASR intelligibility smoke passes if waveform/output behavior can plausibly change.
- [ ] ASR voice audition is rerun only if voice/output/audition behavior is actually affected.

**Acceptance**

- [ ] No stale or partially qualified head is eligible for merge.
- [ ] Expensive workflows are required by diff scope, not rerun merely for evidence recursion.

---

## LTR-800 — Guarded merge and exact-master verification

- [ ] Recheck PR mergeability immediately before merge.
- [ ] Merge only with the exact tested PR head SHA.
- [ ] Use an allowed guarded merge method.
- [ ] Record exact merged `master` SHA.
- [ ] Verify ordinary CI on exact merged `master`.
- [ ] Verify production CPU acceptance on exact merged `master` if runtime-engine changes required it pre-merge.
- [ ] Verify ASR smoke/audition post-merge only when required by the final source diff.
- [ ] Update this TODO with final evidence without starting an evidence-only recursion loop.

**Acceptance**

- [ ] `master` contains the remediation.
- [ ] Exact merged-master validation evidence is recorded.

---

## Final checklist

- [ ] Restart-policy Local voice test performs a real voice change.
- [ ] Post-closeout hardening SPEC reconciled to current Luna/ASR-proxy state.
- [ ] Post-closeout hardening TODO reconciled to current Luna/ASR-proxy state.
- [ ] Remaining current-looking KittenTTS closeout/spec docs reconciled or clearly marked historical.
- [ ] ASR-proxy default-voice closeout evidence bookkeeping completed.
- [ ] Persisted Bella versus new Luna default behavior explicitly documented and tested.
- [ ] Waveform shape contract deliberately strengthened or explicitly deferred with evidence.
- [ ] Repository-wide stale-wording audit completed.
- [ ] Provider separation preserved.
- [ ] No-fallback behavior preserved.
- [ ] Privacy/logging boundaries preserved.
- [ ] Artifact pins/runtime/G2P revisions unchanged unless separately justified.
- [ ] Exact PR-head required validation passed.
- [ ] Guarded merge completed from exact tested head.
- [ ] Exact merged-master required validation passed.
