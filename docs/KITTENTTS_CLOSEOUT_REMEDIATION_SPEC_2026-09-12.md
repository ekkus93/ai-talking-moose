# AI Talking Moose — KittenTTS Closeout Remediation Specification

**Date:** 2026-09-12  
**Repository:** `ekkus93/ai-talking-moose`  
**Baseline reviewed:** `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f` (`master`)  
**Working branch:** `ralph/kcr-r4-r5-reconcile-closeout`  
**Primary tracker to close:** `docs/POST_KITTENTTS_CODE_REVIEW_REMEDIATION_TODO_2026-09-12.md`  
**Legacy tracker to reconcile:** `docs/TODO(20260909-120003).md`

---

## 1. Purpose

This specification defines the finite closeout slice required after the post-KittenTTS code review of `master` at `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f`.

The reviewed implementation is substantially complete through the real KittenTTS CPU acceptance and ASR intelligibility work. This closeout is not a rewrite of the Local TTS feature. It is a surgical remediation pass for the concrete issues found during review:

1. remove a stale Local TTS helper path that still returns the old placeholder provider;
2. reconcile stale project trackers with code and CI evidence;
3. update user/developer documentation to match shipped Local KittenTTS behavior;
4. record exact acceptance, ASR, performance, packaging, and macOS x86_64 boundary evidence;
5. add narrow tests where the review found command/helper coverage gaps;
6. perform final source audit, exact-head CI, guarded merge, and exact-master verification.

The human voice-quality/default-selection gate remains intentionally open. Automated ASR proves intelligibility; it does not choose the best Moose voice.

---

## 2. Non-goals

This closeout must not expand into unrelated feature work.

- Do not change the selected KittenTTS model identity unless the pinned artifact policy itself is proven wrong.
- Do not add a user-facing Local TTS thread-count setting. The 1/2/4-thread sweep supports the conservative production default.
- Do not replace the current production Local TTS runtime architecture.
- Do not claim macOS x86_64 real-inference acceptance unless a real macOS x86_64 run is actually executed.
- Do not treat automated ASR as human voice acceptance.
- Do not mark a tracker item complete without code, test, workflow, documentation, or explicit evidence support.
- Do not make ordinary CI download model weights.
- Do not introduce any Local-to-Google or Google-to-Local implicit fallback.

---

## 3. Review findings to remediate

### 3.1 Stale `AppState::get_speech_synthesizer()` Local path

`AppState::get_speech_synthesizer()` still returns `PendingLocalSpeechSynthesizer` when `TtsProvider::Local` is selected.

That helper is no longer the authoritative standalone speech path, but it remains a public app-state helper and conflicts with the now-implemented Local TTS runtime. It must be routed through `LocalSpeechSynthesizer` using the shared `Arc<LocalTtsRuntimeManager>` and the current settings snapshot.

The stale tests that expect Local TTS to fail before runtime integration must be updated. If no code path needs `PendingLocalSpeechSynthesizer` afterward, remove it and all tests dedicated to its placeholder behavior.

### 3.2 Stale trackers

`docs/POST_KITTENTTS_CODE_REVIEW_REMEDIATION_TODO_2026-09-12.md` and `docs/TODO(20260909-120003).md` are materially behind the implementation. Many R1/R2/R3 and old KTT items remain unchecked even though source and CI evidence show they are complete.

Both trackers must be reconciled from evidence. Human-only or deliberately deferred items must stay open.

### 3.3 Documentation drift

README and voice/privacy documentation still under-describe or misdescribe shipped Local KittenTTS behavior. Required documentation topics include:

- standalone TTS can be Google Gemini TTS or Local KittenTTS;
- Gemini Live voice remains separate cloud-native live audio;
- Google standalone voice, Local KittenTTS voice, and Gemini Live voice are separate settings;
- Local TTS installs with network access, then synthesizes offline after installation;
- Local TTS V1 is English-only;
- Local TTS is CPU-only/no GPU required;
- no Local-to-Google or Google-to-Local fallback exists;
- Local pitch is not exposed as a truthful capability;
- automated ASR intelligibility evidence is not human voice-quality acceptance;
- macOS x86_64 status is compile/package-boundary unless real inference is explicitly run.

### 3.4 Evidence closeout gap

The implementation has exact evidence for real-model acceptance, thread-sweep performance, offline privacy, cancellation, packaging policy, and ASR intelligibility, but the active closeout docs do not consolidate that evidence.

A reconciliation/evidence document or updated existing evidence sections must record:

- exact implementation head(s);
- exact merged master SHA(s);
- relevant CI run IDs;
- Linux x86_64 and macOS arm64 real KittenTTS acceptance status;
- 1/2/4-thread performance conclusion and retained production default;
- ASR all-eight-voice intelligibility result;
- known macOS x86_64 boundary status;
- exact statement that KCR-330 remains human-only and open.

### 3.5 Command/helper test coverage gap

Runtime-level and frontend diagnostics tests are strong. Command/helper tests should be tightened where practical:

- `AppState::get_speech_synthesizer()` should be covered for Google and Local routing semantics;
- Local helper should return the real Local provider and fail for missing/not-installed model without touching Google;
- diagnostics command composition should cover at least representative not-installed, installed/ready, and failed/error states if feasible without heavyweight real-model setup;
- all new tests must preserve sentinel/privacy assertions and avoid requiring model downloads in ordinary CI.

---

## 4. Required behavior after remediation

### 4.1 Local TTS helper routing

When `settings.tts_provider == TtsProvider::Local`, `AppState::get_speech_synthesizer()` must construct `LocalSpeechSynthesizer` with:

- `self.local_tts_runtime.clone()`;
- `settings.local_tts_model.clone()`;
- `settings.local_tts_voice.clone()`.

The helper must not return `PendingLocalSpeechSynthesizer` for production Local TTS selection.

### 4.2 Placeholder cleanup

If `PendingLocalSpeechSynthesizer` has no remaining production or test-only purpose after helper routing is fixed, remove it. If it is retained temporarily for a specific compatibility reason, the reason must be documented in code and it must not be reachable from production Local TTS selection.

### 4.3 No-fallback invariant

Every Local TTS failure mode must remain Local. Every Google TTS failure mode must remain Google. Provider errors must not cause implicit fallback to the other provider.

### 4.4 Offline invariant

Local TTS may use network access only for explicit installation/download before the offline phase. Real synthesis after installation must succeed with the network-denial harness active.

### 4.5 Diagnostics/privacy invariant

Diagnostics may expose provider/model/voice/install/runtime/timing/state/error category information. They must not expose:

- utterance text;
- raw PCM/audio bytes;
- credentials;
- unnecessary filesystem paths;
- model download temp paths;
- privacy sentinels.

### 4.6 Tracker truthfulness

A checked task means the implementation and evidence actually exist. An unchecked task means either the work is incomplete, deliberately deferred, or not actually executed on the claimed platform.

KCR-330 / KTT-805 must remain unchecked until the owner performs the human voice audition and explicitly accepts a default Local KittenTTS voice.

---

## 5. Documentation requirements

### 5.1 README

Update README to describe Local KittenTTS as a production standalone speech option, not merely generic Rust-owned speech output.

Required README content:

- standalone speech provider selector: Google Gemini TTS or Local KittenTTS;
- Local TTS is CPU-only and uses the existing playback/mouth-animation path after synthesis;
- Local TTS model install requires network and verification;
- post-install Local synthesis is offline and no-fallback;
- Gemini Live conversation voice is separate.

### 5.2 `docs/VOICE_SELECTION.md`

Update or replace stale Google-only voice-selection language.

Required content:

- Google standalone voice catalog and ownership;
- Local KittenTTS voice catalog and ownership;
- Gemini Live voice catalog and ownership;
- settings migration behavior;
- provider-aware audition behavior;
- Local pitch capability limitation;
- automated ASR smoke result and its limits;
- KCR-330 human audition still open.

### 5.3 `docs/PRIVACY.md`

Add Local TTS privacy section.

Required content:

- explicit download/install is the only expected network phase for Local TTS;
- Local synthesis after installation is offline;
- Local selected + Local failure does not send text to Google;
- Google selected + Google failure does not silently use Local;
- diagnostics omit utterance text/audio/credentials;
- Gemini Live remains cloud-native and separate from standalone TTS.

### 5.4 Evidence docs

Update the active remediation tracker or a dedicated evidence doc with exact run/commit facts.

Required content:

- R3 final PR head and merged master SHA;
- real-model Linux/macOS result summary;
- ASR all-eight-voice result summary;
- thread policy conclusion;
- packaging/license policy status;
- exact known remaining open item: KCR-330 human audition/default selection.

---

## 6. Validation requirements

### 6.1 Local/static validation where available

Run or explicitly delegate unavailable checks:

- `git diff --check`;
- Rust formatting;
- Rust Clippy;
- Rust unit tests;
- generated backend contract validation;
- Tauri command registration/shape checks;
- frontend format/lint/typecheck/tests when dependencies are available;
- Local TTS packaging-policy script;
- dependency/license collection gate.

If the local environment cannot run a gate, do not claim it passed locally. Record that CI is the authority for that gate.

### 6.2 Exact-head CI validation

The final closeout PR head must pass ordinary CI on the exact commit being merged.

If only docs are changed, the docs-only fast path may be sufficient if repository policy classifies it that way. If Rust/source changes are included, ordinary Rust CI must run and pass.

### 6.3 Real-model workflow validation

If the closeout changes Local TTS runtime, synthesis, installer, ASR, native runtime, or real-model workflow code, rerun the explicit real-model acceptance and ASR workflows on the exact final head.

If the closeout is docs/tracker-only after the helper fix is separately qualified, do not recursively create evidence commits solely to record evidence.

### 6.4 Guarded merge and exact master

Merge only after the final PR head is current and green. Use expected-head guard behavior where available. After merge, verify the exact merged master SHA and record the final result.

---

## 7. Completion criteria

This closeout is complete when:

1. `AppState::get_speech_synthesizer()` no longer routes Local to the placeholder provider;
2. no stale placeholder tests assert pre-integration Local failure as production behavior;
3. trackers accurately reflect completed, deferred, and open tasks;
4. README, voice docs, privacy docs, and evidence docs describe current production behavior;
5. macOS x86_64 claims are limited to evidence actually run;
6. KCR-330 remains explicitly open until human audition is done;
7. exact final PR head CI passes;
8. the final PR is guarded-merged;
9. exact merged master is verified;
10. no source audit item remains unresolved except the intentionally human-only voice acceptance gate.
