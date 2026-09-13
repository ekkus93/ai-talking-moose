# AI Talking Moose — Legacy KittenTTS TODO Reconciliation

**Date:** 2026-09-12  
**Legacy tracker:** `docs/TODO(20260909-120003).md`  
**Closeout TODO:** `docs/KITTENTTS_CLOSEOUT_REMEDIATION_TODO_2026-09-12.md`  
**Reviewed master:** `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f`

This document reconciles the older KTT tracker without rewriting its historical task log. Treat this document plus the closeout TODO as the authoritative status for KittenTTS closeout.

---

## Reconciliation rules

- Mark an old KTT item complete only when there is code, test, workflow, or exact CI/evidence support.
- Do not mark owner-only human audition/default voice selection complete from automated ASR evidence.
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
| KTT-805 | Open / owner-only | Automated ASR smoke passed all eight voices, but human audition/default Local voice selection remains required. |
| KTT-900 / KTT-901 | Complete for R3 | Exact R3 PR-head and merged-master ordinary CI, real-model CPU acceptance, and ASR smoke passed. Final closeout PR-head/master CI remains tracked in the closeout TODO. |
| KTT-902 / KTT-903 | Complete after docs/evidence commits | README, voice-selection docs, privacy docs, evidence docs, and this reconciliation doc record current behavior and limitations. |
| KTT-1000 through KTT-1003 | Pending until closeout PR merge | These final gates should be checked only after exact final closeout PR-head CI passes, guarded merge completes, and exact merged master verification passes. |

---

## Items that must remain explicitly open

### KCR-330 / KTT-805 — human Local voice audition/default choice

Automated ASR smoke passed for all eight Kitten voices, but it only demonstrates machine-recognizable intelligibility under the smoke-test phrase and thresholds. It does not decide:

- best Moose comedic fit;
- naturalness;
- timbre;
- fatigue over repeated playback;
- artifact/noise acceptability;
- final default Local voice.

This remains an owner decision.

### macOS x86_64 real inference

The current evidence supports macOS x86_64 as a compile/package/provenance boundary, not as a real KittenTTS inference target. Any old checkbox specifically requiring real x86_64 Kitten inference must remain unchecked until a future workflow runs and records it.

### Aspirational latency thresholds

The evidence supports the hard warm RTF `< 1.0` gate. It does not prove every aspirational latency target on every accepted platform. Those targets must not be represented as universally complete.

---

## Final closeout dependency

This legacy reconciliation becomes final only after the closeout PR itself passes exact-head CI and is merged with exact-master verification. Until then, treat KTT-1000 through KTT-1003 as pending final gates.
