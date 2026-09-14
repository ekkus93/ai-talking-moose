# AI Talking Moose — Local TTS ASR-Proxy Default Voice Closeout

**Date:** 2026-09-14
**Scope:** Close `KCR-330` / `KTT-805` for V1 by owner-approved ASR proxy
**Selected Local KittenTTS default:** `Luna`
**Decision type:** Owner-approved automated ASR proxy, not subjective human listening

---

## Decision

The V1 Local KittenTTS default voice is `Luna`.

The owner accepted the automated ASR proxy as sufficient for this project phase. This closes `KCR-330` / `KTT-805` for V1 without claiming subjective human listening evidence.

A future real-device subjective audition can still replace `Luna`, but that is a new override decision rather than a blocker for the current V1 closeout.

---

## Evidence basis

The automated audition workflow synthesizes every Local KittenTTS V1 voice, transcribes the generated audio with pinned Moonshine Tiny, and ranks passing voices by objective transcript quality.

Validated evidence:

| Context | SHA | Workflow run | Result |
| --- | --- | ---: | :---: |
| PR #114 head | `037a0f0a61e13c76ddc0f9a16ef88adb2f38f141` | `34821757481` | PASS |
| Post-merge `master` | `21fac6785f008b1d5ab7f46e9125c76ca4c087ad` | `34848251323` | PASS |

Per-voice PR-head result:

| Voice | WER | Content recall | Result |
| --- | ---: | ---: | :---: |
| Bella | 0.0909 | 0.8571 | PASS |
| Jasper | 0.1818 | 0.8571 | PASS |
| Luna | 0.0000 | 1.0000 | PASS |
| Bruno | 0.0000 | 1.0000 | PASS |
| Rosie | 0.0909 | 0.8571 | PASS |
| Hugo | 0.0000 | 1.0000 | PASS |
| Kiki | 0.0909 | 0.8571 | PASS |
| Leo | 0.0000 | 1.0000 | PASS |

`Luna`, `Bruno`, `Hugo`, and `Leo` tied with perfect WER and content recall. The workflow selected `Luna` by the documented catalog-order tie-breaker.

---

## Truthfulness boundary

This closeout proves and records an objective ASR proxy decision. It does not prove:

- subjective naturalness;
- comedic timing;
- timbre preference;
- long-listening fatigue;
- best real-speaker or real-device character fit.

Docs and release notes must therefore describe the decision as **owner-approved ASR proxy** evidence, not as a completed human listening audition.

---

## Implementation requirements

The default-change implementation must:

- [x] update `DEFAULT_LOCAL_TTS_VOICE` from `Bella` to `Luna`;
- [x] keep `Luna` inside `LOCAL_TTS_VOICE_IDS`;
- [x] preserve Google standalone TTS voice ownership;
- [x] preserve Gemini Live voice ownership;
- [x] preserve Local fail-closed unsupported-voice behavior;
- [x] preserve no Local-to-Google fallback;
- [x] update voice-selection documentation;
- [x] update ASR-proxy/human-audition documentation without claiming subjective listening;
- [ ] pass exact PR-head ordinary CI;
- [ ] pass exact PR-head Local TTS production CPU acceptance if triggered or required;
- [ ] pass exact PR-head ASR/voice-audition validation if triggered or required;
- [ ] merge with exact-head guard;
- [ ] verify exact merged `master`.
