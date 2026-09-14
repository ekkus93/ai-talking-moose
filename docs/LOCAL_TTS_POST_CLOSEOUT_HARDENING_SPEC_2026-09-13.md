# AI Talking Moose — Local TTS Post-Closeout Hardening Spec

**Date:** 2026-09-13
**Companion TODO:** `docs/LOCAL_TTS_POST_CLOSEOUT_HARDENING_TODO_2026-09-13.md`
**Baseline:** `78ccea9838c8003332fc7499e7d5b6bc94623044` (`master`)
**Review source:** post-closeout Local KittenTTS code review against current master
**Status:** Completed by PR #111; historical scope reconciled after the later Luna/ASR-proxy decision

This spec defines a small post-closeout hardening batch for issues found after the KittenTTS technical closeout and the Local TTS runtime-boundary hardening PR. The core KittenTTS implementation, closeout remediation, Idle Banter implementation, and PR #109 runtime hardening are already complete and verified on master. This document must not be used to reopen those completed scopes.

The work here is cleanup, diagnostics precision, and documentation hygiene. It is intended to reduce future confusion and tighten Local TTS behavior at already-identified low-risk edges.

> **Later status note (2026-09-14):** PR #111 correctly preserved `KCR-330` / `KTT-805` as an owner-only gate because no owner voice decision existed when this hardening batch ran. PR #114 later added an automated all-eight-voice ASR audition proxy. The owner explicitly accepted that proxy for V1, and PR #115 changed the Local default to `Luna`. `KCR-330` / `KTT-805` is therefore closed for V1 by owner-approved ASR proxy evidence, without claiming subjective human listening. A future listening pass may override `Luna`, but it is not remaining V1 closeout work.

---

## Goals

1. Remove or definitively retire dead Local TTS placeholder code.
2. Rename stale tests so their names describe the current integrated runtime behavior.
3. Make pre-runtime Local TTS validation failures use safe Local-TTS-specific provider messages.
4. Clarify or strengthen the ONNX Runtime output-contract check for the unused `duration` output.
5. Add explicit superseded banners to historical KittenTTS `TODO` / `SPEC` files that still contain stale unchecked rows.
6. Preserve all existing provider separation, no-fallback, privacy, packaging, and real-acceptance invariants.

---

## Non-goals

- Do not change the pinned KittenTTS model, voice embeddings, CMUdict/G2P data, ONNX Runtime version, or artifact hashes.
- Do not change synthesis semantics, phonemization, tokenization, style-row selection, speaking-rate behavior, PCM conversion, playback, or mouth-animation behavior.
- Do not add a user-facing Local TTS thread-count setting.
- At the time of PR #111, do not mark `KCR-330` / `KTT-805` complete before an owner decision existed. This historical constraint was satisfied; the later owner-approved ASR-proxy decision and PR #115 close the V1 default-selection gate.
- Do not claim real macOS x86_64 KittenTTS inference unless a workflow actually runs inference on that platform.
- Do not treat old historical trackers as active queues after they have been superseded by reconciliation documents.

---

## Current verified baseline

Current master at the time this spec was written:

- `78ccea9838c8003332fc7499e7d5b6bc94623044`
- PR #109: `fix: harden Local TTS runtime boundaries`
- Post-merge CI: `34790686578` — pass
- Post-merge KittenTTS production CPU acceptance: `34790686592` — pass
- Post-merge KittenTTS ASR intelligibility smoke: `34790686499` — pass

The active KittenTTS closeout tracker remains:

- `docs/KITTENTTS_CLOSEOUT_REMEDIATION_TODO_2026-09-12.md`

At the time this spec was written, that tracker was technically complete except for owner-only `KCR-330` / `KTT-805`. That historical gate was later closed for V1 by the owner-approved ASR proxy and PR #115 (`Luna`).

---

## Finding LTH-100 — Retire `PendingLocalSpeechSynthesizer`

### Problem

`PendingLocalSpeechSynthesizer` remains in `src-tauri/src/ai/local_tts.rs` as a fail-closed compatibility bridge. Production Local TTS selection no longer uses it; the production helper and standalone paths route through `LocalSpeechSynthesizer` backed by the shared `LocalTtsRuntimeManager`.

Keeping this placeholder can confuse future reviewers into thinking Local TTS is still pending or partially integrated.

### Requirement

Remove `PendingLocalSpeechSynthesizer` unless a current direct caller requires it. If a current caller requires it, keep it only behind an explicit test-only or compatibility boundary and document the caller.

### Acceptance

- Repository search finds no production caller of `PendingLocalSpeechSynthesizer`.
- The placeholder is removed, or its remaining use is explicitly justified by a current code path.
- Tests that only prove the old placeholder behavior are removed or replaced with tests against current production Local TTS routing.
- Local TTS continues to fail closed when the model is not installed.
- No Google fallback is introduced.

---

## Finding LTH-110 — Rename stale Local TTS tests

### Problem

At least one test name still describes pre-integration behavior, for example `local_tts_selection_fails_closed_before_runtime_integration`. The behavior is still important, but Local TTS integration is now complete, so the test name is misleading.

### Requirement

Rename stale tests to describe current behavior precisely.

Suggested replacement wording:

- `local_tts_selection_fails_closed_when_model_is_not_installed`
- `local_tts_selection_does_not_fallback_or_leak_utterance_when_unavailable`

### Acceptance

- No test name says or implies Local TTS is still awaiting runtime integration.
- Renamed tests still cover the same invariants.
- No test coverage is deleted merely to silence the stale name.

---

## Finding LTH-200 — Use Local-specific safe messages for pre-runtime validation errors

### Problem

PR #109 preserved bounded Local-TTS-specific messages for errors returned by `LocalTtsRuntimeManager` and `LocalTtsRuntimeError`. However, pre-runtime validation in `LocalSpeechSynthesizer::inference_request()` still maps invalid configured model/voice through generic `ProviderError::from_kind(...)`, which can surface broad conversation-provider copy such as "conversation model" or "conversation session".

This is safe, but the wording is less accurate than the runtime-originated Local TTS errors.

### Requirement

Return explicit Local-TTS-specific `ProviderError` values for pre-runtime validation failures:

- unsupported Local TTS model catalog value;
- unsupported/unavailable Local TTS voice;
- cancelled-before-runtime behavior must remain provider-neutral cancellation.

Messages must remain bounded and must not include utterance text, audio bytes, credentials, absolute filesystem paths, or raw lower-level error strings.

### Acceptance

- Unknown configured Local TTS model maps to `ProviderErrorKind::Model` with Local-specific safe text.
- Unknown Local TTS voice maps to `ProviderErrorKind::Setup` with Local-specific safe text.
- Retryability stays consistent with the existing provider error policy.
- Tests prove messages do not contain the request text.
- No fallback to Google or any other provider is introduced.

---

## Finding LTH-300 — Clarify or strengthen ONNX `duration` output validation

### Problem

After PR #109, `KittenTtsRuntimeEngine` requires exactly two named ONNX outputs, `waveform` and `duration`, and reads `waveform` by name instead of position. The runtime checks that `duration` exists, but it does not extract or type-check `duration` because synthesis currently consumes only `waveform`.

That is a reasonable drift check, but the implementation should either make the limitation explicit or validate the `duration` tensor more strongly.

### Requirement

Choose one of these approaches:

1. **Clarify presence-only validation:** rename/comment the helper so it is clear that `duration` is required as a model-contract sentinel but not semantically consumed; or
2. **Strengthen validation:** extract `duration` as the expected tensor type/shape and fail closed if it is malformed.

Do not change the generated waveform or synthesis output solely because `duration` exists.

### Acceptance

- The output-contract helper name/comment reflects exactly what is validated.
- Tests cover missing `waveform`, missing `duration`, extra output, and wrong output count.
- If type/shape validation is implemented, tests or real acceptance cover malformed duration handling where practical.
- Real KittenTTS production CPU acceptance still passes on Linux x86_64 and macOS arm64.

---

## Finding LTH-400 — Add superseded banners to historical KittenTTS trackers/specs

### Problem

Historical files such as `docs/TODO(20260909-120003).md` and `docs/SPEC(20260909-120003).md` may still contain old unchecked rows or pre-closeout requirements. The reconciliation docs correctly identify the authoritative status, but future reviewers can still misread the historical files as active work.

### Requirement

Add a clear banner near the top of each historical KittenTTS tracker/spec stating that it is superseded and should not be used as the active queue.

The banner should point to:

- `docs/KITTENTTS_CLOSEOUT_REMEDIATION_TODO_2026-09-12.md`
- `docs/KITTENTTS_LEGACY_TODO_RECONCILIATION_2026-09-12.md`
- this post-closeout hardening TODO, once it exists

### Acceptance

- Historical docs remain preserved for provenance.
- A reader opening a historical file sees the superseded warning before old unchecked rows.
- The banner does not rewrite historical evidence or falsely mark human-only work complete.

---

## Finding LTH-500 — Preserve the then-human-only voice-selection boundary

### Historical requirement

When PR #111 implemented this hardening batch, `KCR-330` / `KTT-805` was still owner-only. Automated ASR evidence available at that time was intelligibility evidence and did not itself authorize a default-voice change. PR #111 therefore had to preserve Bella as the provisional Local default and keep the gate open.

That requirement was implemented correctly. The project later changed state through a separate, explicit decision: PR #114 added the deterministic ASR audition proxy; the owner accepted that proxy for V1; PR #115 selected `Luna`; exact PR-head and merged-master CI, production CPU acceptance, ASR smoke, and ASR audition all passed.

### Current interpretation

- `KCR-330` / `KTT-805` is closed for V1 by owner-approved ASR proxy evidence.
- `Luna` is the V1 Local KittenTTS default.
- No subjective human listening claim is made.
- Future subjective listening is an optional override/evidence-expansion path, not unfinished V1 closeout work.
- The historical PR #111 acceptance remains truthful because it describes the project state at the time that PR was implemented.

---

## Validation requirements

Before merging implementation of this spec:

1. Review exact diff against current master.
2. Run or rely on exact-head CI for:
   - Rust formatting;
   - Clippy;
   - complete Rust tests;
   - generated backend contract;
   - release/static metadata gates;
   - Local TTS packaging policy.
3. If runtime-engine behavior changes beyond comments/test names, run exact-head KittenTTS production CPU acceptance.
4. If waveform behavior or voice output could plausibly change, run exact-head KittenTTS ASR intelligibility smoke.
5. Merge only from an exact, verified PR head.
6. Verify post-merge CI on exact master.

---

## Completion definition

This hardening batch is complete when:

- the active TODO file for this spec is fully checked off with evidence;
- no stale Local TTS placeholder remains in production code;
- Local pre-runtime validation messages are Local-specific and safe;
- ONNX output-contract validation is either stronger or truthfully documented as presence-only for `duration`;
- historical KittenTTS trackers/specs carry superseded banners;
- exact-head and exact-master validation pass;
- the historical PR #111 human-only boundary remains documented, while current status records the later owner-approved ASR-proxy closeout and `Luna` default without claiming subjective listening.
