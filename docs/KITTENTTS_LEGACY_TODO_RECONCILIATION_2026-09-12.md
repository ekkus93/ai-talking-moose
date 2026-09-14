# AI Talking Moose — Legacy KittenTTS TODO Reconciliation

**Date:** 2026-09-12
**Legacy tracker:** `docs/TODO(20260909-120003).md`
**Closeout TODO:** `docs/KITTENTTS_CLOSEOUT_REMEDIATION_TODO_2026-09-12.md`
**Reviewed master:** `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f`
**Final closeout master:** `ddce257ce539806c83b41ed916a38b9d7adc7a41`
**V1 ASR-proxy default closeout master:** `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4`
**Selected V1 Local default:** `Luna`

This document reconciles the older KTT tracker without rewriting its historical task log. Treat this document plus the closeout TODO and evidence record as the authoritative status for KittenTTS closeout. The technical closeout gates completed through PR #101 and the final documentation bookkeeping completed through PR #102. Later, PR #114 added the automated ASR voice-audition proxy and PR #115 selected `Luna` as the V1 Local KittenTTS default by owner-approved ASR proxy evidence.

---

## Reconciliation rules

- Mark an old KTT item complete only when there is code, test, workflow, or exact CI/evidence support.
- Do not describe owner-approved ASR proxy evidence as subjective human listening evidence.
- Do not claim real macOS x86_64 inference unless a workflow actually runs KittenTTS inference on macOS x86_64.
- Do not check aspirational latency targets unless the measured evidence satisfies the target on the claimed platform.
- Keep ordinary CI model-weight-free; real-model acceptance remains explicit/heavyweight.

---

## Legacy KTT status map

| Legacy range | Status | Basis / limitation |
| --- | --- | --- |
| KTT-001 / KTT-002 / KTT-003 | Complete | Pre-existing provider-selection, settings, and generated-contract groundwork remains present. |
| KTT-100 through KTT-104 | Complete | Settings schema v4 splits Google standalone, Local standalone, and Gemini Live voice/model ownership; migrations preserve legacy intent. |
| KTT-200 through KTT-204 | Complete | Local TTS catalog, installer, pinned artifacts, checksum/size verification, platform manifest, cancellation, delete/retry, and runtime-use verification are implemented. |
| KTT-300 | Complete after closeout source fix | `AppState::get_speech_synthesizer()` now routes Local through `LocalSpeechSynthesizer` over the shared `Arc<LocalTtsRuntimeManager>`; production standalone routing was already authoritative through the speech controller. |
| KTT-301 | Complete | Real KittenTTS CPU inference uses pinned ONNX Runtime artifacts, Kitten model, CMUdict/G2P assets, voice validation, bounded input handling, finite PCM output, and typed provider errors. |
| KTT-302 | Complete | Cancellation is provider-aware and real in-flight cancellation is proved in the hardened acceptance workflow. |
| KTT-303 | Complete | Blocking model load/inference work is isolated from async executor workers and cancellation remains truthful. |
| KTT-304 | Complete | Local Kitten output adapts into the existing `AudioStreamData`/playback/mouth-animation path. |
| KTT-305 / KTT-600 / KTT-601 / KTT-602 | Complete after closeout diagnostics tests | Diagnostics include install state, runtime phase, sample rate, inference thread count, timing/RTF, safe categories, generated contract/UI coverage, and privacy/sentinel tests. |
| KTT-400 through KTT-404 | Complete | Provider routing snapshots settings, preserves Google/Local/Live separation, rejects fallback, proves offline synthesis, and protects sentinels from logs/status. |
| KTT-500 through KTT-505 | Complete | Settings UI supports provider selector, separate Google/Local/Live voice controls, Local install lifecycle, provider-aware audition, truthful rate/pitch capability handling, and fail-closed retry/error UX. |
| KTT-700 through KTT-704 | Complete with x86 caveat | Linux x86_64/macOS arm64 real runtime coverage exists; macOS x86_64 is compile/package/provenance boundary coverage unless future real inference evidence is added. Packaging/licensing/model-weight-free gates are present. |
| KTT-800 through KTT-802 | Complete | Hardened real-model acceptance exists for Linux x86_64 and macOS arm64, including all voices, invalid voice fail-closed behavior, network denial, cancellation, JSON evidence, and practical caching. |
| KTT-803 | Partially complete | macOS x86_64 compile/package/provenance boundary is covered. Real macOS x86_64 Kitten inference is not recorded and must remain unchecked if the legacy item requires real inference. |
| KTT-804 | Partially complete | Hard warm RTF `< 1.0` is proved on Linux x86_64 and macOS arm64; 2-thread production default is justified. Aspirational universal median `<= 0.5` and short-line `<= 1.5s` targets are not universally proved by the recorded macOS evidence. |
| KTT-805 | Complete for V1 by owner-approved ASR proxy | PR #114 added the automated all-eight-voice ASR audition proxy. PR #115 accepted its deterministic recommendation and changed the V1 Local default to `Luna`; this is not subjective human listening evidence. |
| KTT-900 / KTT-901 | Complete | Exact R3 PR-head and merged-master ordinary CI, real-model CPU acceptance, and ASR smoke passed. Closeout PR #101 exact-head CI and exact merged-master verification also passed. |
| KTT-902 / KTT-903 | Complete after docs/evidence commits | README, voice-selection docs, privacy docs, evidence docs, and this reconciliation doc record current behavior and limitations. |
| KTT-1000 through KTT-1003 | Complete for technical closeout | Closeout PR #101 passed exact-head ordinary CI and production KittenTTS CPU acceptance, was guarded-merged at the expected head, then passed exact merged-master ordinary CI and production CPU acceptance. PR #102 recorded the final evidence and passed docs-only exact-head and post-merge CI. |

---

## Remaining caveats and bounded evidence

### KCR-330 / KTT-805 — V1 Local default selected by owner-approved ASR proxy

`KCR-330` / `KTT-805` is closed for V1 by owner-approved ASR proxy evidence. The accepted V1 Local KittenTTS default is `Luna`.

Evidence:

- PR #114 head `037a0f0a61e13c76ddc0f9a16ef88adb2f38f141`, ASR voice audition run `34821757481` — PASS.
- PR #114 post-merge master `21fac6785f008b1d5ab7f46e9125c76ca4c087ad`, ASR voice audition run `34848251323` — PASS.
- PR #115 head `4a8bf77e4803b7898c717a3cbe145f9df9600c75`, ordinary CI `34853091162`, production CPU acceptance `34853091150`, ASR smoke `34853091234`, and ASR voice audition `34853091230` — PASS.
- PR #115 post-merge master `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4`, ordinary CI `34854193904`, production CPU acceptance `34854193873`, ASR smoke `34854193869`, and ASR voice audition `34854193968` — PASS.

This is not subjective human listening evidence. It does not prove best Moose comedic fit, timbre preference, naturalness, fatigue, or real-speaker character fit. A future subjective pass can still override `Luna`, but that is no longer a V1 closeout blocker.

### macOS x86_64 real inference

The current evidence supports macOS x86_64 as a compile/package/provenance boundary, not as a real KittenTTS inference target. Any old checkbox specifically requiring real x86_64 Kitten inference must remain unchecked until a future workflow runs and records it.

### Aspirational latency thresholds

The evidence supports the hard warm RTF `< 1.0` gate. It does not prove every aspirational latency target on every accepted platform. Those targets must not be represented as universally complete.

---

## Final closeout state

The closeout dependency is satisfied. PR #101 passed exact-head ordinary CI (`34755503886`) and production KittenTTS CPU acceptance (`34755503936`), was guarded-merged as `79423e8cc0ad48e796e964dee7eb51c3a440ddc4`, and then passed exact merged-master ordinary CI (`34755879197`) plus production KittenTTS CPU acceptance (`34755879210`). PR #102 recorded the final bookkeeping on master `ddce257ce539806c83b41ed916a38b9d7adc7a41` and passed post-merge docs-only CI (`34757108163`).

Treat KTT-1000 through KTT-1003 as complete for the technical closeout. Treat `KCR-330` / `KTT-805` as complete for V1 by owner-approved ASR proxy evidence after PR #115 selected `Luna` and exact PR-head plus post-merge validation passed. Do not describe that closeout as subjective human listening.
