# AI Talking Moose — KittenTTS Closeout Remediation TODO

**Date:** 2026-09-12  
**Companion spec:** `docs/KITTENTTS_CLOSEOUT_REMEDIATION_SPEC_2026-09-12.md`  
**Evidence:** `docs/KITTENTTS_CLOSEOUT_EVIDENCE_2026-09-12.md`  
**Legacy reconciliation:** `docs/KITTENTTS_LEGACY_TODO_RECONCILIATION_2026-09-12.md`  
**Baseline reviewed:** `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f` (`master`)  
**Working branch:** `ralph/kcr-r4-r5-reconcile-closeout`  
**Status:** Closeout implementation in progress; final PR-head CI and merge still pending.

This TODO is the finite remediation queue for issues found during the post-KittenTTS code review. The core Local TTS implementation is already present; this queue closes stale helper routing, tracker/documentation drift, evidence recording, final audit, and exact-head validation.

Do not mark KCR-330 / KTT-805 complete from automated ASR alone. Human voice audition/default selection remains owner-only.

---

## KCR-C000 — Preserve baseline and scope

- [x] Confirm the branch starts from the reviewed master lineage.
- [x] Record the exact reviewed baseline SHA: `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f`.
- [x] Keep this closeout limited to code-review findings and tracker/doc/evidence reconciliation.
- [x] Do not add unrelated Local TTS features.
- [x] Do not introduce a user-facing Local TTS thread-count setting.
- [x] Do not claim human voice acceptance from ASR evidence.

**Acceptance**

- [ ] Final PR description lists only the closeout/remediation scope.
- [x] Any intentionally deferred item is explicitly recorded rather than silently checked off.

---

## KCR-C100 — Route legacy `AppState` Local TTS helper through the real shared runtime

- [x] Replace `PendingLocalSpeechSynthesizer` in `AppState::get_speech_synthesizer()` with `LocalSpeechSynthesizer`.
- [x] Pass `self.local_tts_runtime.clone()` into the Local synthesizer.
- [x] Pass `settings.local_tts_model.clone()` into the Local synthesizer.
- [x] Pass `settings.local_tts_voice.clone()` into the Local synthesizer.
- [x] Ensure the Google branch remains unchanged and still uses Google auth/model/voice settings.
- [x] Avoid changing the authoritative standalone speech controller path except where tests prove it is necessary.
- [x] Preserve provider-neutral error semantics.
- [x] Preserve no-fallback behavior: Local helper failures remain Local failures and do not call Google.

**Acceptance**

- [x] `AppState::get_speech_synthesizer()` no longer returns the placeholder provider for production Local selection.
- [x] The helper uses the same shared `Arc<LocalTtsRuntimeManager>` owned by `AppState`.
- [x] Missing/not-installed Local model fails as a Local setup/model error, not a Google fallback.

---

## KCR-C101 — Remove or quarantine `PendingLocalSpeechSynthesizer`

- [x] Search the repository for `PendingLocalSpeechSynthesizer` references.
- [ ] Remove `PendingLocalSpeechSynthesizer` if no longer needed.
- [ ] If retained, document the exact compatibility reason in code.
- [x] Prove it is not reachable from production Local TTS selection through `AppState::get_speech_synthesizer()`.
- [ ] Remove placeholder-specific tests that assert pre-integration Local failure as current production behavior.

**Acceptance**

- [x] No production Local TTS route can hit the stale placeholder.
- [ ] Tests describe current production behavior, not historical pre-integration behavior.

**Note:** The production-routing defect is fixed. The obsolete placeholder cleanup is still a source-hygiene item unless removed or explicitly quarantined before final merge.

---

## KCR-C110 — Add/update direct helper tests

- [ ] Add or update a test proving Google selection still constructs the Google standalone synthesizer path and fails auth without a saved key.
- [x] Add or update a test proving Local selection constructs the real Local provider path.
- [x] Add or update a test proving Local selection with an unavailable model fails closed without exposing utterance text.
- [ ] Add or update a test proving Local helper cancellation returns provider-neutral cancellation before runtime use.
- [x] Ensure the tests do not download real model artifacts in ordinary CI.

**Acceptance**

- [x] Ordinary Rust tests catch regression from real Local helper routing back to the placeholder at the helper-routing level.
- [x] Ordinary Rust tests catch accidental Local-to-Google fallback from the helper path.

---

## KCR-C200 — Reconcile original active remediation TODO

- [ ] Update `docs/POST_KITTENTTS_CODE_REVIEW_REMEDIATION_TODO_2026-09-12.md` directly, or explicitly supersede it with this closeout TODO and the evidence/reconciliation docs.
- [x] Keep KCR-330 open and explicitly labeled human-only.
- [ ] Mark final KCR-500/KCR-501/KCR-502/KCR-503 only after final source audit, exact-head CI, guarded merge, and exact-master verification complete.

**Acceptance**

- [x] Completed implementation work is no longer treated as missing in the closeout tracker.
- [x] No human-only or not-run evidence gate is falsely claimed complete here.

---

## KCR-C210 — Reconcile `docs/TODO(20260909-120003).md`

- [x] Create `docs/KITTENTTS_LEGACY_TODO_RECONCILIATION_2026-09-12.md` mapping legacy KTT task ranges to current evidence.
- [x] Record that KTT-805 remains owner-only/open.
- [x] Record macOS x86_64 as compile/package/provenance boundary unless future real inference evidence exists.
- [x] Record that aspirational latency targets are not universally proved unless measured evidence satisfies them.
- [ ] Optionally update the legacy tracker itself if historical TODO mutation is desired; otherwise treat the reconciliation doc as authoritative.

**Acceptance**

- [x] Every completed old KTT range has a concrete code/test/workflow/evidence basis in the reconciliation doc.
- [x] Every open/partial old KTT range is explicitly listed with the reason it remains open/partial.

---

## KCR-C300 — Update README for production Local KittenTTS behavior

- [x] Document standalone speech provider options: Google Gemini TTS and Local KittenTTS.
- [x] Document that Gemini Live voice conversations remain separate cloud-native live audio.
- [x] Document that Local TTS V1 is CPU-only/no-GPU-required.
- [x] Document that Local TTS V1 is English-only.
- [x] Document that Local model installation requires network access and verification.
- [x] Document that synthesis after installation runs offline.
- [x] Document that there is no Local-to-Google or Google-to-Local fallback.
- [x] Document that Local TTS reuses the existing playback/mouth-animation path after producing PCM.

**Acceptance**

- [x] README describes current shipped behavior rather than generic or aspirational speech output.

---

## KCR-C310 — Update voice-selection documentation

- [x] Document Google standalone voice ownership and catalog behavior.
- [x] Document Local KittenTTS voice ownership and the eight Kitten voice IDs.
- [x] Document Gemini Live voice ownership as separate from standalone TTS.
- [x] Document settings migration for split Google/Local/Live voices.
- [x] Document provider-aware audition behavior.
- [x] Document that Local audition requires installed/verified Local model assets.
- [x] Document that Local pitch is disabled/not exposed as a truthful capability.
- [x] Document that rate control is truthful for both providers but provider semantics differ.
- [x] Document ASR intelligibility smoke result and its limitations.
- [x] Document that human KCR-330/KTT-805 voice-quality acceptance remains open.

**Acceptance**

- [x] Voice docs no longer describe Google/Fenrir-only behavior as the production voice-selection model.

---

## KCR-C320 — Update privacy documentation

- [x] Document Local TTS install-time network use.
- [x] Document post-install offline synthesis.
- [x] Document no Local-to-Google fallback on Local error.
- [x] Document no Google-to-Local fallback on Google error.
- [x] Document diagnostics/logging exclusions for utterance text, raw audio, credentials, and unnecessary filesystem paths.
- [x] Document that Gemini Live remains cloud-native and separate from standalone Local TTS.
- [x] Document the real network-denial acceptance evidence at a summary level.

**Acceptance**

- [x] Privacy docs accurately describe where standalone speech text can go for each provider choice.

---

## KCR-C330 — Record consolidated evidence

- [x] Record R3 final qualified PR head: `5280b9246b4af78bc47020fb079120f1b935a188`.
- [x] Record R3 merged master SHA: `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f`.
- [x] Record exact ordinary CI run ID `34737621353`.
- [x] Record exact real KittenTTS CPU acceptance run ID `34737621401`.
- [x] Record exact ASR intelligibility smoke run ID `34737621335`.
- [x] Summarize Linux x86_64 real acceptance outcome.
- [x] Summarize macOS arm64 real acceptance outcome.
- [x] Summarize ASR all-eight-voice outcome.
- [x] Summarize 1/2/4-thread evidence and production default decision.
- [x] Record that macOS x86_64 is compile/package-boundary unless real inference was actually run.
- [x] Record that KCR-330 / KTT-805 human audition remains open.

**Acceptance**

- [x] A reader can determine exactly what was tested, where it was tested, and what remains untested/deferred.

---

## KCR-C400 — Tighten command-level diagnostics coverage where practical

- [x] Add a command-composition diagnostics test for not-installed Local TTS state without real model downloads.
- [x] Add a command-composition diagnostics test for installed/ready state using safe synthetic status/runtime fixtures.
- [x] Add a command-composition diagnostics test for safe failed/error categories.
- [x] Preserve existing runtime lifecycle tests.
- [x] Preserve existing frontend diagnostics privacy tests.
- [x] Ensure diagnostics tests never expose utterance text, audio bytes, credentials, or full temp paths.

**Acceptance**

- [x] Diagnostics command composition has representative Rust-side coverage in addition to runtime and frontend tests.

---

## KCR-C500 — Run source audit before final PR

- [x] `AppState::get_speech_synthesizer()` helper routing audited.
- [x] Authoritative standalone speech controller routing audited in prior review; unchanged by closeout branch.
- [x] Settings snapshot ownership audited.
- [x] Google standalone voice ownership audited.
- [x] Local standalone voice ownership audited.
- [x] Gemini Live voice ownership audited.
- [x] Local model catalog and artifact pinning audited in prior R3 review; unchanged by closeout branch.
- [x] Installer integrity, cancellation, and path safety audited in prior R3 review; unchanged by closeout branch.
- [x] Runtime load/reuse/invalidation/unload behavior audited in prior R3 review; unchanged by closeout branch.
- [x] Async/blocking boundaries for load and inference audited in prior R3 review; unchanged by closeout branch.
- [x] Cancellation semantics audited.
- [x] PCM adaptation and playback reuse audited in prior R3 review; unchanged by closeout branch.
- [x] No Local-to-Google fallback audited.
- [x] No Google-to-Local fallback audited.
- [x] Diagnostics/logging privacy audited.
- [x] Native runtime packaging/load paths audited in prior R3 review; unchanged by closeout branch.
- [x] Third-party license inventory audited in prior R3 review; unchanged by closeout branch.
- [x] Ordinary CI model-weight-free policy audited.
- [x] ASR smoke scope and limits audited.

**Acceptance**

- [ ] No mandatory defect from the closeout spec remains unresolved except the human-only KCR-330/KTT-805 gate.

---

## KCR-C600 — Run local/static validation available in the implementation environment

- [x] `git diff --check` passed locally against the uploaded master snapshot after closeout edits.
- [ ] Rust formatting passes, or is delegated to exact-head CI if local Rust is unavailable.
- [ ] Rust Clippy passes, or is delegated to exact-head CI if local Rust is unavailable.
- [ ] Rust tests pass, or are delegated to exact-head CI if local Rust is unavailable.
- [x] Generated backend contract / Tauri command registration validation passed locally via `node scripts/check_tauri_command_contract.mjs`.
- [ ] Frontend format/lint/typecheck/tests pass, or are delegated to exact-head CI if dependencies are unavailable.
- [x] Local TTS packaging policy passed locally via `python3 scripts/check_local_tts_packaging_policy.py`.
- [ ] Dependency/license collection gate passes, or is delegated to exact-head CI.

**Acceptance**

- [x] Validation notes distinguish actual local passes from CI-delegated gates.

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

- [x] Decide whether final closeout source changes require explicit real-model KittenTTS acceptance rerun.
- [x] Decide whether final closeout source changes require ASR intelligibility smoke rerun.
- [x] Record decision: this branch does not change the Local TTS runtime engine, native runtime loading, model manifests, ASR round-trip implementation, or real acceptance workflows. Exact R3 merged-master real-model/ASR evidence remains applicable. Re-run only if a later commit touches runtime/synthesis/ASR/native/workflow code.
- [x] Do not create evidence recursion solely to record evidence.

**Acceptance**

- [x] Expensive workflows are neither skipped when needed nor rerun pointlessly for evidence-only commits.

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

- [x] Stale `AppState::get_speech_synthesizer()` Local placeholder removed or unreachable.
- [ ] Direct helper tests fully reflect current Local TTS behavior.
- [x] Active closeout TODO reconciled.
- [x] Legacy KTT TODO reconciled by dedicated reconciliation doc.
- [x] README updated for Local KittenTTS production behavior.
- [x] Voice-selection docs updated for Google/Local/Live split ownership.
- [x] Privacy docs updated for Local TTS install/offline/no-fallback boundaries.
- [x] Evidence doc records exact R3 SHAs and CI runs.
- [x] Linux x86_64 real-model acceptance evidence recorded.
- [x] macOS arm64 real-model acceptance evidence recorded.
- [x] ASR all-eight-voice smoke evidence recorded.
- [x] 1/2/4-thread policy evidence recorded; production default remains justified.
- [x] macOS x86_64 claim is truthful and limited to actual evidence.
- [x] KCR-330/KTT-805 human audition/default selection remains open unless owner explicitly completes it.
- [ ] Local/static validation completed or explicitly delegated through exact-head CI.
- [ ] Exact final PR-head CI passed.
- [ ] Final source audit found no mandatory unresolved issue except human-only audition.
- [ ] Guarded merge completed.
- [ ] Exact merged master verification completed.
