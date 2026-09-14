# AI Talking Moose — Local TTS Post-Closeout Hardening TODO

**Date:** 2026-09-13
**Companion spec:** `docs/LOCAL_TTS_POST_CLOSEOUT_HARDENING_SPEC_2026-09-13.md`
**Original baseline:** `78ccea9838c8003332fc7499e7d5b6bc94623044` (`master`)
**Implementation baseline:** `9ff82b25bc169fdae5f024e8efbbf20ccbc0abcf` (`master`, after docs PR #110)
**Implemented by:** PR #111, `fix: continue Local TTS post-closeout hardening`
**Merged master:** `d1d579295eb224e952fd1e6c11e4983f5f21877f`
**Status:** Complete for all automatable post-closeout hardening work

This TODO covers only the post-closeout hardening issues found in the Local KittenTTS code review after the technical closeout and PR #109. It is not a replacement for the completed closeout tracker, and it must not reopen work already verified in `docs/KITTENTTS_CLOSEOUT_REMEDIATION_TODO_2026-09-12.md`.

At the time PR #111 ran, `KCR-330` / `KTT-805` remained owner-only and was correctly left open by this engineering closeout. Later, PR #114 added an automated ASR audition proxy, the owner explicitly accepted that proxy for V1, and PR #115 selected `Luna`. The gate is now closed for V1 by owner-approved ASR proxy evidence; no subjective human listening claim is made. See `docs/LOCAL_TTS_AUTOMATED_ASR_VOICE_AUDITION_2026-09-14.md` and `docs/LOCAL_TTS_ASR_PROXY_DEFAULT_VOICE_CLOSEOUT_2026-09-14.md`.

---

## Final implementation evidence

### PR-head validation

Exact PR head merged by PR #111:

```text
31630cbe1056e82d28725f4905c6c17f600c698b
```

Validated runs on that exact PR head:

- [x] CI `34814655908` — PASS.
- [x] KittenTTS production CPU acceptance `34814655921` — PASS.
- [x] KittenTTS ASR intelligibility smoke `34814655929` — PASS.
- [x] P21-P23 Rust Stability Acceptance `34814655944` — skipped as unrelated to this Local TTS post-closeout diff.

### Guarded merge and post-merge validation

Merged master SHA:

```text
d1d579295eb224e952fd1e6c11e4983f5f21877f
```

Validated runs on that exact merged master SHA:

- [x] CI `34815365874` — PASS.
- [x] KittenTTS production CPU acceptance `34815366008` — PASS.
- [x] KittenTTS ASR intelligibility smoke `34815365898` — PASS.

---

## LTH-000 — Preserve baseline and scope

- [x] Confirm the implementation branch starts from `78ccea9838c8003332fc7499e7d5b6bc94623044` or a later verified master.
  - Implemented from later verified master `9ff82b25bc169fdae5f024e8efbbf20ccbc0abcf`.
- [x] Keep this batch limited to Local TTS post-closeout cleanup, diagnostics precision, and documentation hygiene.
- [x] Do not change pinned KittenTTS model/runtime/G2P artifacts, hashes, or source revisions.
- [x] Do not change synthesis semantics, waveform generation, playback, or mouth-animation behavior unless required by a validated bug fix.
- [x] Do not introduce Google fallback, Local fallback, browser speech fallback, or platform speech fallback.
- [x] Do not add a user-facing Local TTS thread-count setting.
- [x] Preserve `KCR-330` / `KTT-805` as open for the PR #111 implementation because no owner decision existed yet; later PR #115 closeout is recorded separately.

**Acceptance**

- [x] Final PR description lists only the issues from this TODO/spec.
- [x] Any deferred item is explicitly left unchecked with a reason.

---

## LTH-100 — Retire `PendingLocalSpeechSynthesizer`

- [x] Search the repository for every `PendingLocalSpeechSynthesizer` reference.
- [x] Confirm no production Local TTS route still depends on the placeholder.
- [x] Remove `PendingLocalSpeechSynthesizer` because it had no current direct caller.
- [x] Remove stale placeholder-only tests.
- [x] Preserve fail-closed Local TTS behavior when the model is unavailable or not installed.
- [x] Preserve no-fallback behavior from Local to Google.

**Acceptance**

- [x] Production Local TTS selection cannot hit a stale placeholder.
- [x] Tests cover the current production Local TTS unavailable-model behavior instead of relying on placeholder behavior.
- [x] Repository search no longer leaves confusing unused placeholder code.

---

## LTH-110 — Rename stale Local TTS tests

- [x] Search for test names that imply Local TTS is still pending, unavailable in this build, or awaiting runtime integration.
- [x] Rename `local_tts_selection_fails_closed_before_runtime_integration` to a current-behavior name.
- [x] Use precise wording: `local_tts_selection_fails_closed_when_model_is_not_installed`.
- [x] Confirm renamed tests still assert no utterance leakage.
- [x] Confirm renamed tests still assert no provider fallback.
- [x] Avoid deleting useful coverage just to remove stale wording.

**Acceptance**

- [x] No test name misrepresents the current integrated Local TTS runtime state.
- [x] The same behavioral invariants remain covered after renaming.

---

## LTH-200 — Use Local-specific safe messages for pre-runtime validation errors

- [x] Audit `LocalSpeechSynthesizer::inference_request()` error mapping.
- [x] Add a helper for Local-specific `ProviderError` construction.
- [x] Map unsupported configured Local TTS model to `ProviderErrorKind::Model` with bounded Local-specific text.
- [x] Map unsupported Local TTS voice to `ProviderErrorKind::Setup` with bounded Local-specific text.
- [x] Preserve retryability semantics from the existing provider error policy.
- [x] Preserve provider-neutral cancellation when cancellation is observed before runtime use.
- [x] Ensure error messages do not include utterance text, audio bytes, credentials, raw filesystem paths, or raw lower-level errors.
- [x] Add/update tests for invalid model message text.
- [x] Add/update tests for invalid voice message text.
- [x] Add/update tests proving request text is not leaked in those errors.

**Acceptance**

- [x] Pre-runtime Local TTS validation failures no longer use generic conversation-provider wording.
- [x] Provider error kinds remain compatible with existing callers.
- [x] No Local-to-Google fallback is introduced.

---

## LTH-300 — Clarify or strengthen ONNX `duration` output validation

- [x] Audit `KittenTtsRuntimeEngine` output-contract validation after PR #109.
- [x] Decide whether `duration` should remain presence-only drift detection or should be type/shape validated.
  - Decision: keep `duration` as presence-only drift detection for this hardening batch.
- [x] Comment the helper to state that only `waveform` is semantically consumed.
- [x] Preserve named lookup of `waveform` rather than positional output indexing.
- [x] Preserve exact output-count validation.
- [x] Preserve fail-closed sanitized `LocalTtsRuntimeErrorKind::Inference` for malformed output contracts.
- [x] Preserve existing tests covering missing `waveform` through `model_output_contract_is_valid`.
- [x] Preserve existing tests covering missing `duration` through `model_output_contract_is_valid`.
- [x] Preserve existing tests covering extra outputs through `model_output_contract_is_valid`.
- [x] Preserve existing tests covering wrong output count through `model_output_contract_is_valid`.
- [x] Defer `duration` tensor type/shape validation because no production behavior consumes `duration` yet and this batch intentionally avoided adding a false semantic dependency on that output.

**Acceptance**

- [x] A future reviewer can tell exactly what the ONNX output contract validates.
- [x] Runtime still consumes only intended waveform data.
- [x] Real KittenTTS acceptance remains green after the output-contract documentation/test rename.

---

## LTH-400 — Add superseded banners to historical KittenTTS docs

- [x] Open `docs/TODO(20260909-120003).md` and determine whether it has a clear superseded banner near the top.
- [x] Open `docs/SPEC(20260909-120003).md` and determine whether it has a clear superseded banner near the top.
- [x] Add a banner to each stale historical KittenTTS tracker/spec that can be mistaken for an active queue.
- [x] Point readers to `docs/KITTENTTS_CLOSEOUT_REMEDIATION_TODO_2026-09-12.md` for active closeout status.
- [x] Point readers to `docs/KITTENTTS_LEGACY_TODO_RECONCILIATION_2026-09-12.md` for legacy KTT mapping.
- [x] Point readers to this TODO for post-closeout hardening work.
- [x] Preserve historical content below the banner.
- [x] Do not rewrite old evidence or mark human-only work complete in historical files.

**Acceptance**

- [x] A reader opening a historical KittenTTS TODO/SPEC sees the superseded warning before stale unchecked rows.
- [x] Historical files remain useful for provenance but cannot reasonably be confused with the active queue.

---

## LTH-500 — Preserve the then-human-only `KCR-330` / `KTT-805` boundary

- [x] Verify PR #111-era `docs/VOICE_SELECTION.md` kept audition/default selection open until an owner decision existed.
- [x] Verify the PR #111-era closeout/reconciliation docs preserved that same boundary.
- [x] Do not change Bella from the then-provisional default solely from the pre-PR-114 ASR smoke.
- [x] Do not describe the pre-PR-114 ASR smoke as subjective voice acceptance.
- [x] Record the later chronology separately: PR #114 added the ASR audition proxy; the owner accepted it for V1; PR #115 selected `Luna`.
- [x] Record current status as V1 closed by owner-approved ASR proxy, without claiming subjective human listening.

**Acceptance**

- [x] PR #111 preserved the correct boundary for its historical implementation point.
- [x] Current documentation distinguishes that historical boundary from the later explicit owner-approved ASR-proxy decision.

---

## LTH-600 — Source audit before PR

- [x] Audit `src-tauri/src/ai/local_tts.rs` after changes.
- [x] Audit `src-tauri/src/ai/local_tts/runtime/engine.rs` after changes.
- [x] Audit changed tests for privacy and no-fallback invariants.
- [x] Audit changed docs for accurate active/superseded status.
- [x] Confirm no generated artifacts were edited manually.
- [x] Confirm no unrelated frontend, runtime, installer, manifest, or workflow changes slipped into the diff.

**Acceptance**

- [x] The final diff contains only the intended hardening/documentation changes.

---

## LTH-700 — Validation

- [x] Verify changed-file diff through Ralph Bridge guarded write/readback.
- [x] Verify Rust formatting through CI.
- [x] Verify Clippy through CI.
- [x] Verify complete Rust tests through CI.
- [x] Verify generated backend contract validation through CI.
- [x] Verify release/static metadata gates through CI.
- [x] Verify Local TTS packaging policy through CI.
- [x] Run KittenTTS production CPU acceptance because runtime-engine tests changed and the thread-sweep harness was hardened.
- [x] Run KittenTTS ASR intelligibility smoke because this closeout touches Local TTS acceptance/test boundaries.
- [x] Record exact PR-head CI run IDs.

**Acceptance**

- [x] Required exact-head validation passed before merge.
- [x] Skipped heavyweight gates are explicitly justified by diff scope, not convenience.

---

## LTH-800 — Guarded merge and exact-master verification

- [x] Open/update the PR against `master`.
- [x] Confirm the PR head is exactly the commit intended for merge.
- [x] Merge only with the expected exact PR head.
- [x] Record the merged master SHA.
- [x] Verify ordinary CI on exact merged master.
- [x] Verify production CPU acceptance on exact merged master because required by diff scope.
- [x] Verify ASR smoke on exact merged master because required by diff scope.
- [x] Update this TODO with final evidence without creating an evidence-recursion loop.

**Acceptance**

- [x] Master contains the hardening work.
- [x] Exact-master validation evidence is recorded.

---

## Final checklist

- [x] `PendingLocalSpeechSynthesizer` removed.
- [x] Stale Local TTS test names updated.
- [x] Pre-runtime Local TTS validation messages are Local-specific, safe, and tested.
- [x] ONNX `duration` output validation is explicitly documented as presence-only contract drift detection.
- [x] Historical KittenTTS docs have superseded banners.
- [x] `KCR-330` / `KTT-805` historical PR #111 boundary is preserved, and the later V1 owner-approved ASR-proxy closeout is recorded truthfully.
- [x] Exact-head validation passed.
- [x] Guarded merge completed.
- [x] Exact merged-master verification passed.

## Intentionally out of scope

- Subjective human voice audition as an optional future override/evidence-expansion path; it is no longer required to close the V1 default-voice decision.
- Real macOS x86_64 KittenTTS inference qualification, unless explicitly added as a future evidence-expansion task.
- Aspirational latency target expansion beyond the already-verified hard warm RTF gate.
