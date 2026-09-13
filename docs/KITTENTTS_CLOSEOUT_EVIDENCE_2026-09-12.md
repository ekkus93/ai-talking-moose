# AI Talking Moose — KittenTTS Closeout Evidence

**Date:** 2026-09-12  
**Scope:** evidence for the KittenTTS closeout remediation queue.  
**Reviewed baseline:** `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f` (`master`).  
**R3 final qualified PR head:** `5280b9246b4af78bc47020fb079120f1b935a188`.  
**R3 merged master SHA:** `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f`.

This document records what was actually tested and what remains deliberately deferred. It must not be used to mark the owner-only human voice-quality decision complete.

---

## Exact master CI evidence

The R3 merge to `master` was verified on exact merged master SHA `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f`.

### Ordinary CI

Run `34737621353` completed successfully for the relevant jobs:

- `Classify CI scope`
- `CI plumbing`
- `Generated backend contract`
- `Release metadata static gate`
- `Rust quality`
- `Rust tests`

The path-scoped jobs that did not apply to this backend/source change were skipped as expected.

### Real KittenTTS CPU acceptance

Run `34737621401` completed successfully:

- `Verify exact acceptance head`
- `Real Kitten Mini CPU inference (linux-x86_64)`
- `Real Kitten Mini CPU inference (macos-arm64)`

The acceptance workflow verified the pinned production catalog artifacts, loaded the production Local TTS runtime, synthesized with real KittenTTS CPU inference, measured performance, measured thread policy, denied network during synthesis, proved invalid voice fail-closed behavior, and uploaded machine-readable evidence.

### KittenTTS to Moonshine Tiny ASR smoke

Run `34737621335` completed successfully:

- `Verify exact round-trip head`
- `KittenTTS → Moonshine Tiny (macos-arm64)`

The smoke test synthesized the fixed all-voice sentence with all eight Kitten voices, resampled Kitten 24 kHz PCM to Moonshine 16 kHz input, denied network during the round trip, decoded with Moonshine Tiny, and gated each voice on WER/content-word recall.

---

## Platform evidence summary

### Linux x86_64 real CPU acceptance

Representative R3 artifact evidence:

- Platform: `linux-x86_64`
- Model: `KittenML/kitten-tts-mini-0.8`
- Sample rate: `24000`
- Production inference threads: `2`
- Model load duration: `1035 ms`
- Cold end-to-end duration: `2559 ms`
- Max RSS: `554,299,392 bytes`
- Median warm RTF: `0.48089876752229976`
- p95 warm RTF: `0.5825509462626263`
- Network denied during synthesis: `true`
- Cancellation observed: `true`
- Voices exercised: `Bella`, `Jasper`, `Luna`, `Bruno`, `Rosie`, `Hugo`, `Kiki`, `Leo`

### macOS arm64 real CPU acceptance

Representative R3 artifact evidence:

- Platform: `macos-arm64`
- Model: `KittenML/kitten-tts-mini-0.8`
- Sample rate: `24000`
- Production inference threads: `2`
- Model load duration: `2057 ms`
- Cold end-to-end duration: `4193 ms`
- Max RSS: `805,502,976 bytes`
- Median warm RTF: `0.5902187257933371`
- p95 warm RTF: `0.6284197101449276`
- Network denied during synthesis: `true`
- Cancellation observed: `true`
- Voices exercised: `Bella`, `Jasper`, `Luna`, `Bruno`, `Rosie`, `Hugo`, `Kiki`, `Leo`

### macOS x86_64 boundary

macOS x86_64 is covered as a compile/package/provenance boundary in ordinary release validation. It is not recorded as a real CPU inference acceptance platform here. Do not check off any task that specifically requires real macOS x86_64 Kitten inference unless a future run actually executes that workflow on macOS x86_64.

---

## Thread policy evidence

The real acceptance workflow measured 1/2/4-thread CPU policy and kept production at `2` inference threads. This remains an implementation constant from the production manifest/runtime path, not a user-facing setting.

### Linux x86_64 thread sweep

| Threads | Median warm RTF | p95 warm RTF | Model load |
|---:|---:|---:|---:|
| 1 | 0.5122437421917808 | 0.5212966934246576 | 1039 ms |
| 2 | 0.4510806268493151 | 0.45295998575342467 | 356 ms |
| 4 | 0.5897675706849315 | 0.5981428216438356 | 348 ms |

### macOS arm64 thread sweep

| Threads | Median warm RTF | p95 warm RTF | Model load |
|---:|---:|---:|---:|
| 1 | 0.5697181518367348 | 0.7194521994557823 | 1720 ms |
| 2 | 0.5053111110204082 | 0.5353374604081633 | 693 ms |
| 4 | 0.6208612810884355 | 0.7643205102040815 | 531 ms |

The evidence supports `2` as the conservative production default. Four threads loaded slightly faster in some cases but produced worse warm steady-state RTF, especially on macOS arm64.

---

## ASR intelligibility smoke evidence

The automated ASR smoke passed for all eight Kitten voices:

- `Bella`
- `Jasper`
- `Luna`
- `Bruno`
- `Rosie`
- `Hugo`
- `Kiki`
- `Leo`

Four voices transcribed essentially exactly. The other voices had small substitutions such as `moose` being decoded as a near word, but they still cleared the configured WER/content-word recall gates.

This is automated intelligibility evidence only. It does not decide the default Local voice, naturalness, comedic fit, timbre, fatigue, or artifact acceptability.

---

## Deferred owner-only gate

`KCR-330` / `KTT-805` remains open. The owner must audition the real Local Kitten voices and choose the default Local TTS voice. Automated ASR smoke cannot close this gate.

---

## Expensive workflow rerun decision for closeout branch

The closeout branch fixes the stale direct `AppState::get_speech_synthesizer()` helper path, removes tracker/documentation drift, and adds source-level diagnostics composition coverage. It does not change the Local TTS runtime engine, native runtime loading, model manifests, ASR round-trip implementation, or real acceptance workflows.

Therefore the final closeout PR requires exact-head ordinary CI, Rust quality/tests, generated contract, static packaging/license gates, and final source audit. Re-running the expensive real-model KittenTTS and ASR workflows is not required unless later closeout commits touch runtime, synthesis engine, ASR, native artifact, or workflow code.
