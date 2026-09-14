# AI Talking Moose — KittenTTS Closeout Evidence

**Date:** 2026-09-12
**Scope:** evidence for the KittenTTS closeout remediation queue.
**Reviewed baseline:** `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f` (`master`).
**R3 final qualified PR head:** `5280b9246b4af78bc47020fb079120f1b935a188`.
**R3 merged master SHA:** `32e4fb1dd0cae8d62ca191cca5821ce53e8f7f3f`.
**Closeout PR #101 final head:** `7eb69d81a6709587769e57abd2a2cf567e9b31a4`.
**Closeout PR #101 merged master SHA:** `79423e8cc0ad48e796e964dee7eb51c3a440ddc4`.
**ASR-proxy voice audition PR #114 merged master SHA:** `21fac6785f008b1d5ab7f46e9125c76ca4c087ad`.
**Default voice closeout PR #115 merged master SHA:** `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4`.
**Selected V1 Local default:** `Luna`.

This document records what was actually tested and what remains deliberately caveated. It may be used to mark `KCR-330` / `KTT-805` complete for V1 only as an owner-approved ASR proxy decision, not as subjective human listening evidence.

---

## Exact R3 master CI evidence

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

This was initially automated intelligibility evidence only. PR #114 later added a dedicated ASR voice-audition proxy, and PR #115 used the owner-approved proxy recommendation to select `Luna` as the V1 Local default. This still does not prove subjective naturalness, comedic fit, timbre, fatigue, or artifact acceptability.

---

## Closeout PR #101 qualification evidence

PR #101 (`ralph/kcr-r4-r5-reconcile-closeout`) closed the finite post-review remediation queue. The final PR head was `7eb69d81a6709587769e57abd2a2cf567e9b31a4`.

Exact PR-head validation completed before merge:

- Ordinary CI run `34755503886`: success.
- Production KittenTTS CPU acceptance run `34755503936`: success.
- Production acceptance passed `Real Kitten Mini CPU inference (linux-x86_64)`.
- Production acceptance passed `Real Kitten Mini CPU inference (macos-arm64)`.
- Both real-model platform jobs passed hardened inference and measured the required 1/2/4-thread CPU policy.

A temporary Linux ORT diagnostic workflow was used during investigation of an intermittent 1-thread Linux failure and was removed before the final PR head. No temporary diagnostic workflow was present in the merged tree.

---

## Guarded merge evidence

PR #101 was mergeable immediately before merge and was merged only with expected head `7eb69d81a6709587769e57abd2a2cf567e9b31a4`.

Merge result:

- Merged master SHA: `79423e8cc0ad48e796e964dee7eb51c3a440ddc4`.
- Merge title: `Merge PR #101: KCR-400–503 KittenTTS closeout evidence`.
- Merge body recorded exact qualifying PR-head runs `34755503886` and `34755503936`.

---

## Exact merged-master verification

The merged master SHA `79423e8cc0ad48e796e964dee7eb51c3a440ddc4` was verified after merge.

### Ordinary CI

Run `34755879197` completed successfully on exact merged master.

Relevant successful jobs:

- `Classify CI scope`
- `Frontend quality`
- `Generated backend contract`
- `Rust quality`
- `Rust tests`

Path-scoped jobs that did not apply to this closeout merge were skipped by the repository classifier.

### Real KittenTTS CPU acceptance

Run `34755879210` completed successfully on exact merged master.

Successful jobs:

- `Verify exact acceptance head`
- `Real Kitten Mini CPU inference (linux-x86_64)`
- `Real Kitten Mini CPU inference (macos-arm64)`

Both real-model platform jobs passed hardened inference, 1/2/4-thread CPU-policy measurement, performance/privacy evidence publication, and machine-readable evidence upload.

### ASR rerun decision

The closeout branch did not change the ASR round-trip implementation or KittenTTS-to-Moonshine workflow. The existing exact R3 merged-master ASR smoke run `34737621335` remains the applicable all-eight-voice automated intelligibility evidence. No additional ASR rerun was required for PR #101.

---

## ASR-proxy default voice closeout evidence

PR #114 added `.github/workflows/kittentts-asr-voice-audition.yml`, which converts the all-eight-voice KittenTTS-to-Moonshine Tiny round-trip into a deterministic objective recommendation. The workflow ranks passing voices by lowest WER, highest content-word recall, and Local KittenTTS catalog order as the final tie-breaker.

Validated ASR voice-audition evidence:

| Context | SHA | Workflow run | Result |
| --- | --- | ---: | :---: |
| PR #114 head | `037a0f0a61e13c76ddc0f9a16ef88adb2f38f141` | `34821757481` | PASS |
| PR #114 post-merge master | `21fac6785f008b1d5ab7f46e9125c76ca4c087ad` | `34848251323` | PASS |

`Luna`, `Bruno`, `Hugo`, and `Leo` tied with perfect WER and content recall in the PR-head evidence. `Luna` won by the documented catalog-order tie-breaker.

PR #115 changed the shipped Local TTS default to `Luna` and synced the generated backend contract plus runtime test expectation. Exact PR-head validation passed on `4a8bf77e4803b7898c717a3cbe145f9df9600c75`:

- CI `34853091162` — PASS.
- KittenTTS production CPU acceptance `34853091150` — PASS.
- KittenTTS ASR intelligibility smoke `34853091234` — PASS.
- KittenTTS ASR voice audition `34853091230` — PASS.

PR #115 was guarded-merged as `96ff21e17b02aa0e3b5d8d783bd09400297eaaa4`. Exact post-merge validation passed:

- CI `34854193904` — PASS.
- KittenTTS production CPU acceptance `34854193873` — PASS.
- KittenTTS ASR intelligibility smoke `34854193869` — PASS.
- KittenTTS ASR voice audition `34854193968` — PASS.

This closes `KCR-330` / `KTT-805` for V1 by owner-approved ASR proxy evidence. It does not claim subjective human listening, best Moose comedic fit, timbre preference, fatigue tolerance, or real-speaker character fit.

---

## Final closeout decision

The technical KittenTTS closeout is complete on master at `79423e8cc0ad48e796e964dee7eb51c3a440ddc4`.

Closed technical areas:

- stale `AppState::get_speech_synthesizer()` Local placeholder routing;
- direct helper routing and no-fallback tests;
- command-level Local TTS diagnostics composition tests;
- README, voice-selection, and privacy documentation drift;
- legacy TODO reconciliation;
- exact PR-head CI and real-model acceptance;
- guarded merge;
- exact merged-master CI and real-model acceptance.

`KCR-330` / `KTT-805` is now closed for V1 by owner-approved ASR proxy evidence from PR #114 and PR #115. The selected V1 Local KittenTTS default is `Luna`. This does not claim subjective human listening; a future real-device subjective pass can still override `Luna` if the owner prefers another catalog voice.
