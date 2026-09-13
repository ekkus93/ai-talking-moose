# AI Talking Moose — KittenTTS Closeout Remediation TODO

**Date:** 2026-09-12  
**Companion spec:** `docs/KITTENTTS_CLOSEOUT_REMEDIATION_SPEC_2026-09-12.md`  
**Baseline reviewed:** `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f` (`master`)  
**Working branch:** `ralph/kcr-r4-r5-reconcile-closeout`  
**Status:** Active closeout queue

This TODO is the finite remediation queue for issues found during the post-KittenTTS code review. It is intentionally narrower than the original KittenTTS implementation tracker. The core Local TTS implementation is already present; this queue closes stale helper code, tracker/documentation drift, evidence recording, final audit, and exact-head validation.

Do not mark KCR-330 / KTT-805 complete from automated ASR alone. Human voice audition/default selection remains owner-only.

---

## KCR-C000 — Preserve baseline and scope

- [ ] Confirm the branch starts from the reviewed master lineage.
- [ ] Record the exact reviewed baseline SHA: `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f`.
- [ ] Keep this closeout limited to code-review findings and tracker/doc/evidence reconciliation.
- [ ] Do not add unrelated Local TTS features.
- [ ] Do not introduce a user-facing Local TTS thread-count setting.
- [ ] Do not claim human voice acceptance from ASR evidence.

**Acceptance**

- [ ] Final PR description lists only the closeout/remediation scope.
- [ ] Any intentionally deferred item is explicitly recorded rather than silently checked off.

---

## KCR-C100 — Route legacy `AppState` Local TTS helper through the real shared runtime

Fix the stale direct helper path found in review.

- [ ] Replace `PendingLocalSpeechSynthesizer` in `AppState::get_speech_synthesizer()` with `LocalSpeechSynthesizer`.
- [ ] Pass `self.local_tts_runtime.clone()` into the Local synthesizer.
- [ ] Pass `settings.local_tts_model.clone()` into the Local synthesizer.
- [ ] Pass `settings.local_tts_voice.clone()` into the Local synthesizer.
- [ ] Ensure the Google branch remains unchanged and still uses Google auth/model/voice settings.
- [ ] Avoid changing the authoritative standalone speech controller path except where tests prove it is necessary.
- [ ] Preserve provider-neutral error semantics.
- [ ] Preserve no-fallback behavior: Local helper failures must remain Local failures and must not call Google.

**Acceptance**

- [ ] `AppState::get_speech_synthesizer()` no longer returns the placeholder provider for production Local selection.
- [ ] The helper uses the same shared `Arc<LocalTtsRuntimeManager>` owned by `AppState`.
- [ ] Missing/not-installed Local model fails as a Local setup/model error, not a Google fallback.

---

## KCR-C101 — Remove or quarantine `PendingLocalSpeechSynthesizer`

- [ ] Search the repository for all `PendingLocalSpeechSynthesizer` references.
- [ ] Remove `PendingLocalSpeechSynthesizer` if no longer needed.
- [ ] If retained, document the exact compatibility reason in code.
- [ ] If retained, prove it is not reachable from production Local TTS selection.
- [ ] Remove placeholder-specific tests that assert pre-integration Local failure as current production behavior.

**Acceptance**

- [ ] No production Local TTS route can hit the stale placeholder.
- [ ] Tests describe current production behavior, not historical pre-integration behavior.

---

## KCR-C110 — Add/update direct helper tests

- [ ] Add or update a test proving Google selection still constructs the Google standalone synthesizer path and fails auth without a saved key.
- [ ] Add or update a test proving Local selection constructs the real Local provider path.
- [ ] Add or update a test proving Local selection with an unavailable model fails closed without exposing utterance text.
- [ ] Add or update a test proving Local helper cancellation returns provider-neutral cancellation before runtime use.
- [ ] Ensure the tests do not download real model artifacts in ordinary CI.

**Acceptance**

- [ ] Ordinary Rust tests catch regression from real Local helper routing back to the placeholder.
- [ ] Ordinary Rust tests catch accidental Local-to-Google fallback from the helper path.

---

## KCR-C200 — Reconcile `docs/POST_KITTENTTS_CODE_REVIEW_REMEDIATION_TODO_2026-09-12.md`

Update the active remediation tracker from implementation and CI evidence.

- [ ] Mark KCR-001 complete if baseline/scope evidence remains correct.
- [ ] Mark KCR-100 through KCR-130 according to diagnostics code, generated contract, frontend panel, and privacy/lifecycle tests.
- [ ] Mark KCR-200 through KCR-220 according to native runtime, packaging, license, and model-weight-free evidence.
- [ ] Mark KCR-300 through KCR-320 according to exact real-model acceptance, thread-sweep, ASR, and practical CI evidence.
- [ ] Keep KCR-330 open and explicitly labeled human-only.
- [ ] Mark KCR-400/KCR-410/KCR-420 only after tracker/docs/evidence updates land.
- [ ] Mark KCR-500/KCR-501/KCR-502/KCR-503 only after final source audit, exact-head CI, guarded merge, and exact-master verification complete.

**Acceptance**

- [ ] The active remediation TODO no longer falsely implies completed implementation work is missing.
- [ ] The active remediation TODO no longer falsely claims any human-only or not-run evidence gate is complete.

---

## KCR-C210 — Reconcile `docs/TODO(20260909-120003).md`

Reconcile the older KTT tracker from code, tests, workflows, and evidence.

- [ ] KTT-300: verify shared runtime manager, lazy load, warm reuse, coordination, reload identity, unload, offline inference, and safe runtime state.
- [ ] KTT-301: verify CPU Kitten inference, exact runtime/G2P stack, voice validation, truthful speed handling, no fake pitch, finite PCM, and typed errors.
- [ ] KTT-302/KTT-303: verify provider-aware cancellation and blocking CPU isolation.
- [ ] KTT-304: verify `AudioStreamData` conversion and existing playback reuse.
- [ ] KTT-305/KTT-600/KTT-601/KTT-602: verify diagnostics implementation, generated contract, UI rendering, and privacy tests.
- [ ] KTT-400/KTT-401/KTT-402/KTT-403/KTT-404: verify typed provider routing, immutable settings snapshot, no-fallback tests, offline synthesis, and sentinel privacy tests.
- [ ] KTT-500 through KTT-505: verify provider selector, separate voice controls, lifecycle UI, provider-aware audition, rate/pitch capability handling, and switch/error/retry UX.
- [ ] KTT-700 through KTT-704: verify native runtime target handling, packaging policy, license inventory, and model-weight-free ordinary builds.
- [ ] KTT-800 through KTT-804: verify real-model harness, Linux/macOS arm64 acceptance, macOS x86_64 boundary status, and thread/performance policy.
- [ ] KTT-805: leave human voice audition/default choice open.
- [ ] KTT-900 through KTT-903: verify CI, docs, and reconciliation evidence only after corresponding updates are committed.
- [ ] KTT-1000 through KTT-1003: mark final gates only after final exact-head and exact-master validation.

**Acceptance**

- [ ] Every checked old KTT item has a concrete code/test/workflow/evidence basis.
- [ ] Every unchecked old KTT item is either genuinely incomplete, deliberately deferred, or not actually executed on the claimed platform.

---

## KCR-C300 — Update README for production Local KittenTTS behavior

- [ ] Document standalone speech provider options: Google Gemini TTS and Local KittenTTS.
- [ ] Document that Gemini Live voice conversations remain separate cloud-native live audio.
- [ ] Document that Local TTS V1 is CPU-only/no-GPU-required.
- [ ] Document that Local TTS V1 is English-only.
- [ ] Document that Local model installation requires network access and verification.
- [ ] Document that synthesis after installation runs offline.
- [ ] Document that there is no Local-to-Google or Google-to-Local fallback.
- [ ] Document that Local TTS reuses the existing playback/mouth-animation path after producing PCM.

**Acceptance**

- [ ] README describes current shipped behavior rather than generic or aspirational speech output.

---

## KCR-C310 — Update voice-selection documentation

Update `docs/VOICE_SELECTION.md` or an equivalent voice-selection doc.

- [ ] Document Google standalone voice ownership and catalog behavior.
- [ ] Document Local KittenTTS voice ownership and the eight Kitten voice IDs.
- [ ] Document Gemini Live voice ownership as separate from standalone TTS.
- [ ] Document settings migration for split Google/Local/Live voices.
- [ ] Document provider-aware audition behavior.
- [ ] Document that Local audition requires installed/verified Local model assets.
- [ ] Document that Local pitch is disabled/not exposed as a truthful capability.
- [ ] Document that rate control is truthful for both providers but provider semantics differ.
- [ ] Document ASR intelligibility smoke result and its limitations.
- [ ] Document that human KCR-330/KTT-805 voice-quality acceptance remains open.

**Acceptance**

- [ ] Voice docs no longer describe Google/Fenrir-only behavior as the production voice-selection model.

---

## KCR-C320 — Update privacy documentation

Update `docs/PRIVACY.md` or an equivalent privacy doc.

- [ ] Document Local TTS install-time network use.
- [ ] Document post-install offline synthesis.
- [ ] Document no Local-to-Google fallback on Local error.
- [ ] Document no Google-to-Local fallback on Google error.
- [ ] Document diagnostics/logging exclusions for utterance text, raw audio, credentials, and unnecessary filesystem paths.
- [ ] Document that Gemini Live remains cloud-native and separate from standalone Local TTS.
- [ ] Document the real network-denial acceptance evidence at a summary level.

**Acceptance**

- [ ] Privacy docs accurately describe where standalone speech text can go for each provider choice.

---

## KCR-C330 — Record consolidated evidence

Create or update a dedicated evidence/reconciliation document, or update the active remediation tracker with an evidence section.

- [ ] Record R3 final qualified PR head: `5280b9246b4af78bc47020fb079120f1b935a188`.
- [ ] Record R3 merged master SHA: `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f`.
- [ ] Record exact ordinary CI run IDs used for final qualification.
- [ ] Record exact real KittenTTS CPU acceptance run IDs used for final qualification.
- [ ] Record exact ASR intelligibility smoke run IDs used for final qualification.
- [ ] Summarize Linux x86_64 real acceptance outcome.
- [ ] Summarize macOS arm64 real acceptance outcome.
- [ ] Summarize ASR all-eight-voice outcome.
- [ ] Summarize 1/2/4-thread evidence and production default decision.
- [ ] Record that macOS x86_64 is compile/package-boundary unless real inference was actually run.
- [ ] Record that KCR-330 / KTT-805 human audition remains open.

**Acceptance**

- [ ] A reader can determine exactly what was tested, where it was tested, and what remains untested/deferred.

---

## KCR-C400 — Tighten command-level diagnostics coverage where practical

- [ ] Add a command-level diagnostics test for not-installed Local TTS state if feasible without real model downloads.
- [ ] Add a command-level diagnostics test for installed/ready state using safe fixture or storage-marker setup if feasible.
- [ ] Add a command-level diagnostics test for safe failed/error category if feasible.
- [ ] Preserve existing runtime lifecycle tests.
- [ ] Preserve existing frontend diagnostics privacy tests.
- [ ] Ensure diagnostics tests never expose utterance text, audio bytes, credentials, or full temp paths.

**Acceptance**

- [ ] Diagnostics command composition has representative Rust-side coverage in addition to runtime and frontend tests, or the exact infeasibility reason is documented.

---

## KCR-C500 — Run source audit before final PR

Re-read and verify the final source state for:

- [ ] `AppState::get_speech_synthesizer()` helper routing.
- [ ] authoritative standalone speech controller routing.
- [ ] settings snapshot ownership.
- [ ] Google standalone voice ownership.
- [ ] Local standalone voice ownership.
- [ ] Gemini Live voice ownership.
- [ ] Local model catalog and artifact pinning.
- [ ] installer integrity, cancellation, and path safety.
- [ ] runtime load/reuse/invalidation/unload behavior.
- [ ] async/blocking boundaries for load and inference.
- [ ] cancellation semantics.
- [ ] PCM adaptation and playback reuse.
- [ ] no Local-to-Google fallback.
- [ ] no Google-to-Local fallback.
- [ ] diagnostics/logging privacy.
- [ ] native runtime packaging/load paths.
- [ ] third-party license inventory.
- [ ] ordinary CI model-weight-free policy.
- [ ] ASR smoke scope and limits.

**Acceptance**

- [ ] No mandatory defect from the closeout spec remains unresolved except the human-only KCR-330/KTT-805 gate.

---

## KCR-C600 — Run local/static validation available in the implementation environment

Run locally where available; otherwise explicitly delegate to CI.

- [ ] `git diff --check` passes.
- [ ] Rust formatting passes, or is delegated to exact-head CI if local Rust is unavailable.
- [ ] Rust Clippy passes, or is delegated to exact-head CI if local Rust is unavailable.
- [ ] Rust tests pass, or are delegated to exact-head CI if local Rust is unavailable.
- [ ] generated backend contract validation passes.
- [ ] Tauri command registration/shape check passes.
- [ ] frontend format/lint/typecheck/tests pass, or are delegated to exact-head CI if dependencies are unavailable.
- [ ] Local TTS packaging policy passes.
- [ ] dependency/license collection gate passes.

**Acceptance**

- [ ] Validation notes distinguish actual local passes from CI-delegated gates.

---

## KCR-C610 — Pass exact final PR-head CI

- [ ] Open or update the closeout PR against `master`.
- [ ] Confirm the PR head is exactly the commit intended for merge.
- [ ] Confirm ordinary CI passes on that exact PR head.
- [ ] Confirm release/static/license gates pass on that exact PR head.
- [ ] Confirm generated backend contract gate passes on that exact PR head.
- [ ] If Rust source changed, confirm Rust fmt/Clippy/tests pass on that exact PR head.
- [ ] If frontend source changed, confirm frontend quality gates pass on that exact PR head.

**Acceptance**

- [ ] The final closeout PR is not merged from a stale or partially qualified head.

---

## KCR-C620 — Re-run expensive real-model/ASR workflows only when required

- [ ] Decide whether final closeout source changes require explicit real-model KittenTTS acceptance rerun.
- [ ] Decide whether final closeout source changes require ASR intelligibility smoke rerun.
- [ ] If runtime/synthesis/ASR/native/workflow code changed, rerun the relevant explicit workflow(s) on the exact final head.
- [ ] If only docs/tracker files changed after a separately qualified source fix, do not create evidence recursion solely to record evidence.
- [ ] Record the decision and exact run IDs if rerun.

**Acceptance**

- [ ] Expensive workflows are neither skipped when needed nor rerun pointlessly for evidence-only commits.

---

## KCR-C700 — Guarded merge and exact master verification

- [ ] Recheck PR mergeability immediately before merge.
- [ ] Merge only with the expected final head.
- [ ] Record exact merged master SHA.
- [ ] Verify ordinary CI on the exact merged master SHA.
- [ ] Verify any required explicit real-model/ASR post-merge workflows on the exact merged master SHA.
- [ ] Update closeout evidence without creating an infinite evidence-update loop.
- [ ] Delete/close obsolete closeout branch only after master verification is recorded, if branch cleanup is in scope.

**Acceptance**

- [ ] Master contains the closeout work and exact post-merge verification evidence.

---

## Final closeout checklist

- [ ] Stale `AppState::get_speech_synthesizer()` Local placeholder removed or unreachable.
- [ ] Direct helper tests reflect current Local TTS behavior.
- [ ] Active KCR remediation TODO reconciled.
- [ ] Legacy KTT TODO reconciled.
- [ ] README updated for Local KittenTTS production behavior.
- [ ] Voice-selection docs updated for Google/Local/Live split ownership.
- [ ] Privacy docs updated for Local TTS install/offline/no-fallback boundaries.
- [ ] Evidence doc records exact R3/R4/R5 SHAs and CI runs.
- [ ] Linux x86_64 real-model acceptance evidence recorded.
- [ ] macOS arm64 real-model acceptance evidence recorded.
- [ ] ASR all-eight-voice smoke evidence recorded.
- [ ] 1/2/4-thread policy evidence recorded; production default remains justified.
- [ ] macOS x86_64 claim is truthful and limited to actual evidence.
- [ ] KCR-330/KTT-805 human audition/default selection remains open unless owner explicitly completes it.
- [ ] Local/static validation available in the environment completed or explicitly delegated.
- [ ] Exact final PR-head CI passed.
- [ ] Final source audit found no mandatory unresolved issue except human-only audition.
- [ ] Guarded merge completed.
- [ ] Exact merged master verification completed.
