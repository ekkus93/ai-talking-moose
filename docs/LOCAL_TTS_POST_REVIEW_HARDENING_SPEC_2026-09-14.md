# AI Talking Moose — Local TTS Post-Review Hardening Specification

**Date:** 2026-09-14
**Companion TODO:** `docs/LOCAL_TTS_POST_REVIEW_HARDENING_TODO_2026-09-14.md`
**Reviewed baseline:** `05c5d8106cda6febe871ba7a4eb579c818be695a` (`master`)
**Review scope:** Local KittenTTS post-closeout implementation, tests, closeout documentation, persisted-default semantics, and ONNX output-contract hardening
**Status:** Implemented by PR #118; closeout reconciled by PR #119; documentation precision cleanup pending

This specification captures the issues found during the 2026-09-14 code review of current `master` after the Local KittenTTS technical closeout, post-closeout hardening, automated ASR voice audition, Luna default selection, and the PR #116 documentation reconciliation.

The Local TTS implementation is substantially complete and healthy. This remediation must **not** reopen already-qualified runtime, installer, playback, provider-separation, privacy, or model-artifact work without concrete evidence of a regression. The purpose of this batch is to repair one real test-regression, reconcile stale documentation, make persisted-default behavior explicit, and optionally strengthen the ONNX waveform contract at a narrow defensive boundary.

---

## 1. Current verified state

At the reviewed baseline:

- current `master`: `05c5d8106cda6febe871ba7a4eb579c818be695a`;
- production Local KittenTTS model: `KittenML/kitten-tts-mini-0.8`;
- current Local KittenTTS default voice: `Luna`;
- `KCR-330` / `KTT-805`: closed for V1 by explicit owner-approved automated ASR proxy evidence;
- no subjective human listening claim is made by that closeout;
- existing persisted Local voice choices remain preserved by the current settings loader;
- Local-to-Google fallback remains forbidden;
- Google standalone TTS, Local standalone TTS, and Gemini Live voice ownership remain separate;
- real KittenTTS production CPU acceptance is qualified on Linux x86_64 and macOS arm64;
- macOS x86_64 remains a compile/package/provenance boundary unless separate real-inference evidence is added.

Relevant validated voice-selection evidence already on record:

- PR #114 added automated ASR voice audition;
- PR #114 PR-head ASR audition: `34821757481` — PASS;
- PR #114 post-merge ASR audition: `34848251323` — PASS;
- PR #115 changed the default Local voice to `Luna`;
- PR #115 exact PR head: `4a8bf77e4803b7898c717a3cbe145f9df9600c75`;
- PR-head CI `34853091162` — PASS;
- PR-head production CPU acceptance `34853091150` — PASS;
- PR-head ASR intelligibility smoke `34853091234` — PASS;
- PR-head ASR voice audition `34853091230` — PASS;
- PR #115 merged master: `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4`;
- post-merge CI `34854193904` — PASS;
- post-merge production CPU acceptance `34854193873` — PASS;
- post-merge ASR intelligibility smoke `34854193869` — PASS;
- post-merge ASR voice audition `34854193968` — PASS;
- PR #116 reconciled the main KittenTTS closeout TODO/reconciliation/evidence documents;
- PR #116 merged master: `05c5d8106cda6febe871ba7a4eb579c818be695a`;
- PR #116 post-merge docs CI `34857432063` — PASS.

---

## 2. Goals

This remediation has seven primary goals:

1. Repair the Local-TTS restart-policy regression test so it proves a real Local voice change rather than a no-op assignment to the current default.
2. Reconcile stale post-closeout hardening specification/TODO wording with the later Luna/ASR-proxy decision without rewriting historical evidence.
3. Reconcile other still-current-looking KittenTTS closeout/spec documentation that presents the pre-PR-115 human-only voice gate as current state.
4. Complete the ASR-proxy default-voice closeout bookkeeping that still contains unchecked validation/merge rows despite completed evidence.
5. Define and document the intended behavior for existing persisted `Bella` profiles versus the new `Luna` default.
6. Decide whether to strengthen the ONNX `waveform` output tensor rank/shape contract as defense in depth, and implement/tests it only if the expected production tensor shape can be stated truthfully.
7. Run a repository-wide stale-wording/source audit and finish with exact-head validation, guarded merge, and exact-master verification.

---

## 3. Non-goals

This remediation must not, unless a new defect is discovered and explicitly documented:

- change the pinned KittenTTS model revision;
- change Kitten voice embedding artifacts;
- change CMUdict/G2P source data or revision;
- change ONNX Runtime version or platform artifacts;
- change artifact SHA-256 values or expected sizes;
- change phonemization/tokenization semantics;
- change style-row selection semantics;
- change speaking-rate semantics;
- add Local pitch support;
- add a user-facing Local TTS thread-count setting;
- change the 2-thread production policy;
- introduce Google/Local/browser/platform fallback;
- change the CPAL playback path or mouth-animation adaptation;
- change Gemini Live behavior;
- re-open `KCR-330` / `KTT-805` merely because the closeout used ASR proxy evidence rather than subjective listening;
- claim subjective listening that did not occur;
- claim real macOS x86_64 inference without a real inference run.

---

## 4. Finding LTR-100 — Restart-policy test no longer proves a real Local voice change

### Problem

After the default Local KittenTTS voice changed from `Bella` to `Luna`, a restart-policy test still assigns:

```rust
next.local_tts_voice = "Luna".to_string();
```

when `AppSettings::default()` already has `local_tts_voice == "Luna"`.

The test therefore verifies that a **no-op** does not trigger a conversation restart rather than verifying the intended invariant that a real Local voice change does not require a conversation restart.

The production policy itself appears correct: Local standalone voice changes are intentionally not part of the conversation restart decision. The defect is in regression coverage.

### Requirement

Update the test so it performs a real transition from the default Local voice to another valid Local catalog voice, preferably `Leo` or another non-default voice.

The test must explicitly prove that the before and after Local voice IDs differ before asserting restart behavior.

### Required assertions

- `previous.local_tts_voice != next.local_tts_voice`;
- both values are valid Local KittenTTS voice IDs;
- changing only `local_tts_voice` does not require a conversation restart;
- the test must not mutate Google standalone voice or Gemini Live voice as an accidental substitute for the Local invariant.

### Acceptance

- The test would fail if `local_tts_voice` were accidentally added to the conversation-restart key set.
- The test cannot pass by assigning the already-current default.
- Existing provider-switch and settings-validation tests remain green.

---

## 5. Finding LTR-200 — Post-closeout hardening SPEC/TODO are stale after PR #114/#115

### Problem

The following documents still describe the pre-PR-115 state:

- `docs/LOCAL_TTS_POST_CLOSEOUT_HARDENING_SPEC_2026-09-13.md`
- `docs/LOCAL_TTS_POST_CLOSEOUT_HARDENING_TODO_2026-09-13.md`

They contain statements such as:

- `KCR-330` / `KTT-805` must remain open;
- final default voice selection remains human-only;
- Bella remains provisional/default;
- automated ASR must not close the decision;
- the hardening spec still presents itself as a proposed queue even though PR #111 completed the technical work.

Those statements were truthful when written but became stale after the owner explicitly accepted ASR proxy evidence and PR #115 selected `Luna`.

### Requirement

Reconcile both documents without destroying historical context.

The updated documents must distinguish:

1. what PR #111 was required to preserve **at the time**;
2. what later PR #114/#115 changed by explicit owner decision;
3. what remains unproved subjectively.

### Required current-state wording

The docs must state that:

- PR #111 completed all automatable LTH engineering work;
- PR #114 later introduced a dedicated automated ASR voice-audition proxy;
- the owner explicitly accepted that proxy for V1;
- PR #115 selected `Luna` as the V1 Local default;
- `KCR-330` / `KTT-805` is closed for V1;
- this is **not** subjective listening evidence;
- future human listening can override `Luna`, but is a new decision rather than remaining V1 closeout work.

### Acceptance

- No reader can reasonably conclude from the active hardening docs that Bella is still the V1 default.
- No reader can reasonably conclude that `KCR-330` / `KTT-805` remains an open V1 blocker.
- Historical PR #111 constraints remain explainable as historical constraints rather than silently rewritten as if they never existed.

---

## 6. Finding LTR-210 — Additional KittenTTS closeout/spec documentation still presents stale voice-gate state

### Problem

At least one still-current-looking closeout specification retains pre-PR-115 wording around the human-only voice gate. Historical files can remain historically accurate, but active or authoritative-looking documents must not contradict current master.

Known review target:

- `docs/KITTENTTS_CLOSEOUT_REMEDIATION_SPEC_2026-09-12.md`

The implementation must also search for other occurrences of stale phrases rather than assuming that this one file is the only remaining case.

### Requirement

Perform a bounded documentation reconciliation audit using exact repository search for at least:

- `Bella`;
- `Luna`;
- `KCR-330`;
- `KTT-805`;
- `human audition`;
- `human-only`;
- `owner-only`;
- `provisional`;
- `remains open`;
- `default voice`.

Each hit must be classified as one of:

1. current and accurate;
2. historical and intentionally preserved;
3. stale and must be updated;
4. historical but too easy to mistake for current state, requiring a superseded/current-state banner.

### Acceptance

- All current-state docs agree that Luna is the V1 default.
- All current-state docs agree that `KCR-330` / `KTT-805` is closed for V1 by owner-approved ASR proxy.
- Historical evidence is not falsified.
- Historical specs/trackers that intentionally preserve old wording clearly point to current reconciliation documents when needed.

---

## 7. Finding LTR-300 — ASR-proxy default-voice closeout bookkeeping is incomplete

### Problem

`docs/LOCAL_TTS_ASR_PROXY_DEFAULT_VOICE_CLOSEOUT_2026-09-14.md` still contains unchecked validation and merge rows even though the corresponding PR-head and post-merge evidence is complete.

Unchecked bookkeeping can be misread as incomplete engineering work.

### Requirement

Update the closeout document to mark only evidence that actually exists.

At minimum, record/check:

- exact PR #115 head `4a8bf77e4803b7898c717a3cbe145f9df9600c75`;
- PR-head ordinary CI `34853091162` — PASS;
- PR-head production CPU acceptance `34853091150` — PASS;
- PR-head ASR intelligibility smoke `34853091234` — PASS;
- PR-head ASR voice audition `34853091230` — PASS;
- guarded PR #115 merge;
- merged master `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4`;
- post-merge CI `34854193904` — PASS;
- post-merge production CPU acceptance `34854193873` — PASS;
- post-merge ASR intelligibility smoke `34854193869` — PASS;
- post-merge ASR voice audition `34854193968` — PASS.

### Acceptance

- No completed evidence row remains falsely unchecked.
- No evidence is checked without an exact SHA/run or other concrete basis.
- The document still says the decision is ASR-proxy based, not subjective human listening.

---

## 8. Finding LTR-400 — Persisted Bella behavior must be explicitly decided and documented

### Problem

`AppSettings::default()` now uses `Luna`, but an existing persisted settings record with:

```json
"local_tts_voice": "Bella"
```

continues to load as `Bella` because the settings version did not introduce a migration that rewrites existing Local voice choices.

This is likely desirable because the application cannot safely distinguish:

- a user who explicitly chose Bella; from
- a user who simply retained Bella when it was the old provisional default.

However, the intended product behavior must be explicit rather than accidental.

### Required policy decision

Unless the owner explicitly chooses otherwise during implementation, adopt this policy:

> `Luna` is the default for new profiles and profiles where `local_tts_voice` is missing. Existing valid persisted Local voice selections, including `Bella`, are preserved as user preference and are not silently migrated.

### Requirement

- Document the policy in the appropriate voice/settings documentation.
- Add or update a settings migration/normalization test proving a valid persisted Bella value remains Bella.
- Add or update a test proving a missing legacy/current Local voice field receives the current default Luna where that behavior is applicable.
- Do not bump the settings schema merely to force an existing valid voice preference to Luna.
- Do not add heuristic migration that guesses whether Bella was user-chosen.

### Acceptance

- New/default-on-missing behavior is Luna.
- Existing valid persisted Bella remains Bella.
- Unknown Local voices continue to fail or normalize according to the existing settings policy; no silent Google fallback is introduced.
- Documentation accurately distinguishes **default** from **forced migration**.

---

## 9. Finding LTR-500 — Consider waveform tensor rank/shape validation as defense in depth

### Problem

The Local KittenTTS runtime correctly:

- requires exact named outputs `waveform` and `duration`;
- requires exact output count;
- extracts `waveform` as `f32`;
- rejects empty/non-finite samples;
- uses `duration` only as a presence-only contract sentinel, as intentionally decided by LTH-300.

The runtime does not currently enforce a specific waveform tensor rank/shape before flattening the extracted `f32` values into mono PCM.

Because the production model/runtime artifacts are immutably pinned, this is low-risk defense-in-depth rather than a confirmed production bug.

### Decision requirement

Before changing code, inspect the real production model output shape and the `ort` extraction API contract.

Choose exactly one:

1. **Strengthen:** if the expected shape can be stated truthfully and stably, validate the allowed waveform shape/rank and fail closed on unexpected batch/channel dimensions; or
2. **Defer with evidence:** if the model/API contract does not provide a stable shape guarantee worth enforcing, record why nonempty finite `f32` plus pinned artifact identity remains the accepted V1 boundary.

### If strengthening

Tests must cover the shape predicate independently from real ORT inference where possible. The real production CPU acceptance workflow must rerun on Linux x86_64 and macOS arm64.

### Invariants

- Do not change waveform sample values.
- Do not alter sample rate.
- Do not invent semantics for `duration`.
- Do not consume `duration` downstream merely to justify validation.
- Unexpected output shape must fail with sanitized inference error semantics.

### Acceptance

- The chosen contract is explicit in code/docs/tests.
- Real KittenTTS acceptance remains green if runtime code changes.

---

## 10. Finding LTR-600 — Final source/documentation audit

Before final validation, audit the complete diff and relevant current source.

At minimum inspect:

- `src-tauri/src/app/settings_policy.rs`;
- `src-tauri/src/app/state.rs` settings defaults/migration/normalization paths;
- `src-tauri/src/ai/local_tts.rs`;
- `src-tauri/src/ai/local_tts/runtime/engine.rs` if LTR-500 changes it;
- `docs/VOICE_SELECTION.md`;
- `docs/LOCAL_TTS_POST_CLOSEOUT_HARDENING_SPEC_2026-09-13.md`;
- `docs/LOCAL_TTS_POST_CLOSEOUT_HARDENING_TODO_2026-09-13.md`;
- `docs/LOCAL_TTS_ASR_PROXY_DEFAULT_VOICE_CLOSEOUT_2026-09-14.md`;
- `docs/KITTENTTS_CLOSEOUT_REMEDIATION_SPEC_2026-09-12.md`;
- current reconciliation/evidence documents;
- historical TODO/SPEC banners.

Audit for:

- no Local-to-Google fallback;
- no Google-to-Local fallback;
- Google/Local/Live voice ownership separation;
- privacy-safe errors/logs;
- no utterance/audio/credential leakage;
- no accidental change to model/runtime/G2P pins;
- no accidental change to Local thread policy;
- no stale current-state Bella/default wording;
- no false claim of subjective listening;
- no unchecked evidence rows that have concrete completed evidence;
- no historical evidence rewritten inaccurately.

---

## 11. Validation requirements

Validation must be proportional to the final diff.

### Always required

- review exact branch diff against current master;
- `git diff --check` equivalent through CI/static checks;
- Rust formatting if Rust source changes;
- Clippy if Rust source changes;
- complete Rust tests if Rust source changes;
- generated backend contract validation if settings/default/contract source changes;
- frontend/static quality gates according to repository path classifier;
- exact PR-head ordinary CI;
- guarded merge using the exact tested head;
- exact merged-master ordinary CI.

### Required if settings tests/source change

- settings migration/normalization tests;
- restart-policy regression test;
- generated contract validation if representative settings/default contract changes.

### Required if `engine.rs` waveform contract changes

- exact PR-head KittenTTS production CPU acceptance on Linux x86_64 and macOS arm64;
- exact post-merge production CPU acceptance;
- ASR intelligibility smoke if output adaptation could plausibly affect audio;
- ASR voice audition only if voice/output behavior or its workflow contract is affected.

### Docs-only follow-up evidence

Do not rerun expensive real-model workflows solely because an evidence-only docs commit records already-qualified source behavior.

---

## 12. Merge and evidence policy

- Use a dedicated branch under the repository's allowed Ralph prefix.
- Bind the PR to the exact inspected head SHA where supported.
- Do not merge from a stale head after CI passes.
- Use the repository's guarded merge path.
- Verify the exact merged `master` SHA.
- Record exact run IDs in the companion TODO/evidence update without creating an infinite evidence-update loop.
- Do not modify branch protection, required checks, or other repository policy to force the merge.

---

## 13. Completion definition

This remediation is complete only when all of the following are true:

- the Local voice restart-policy test performs a real non-default voice change and proves the intended restart invariant;
- the post-closeout hardening SPEC/TODO accurately reflect the later Luna/ASR-proxy closeout while preserving historical context;
- remaining current-looking closeout/spec docs no longer present Bella or the human-only V1 gate as current state;
- ASR-proxy closeout evidence rows accurately reflect completed validation/merge work;
- persisted Bella versus new Luna-default behavior is explicitly documented and regression-tested;
- the waveform tensor shape decision is either implemented/tested or deliberately deferred with a concrete technical rationale;
- repository-wide stale-wording audit is complete;
- no provider separation, no-fallback, privacy, artifact-pinning, runtime, playback, or thread-policy regression was introduced;
- exact PR-head required CI passes;
- guarded merge uses the exact tested head;
- exact merged-master required validation passes;
- companion TODO is updated with final evidence without evidence recursion.
