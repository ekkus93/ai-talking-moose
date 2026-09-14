# AI Talking Moose — Local TTS Post-Closeout Hardening TODO

**Date:** 2026-09-13
**Companion spec:** `docs/LOCAL_TTS_POST_CLOSEOUT_HARDENING_SPEC_2026-09-13.md`
**Baseline:** `78ccea9838c8003332fc7499e7d5b6bc94623044` (`master`)
**Status:** Planned implementation queue

This TODO covers only the post-closeout hardening issues found in the Local KittenTTS code review after the technical closeout and PR #109. It is not a replacement for the completed closeout tracker, and it must not reopen work already verified in `docs/KITTENTTS_CLOSEOUT_REMEDIATION_TODO_2026-09-12.md`.

`KCR-330` / `KTT-805` remains owner-only human voice audition/default selection. Do not mark it complete here from automated ASR evidence.

---

## LTH-000 — Preserve baseline and scope

- [ ] Confirm the implementation branch starts from `78ccea9838c8003332fc7499e7d5b6bc94623044` or a later verified master.
- [ ] Keep this batch limited to Local TTS post-closeout cleanup, diagnostics precision, and documentation hygiene.
- [ ] Do not change pinned KittenTTS model/runtime/G2P artifacts, hashes, or source revisions.
- [ ] Do not change synthesis semantics, waveform generation, playback, or mouth-animation behavior unless required by a validated bug fix.
- [ ] Do not introduce Google fallback, Local fallback, browser speech fallback, or platform speech fallback.
- [ ] Do not add a user-facing Local TTS thread-count setting.
- [ ] Do not mark `KCR-330` / `KTT-805` complete.

**Acceptance**

- [ ] Final PR description lists only the issues from this TODO/spec.
- [ ] Any deferred item is explicitly left unchecked with a reason.

---

## LTH-100 — Retire `PendingLocalSpeechSynthesizer`

- [ ] Search the repository for every `PendingLocalSpeechSynthesizer` reference.
- [ ] Confirm no production Local TTS route still depends on the placeholder.
- [ ] Remove `PendingLocalSpeechSynthesizer` if it has no current direct caller.
- [ ] Remove stale placeholder-only tests if the placeholder is removed.
- [ ] If any current direct caller requires retention, document the caller and constrain the placeholder to that compatibility path only.
- [ ] Preserve fail-closed Local TTS behavior when the model is unavailable or not installed.
- [ ] Preserve no-fallback behavior from Local to Google.

**Acceptance**

- [ ] Production Local TTS selection cannot hit a stale placeholder.
- [ ] Tests cover the current production Local TTS unavailable-model behavior instead of relying on placeholder behavior.
- [ ] Repository search no longer leaves confusing unused placeholder code, or the remaining compatibility reason is explicit and current.

---

## LTH-110 — Rename stale Local TTS tests

- [ ] Search for test names that imply Local TTS is still pending, unavailable in this build, or awaiting runtime integration.
- [ ] Rename `local_tts_selection_fails_closed_before_runtime_integration` to a current-behavior name.
- [ ] Use precise wording such as `local_tts_selection_fails_closed_when_model_is_not_installed`.
- [ ] Confirm renamed tests still assert no utterance leakage.
- [ ] Confirm renamed tests still assert no provider fallback.
- [ ] Avoid deleting useful coverage just to remove stale wording.

**Acceptance**

- [ ] No test name misrepresents the current integrated Local TTS runtime state.
- [ ] The same behavioral invariants remain covered after renaming.

---

## LTH-200 — Use Local-specific safe messages for pre-runtime validation errors

- [ ] Audit `LocalSpeechSynthesizer::inference_request()` error mapping.
- [ ] Add a small helper for Local-specific `ProviderError` construction if needed.
- [ ] Map unsupported configured Local TTS model to `ProviderErrorKind::Model` with bounded Local-specific text.
- [ ] Map unsupported Local TTS voice to `ProviderErrorKind::Setup` with bounded Local-specific text.
- [ ] Preserve retryability semantics from the existing provider error policy.
- [ ] Preserve provider-neutral cancellation when cancellation is observed before runtime use.
- [ ] Ensure error messages do not include utterance text, audio bytes, credentials, raw filesystem paths, or raw lower-level errors.
- [ ] Add or update tests for invalid model message text.
- [ ] Add or update tests for invalid voice message text.
- [ ] Add or update tests proving request text is not leaked in those errors.

**Acceptance**

- [ ] Pre-runtime Local TTS validation failures no longer use generic conversation-provider wording.
- [ ] Provider error kinds remain compatible with existing callers.
- [ ] No Local-to-Google fallback is introduced.

---

## LTH-300 — Clarify or strengthen ONNX `duration` output validation

- [ ] Audit `KittenTtsRuntimeEngine` output-contract validation after PR #109.
- [ ] Decide whether `duration` should remain presence-only drift detection or should be type/shape validated.
- [ ] If keeping presence-only validation, rename or comment the helper to state that only `waveform` is semantically consumed.
- [ ] If strengthening validation, extract and validate `duration` with the expected tensor type/shape.
- [ ] Preserve named lookup of `waveform` rather than positional output indexing.
- [ ] Preserve exact output-count validation.
- [ ] Preserve fail-closed sanitized `LocalTtsRuntimeErrorKind::Inference` for malformed output contracts.
- [ ] Add or update tests covering missing `waveform`.
- [ ] Add or update tests covering missing `duration`.
- [ ] Add or update tests covering extra outputs.
- [ ] Add or update tests covering wrong output count.
- [ ] If type/shape validation is implemented, add focused coverage for malformed `duration` where practical.

**Acceptance**

- [ ] A future reviewer can tell exactly what the ONNX output contract validates.
- [ ] Runtime still consumes only intended waveform data.
- [ ] Real KittenTTS acceptance remains green if runtime behavior changes beyond comments/tests.

---

## LTH-400 — Add superseded banners to historical KittenTTS docs

- [ ] Open `docs/TODO(20260909-120003).md` and determine whether it has a clear superseded banner near the top.
- [ ] Open `docs/SPEC(20260909-120003).md` and determine whether it has a clear superseded banner near the top.
- [ ] Add a banner to each stale historical KittenTTS tracker/spec that can be mistaken for an active queue.
- [ ] Point readers to `docs/KITTENTTS_CLOSEOUT_REMEDIATION_TODO_2026-09-12.md` for active closeout status.
- [ ] Point readers to `docs/KITTENTTS_LEGACY_TODO_RECONCILIATION_2026-09-12.md` for legacy KTT mapping.
- [ ] Point readers to this TODO for post-closeout hardening work.
- [ ] Preserve historical content below the banner.
- [ ] Do not rewrite old evidence or mark human-only work complete in historical files.

**Acceptance**

- [ ] A reader opening a historical KittenTTS TODO/SPEC sees the superseded warning before stale unchecked rows.
- [ ] Historical files remain useful for provenance but cannot reasonably be confused with the active queue.

---

## LTH-500 — Preserve human-only `KCR-330` / `KTT-805` boundary

- [ ] Verify `docs/VOICE_SELECTION.md` still says human audition/default selection remains open unless the owner completes it.
- [ ] Verify `docs/KITTENTTS_CLOSEOUT_REMEDIATION_TODO_2026-09-12.md` still preserves the human-only gate.
- [ ] Verify `docs/KITTENTTS_LEGACY_TODO_RECONCILIATION_2026-09-12.md` still treats KTT-805 as owner-only/open.
- [ ] Do not change Bella from provisional/default based only on automated evidence.
- [ ] Do not describe ASR smoke as subjective voice acceptance.

**Acceptance**

- [ ] Human voice audition remains explicitly open and owner-owned.
- [ ] Automated tests remain framed as intelligibility/technical evidence only.

---

## LTH-600 — Source audit before PR

- [ ] Audit `src-tauri/src/ai/local_tts.rs` after changes.
- [ ] Audit `src-tauri/src/ai/local_tts/runtime/engine.rs` after changes.
- [ ] Audit any changed tests for privacy and no-fallback invariants.
- [ ] Audit changed docs for accurate active/superseded status.
- [ ] Confirm no generated artifacts were edited manually unless intentionally regenerated.
- [ ] Confirm no unrelated frontend, runtime, installer, manifest, or workflow changes slipped into the diff.

**Acceptance**

- [ ] The final diff contains only the intended hardening/documentation changes.

---

## LTH-700 — Validation

- [ ] Run or verify `git diff --check`.
- [ ] Run or verify Rust formatting.
- [ ] Run or verify Clippy.
- [ ] Run or verify complete Rust tests.
- [ ] Run or verify generated backend contract validation.
- [ ] Run or verify release/static metadata gates.
- [ ] Run or verify Local TTS packaging policy.
- [ ] Run KittenTTS production CPU acceptance if runtime-engine behavior changes beyond comments/test-only renames.
- [ ] Run KittenTTS ASR intelligibility smoke if waveform behavior or voice output could plausibly change.
- [ ] Record exact PR-head CI run IDs.

**Acceptance**

- [ ] Required exact-head validation passes before merge.
- [ ] Skipped heavyweight gates are explicitly justified by diff scope, not convenience.

---

## LTH-800 — Guarded merge and exact-master verification

- [ ] Open or update the PR against `master`.
- [ ] Confirm the PR head is exactly the commit intended for merge.
- [ ] Merge only with the expected exact PR head.
- [ ] Record the merged master SHA.
- [ ] Verify ordinary CI on exact merged master.
- [ ] Verify production CPU acceptance on exact merged master if required by diff scope.
- [ ] Verify ASR smoke on exact merged master if required by diff scope.
- [ ] Update this TODO with final evidence without creating an evidence-recursion loop.

**Acceptance**

- [ ] Master contains the hardening work.
- [ ] Exact-master validation evidence is recorded.

---

## Final checklist

- [ ] `PendingLocalSpeechSynthesizer` removed or explicitly justified by a current compatibility path.
- [ ] Stale Local TTS test names updated.
- [ ] Pre-runtime Local TTS validation messages are Local-specific, safe, and tested.
- [ ] ONNX `duration` output validation is either strengthened or explicitly documented as presence-only contract drift detection.
- [ ] Historical KittenTTS docs have superseded banners.
- [ ] `KCR-330` / `KTT-805` remains human-only/open.
- [ ] Exact-head validation passed.
- [ ] Guarded merge completed.
- [ ] Exact merged-master verification passed.

## Intentionally out of scope

- Human voice audition and final default Local KittenTTS voice selection.
- Real macOS x86_64 KittenTTS inference qualification, unless explicitly added as a future evidence-expansion task.
- Aspirational latency target expansion beyond the already-verified hard warm RTF gate.
