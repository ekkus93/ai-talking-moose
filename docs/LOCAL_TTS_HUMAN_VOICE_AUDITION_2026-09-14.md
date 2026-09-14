# AI Talking Moose — Local KittenTTS Human Voice Audition

**Date:** 2026-09-14
**Scope:** `KCR-330` / `KTT-805` human audition and final Local KittenTTS default-voice selection
**Status:** V1 default resolved by owner-approved ASR proxy; subjective listening not performed
**Related docs:**

- `docs/VOICE_SELECTION.md`
- `docs/LOCAL_TTS_AUTOMATED_ASR_VOICE_AUDITION_2026-09-14.md`
- `docs/KITTENTTS_CLOSEOUT_REMEDIATION_TODO_2026-09-12.md`
- `docs/KITTENTTS_LEGACY_TODO_RECONCILIATION_2026-09-12.md`
- `docs/LOCAL_TTS_POST_CLOSEOUT_HARDENING_TODO_2026-09-13.md`

This worksheet remains available for a future subjective listening pass. It was not used to select the V1 Local KittenTTS default.

The V1 default was instead selected by owner-approved automated ASR proxy evidence. Do not describe that proxy decision as subjective listening evidence.

---

## V1 ASR-proxy decision record

```text
Decision date: 2026-09-14
Basis: owner-approved ASR proxy, not human listening
Accepted Local KittenTTS default voice: Luna
PR-head ASR audition run: 34821757481
PR-head qualified SHA: 037a0f0a61e13c76ddc0f9a16ef88adb2f38f141
Post-merge ASR audition run: 34848251323
Post-merge qualified SHA: 21fac6785f008b1d5ab7f46e9125c76ca4c087ad
Reason: Luna tied with Bruno, Hugo, and Leo at WER 0.0000 and content recall 1.0000, then won by documented catalog-order tie-breaker.
Subjective human listening performed: no
Follow-up fixes required before default change: none known from ASR proxy evidence
Owner approval: accepted ASR proxy in project chat on 2026-09-14
```

---

## Fixed voice catalog

Audition exactly the eight Local KittenTTS V1 voices if a future subjective listening pass is performed:

- `Bella`
- `Jasper`
- `Luna`
- `Bruno`
- `Rosie`
- `Hugo`
- `Kiki`
- `Leo`

`Luna` is now the V1 Local TTS default by owner-approved ASR proxy evidence. A future subjective pass can still replace it if the owner records a different accepted voice.

---

## Setup checklist for future subjective listening

- [ ] Build or run an app version that includes the current Local TTS default decision.
- [ ] Open the Voice settings surface.
- [ ] Select standalone speech provider: Local KittenTTS.
- [ ] Install the Local KittenTTS model if it is not already installed.
- [ ] Confirm the UI reports the Local model as installed/verified before auditioning.
- [ ] Keep Gemini Live voice settings separate from Local standalone voice settings.
- [ ] Do not change `google_tts_voice` or `live_voice` during this audition unless intentionally testing settings separation.
- [ ] Use the same playback device and volume for every voice.
- [ ] Audition every voice against the same corpus before choosing a replacement default.

---

## Audition corpus

Use these lines for each voice. The goal is not only ASR intelligibility; it is whether the voice sounds like a good original Talking Moose voice over repeated use.

1. `Local speech synthesis is running on this machine.`
2. `I have reviewed the evidence and, regrettably, the computer is haunted.`
3. `That is a bold strategy for turning electricity into regret.`
4. `At 3:45 PM on September fourteenth, the Moose bought twelve bolts for nineteen dollars and ninety-five cents.`
5. `Dr. Nguyen checked OAuth logs, SQL rows, USB-C cables, and a very suspicious sandwich.`
6. `Piper, Kitten, Gemini, Moonshine, and ONNX Runtime all walked into a task queue.`
7. `I can explain it again, but the second explanation will contain more antlers.`
8. `The correct answer is probably somewhere between a race condition and a raccoon with administrator privileges.`
9. `Please do not interpret this calm voice as approval of your build system.`
10. `A talking moose should sound dry, readable, slightly ridiculous, and not like it is imitating any existing performer.`

Optional fatigue pass after narrowing finalists:

- Read the top two or three voices through all ten lines twice.
- Reject a voice if it becomes irritating, harsh, muddy, or tiring after repeated playback.

---

## Scoring rubric

Score each dimension from 1 to 5.

| Score | Meaning |
| --- | --- |
| 1 | Unacceptable; clearly wrong for the Moose. |
| 2 | Weak; usable only as a fallback. |
| 3 | Adequate; understandable but not an obvious default. |
| 4 | Strong; good candidate for default. |
| 5 | Excellent; preferred default candidate. |

Scoring dimensions:

| Dimension | What to listen for |
| --- | --- |
| Intelligibility | Words are clear without needing subtitles or repeated playback. |
| Naturalness | The voice does not sound broken, robotic in a bad way, or unstable. |
| Dry/comedic fit | The line delivery works for a dry, sarcastic, mildly absurd Moose. |
| Timbre | The voice has a tolerable tone for frequent use. |
| Pronunciation | Proper names, acronyms, dates, numbers, and technical terms are acceptable. |
| Artifacts/noise | No distracting buzzing, clicking, dropouts, clipping, or breathy artifacts. |
| Fatigue | The voice remains tolerable after repeated lines. |
| Originality | The voice does not read as an attempt to imitate Bullwinkle, Bill Scott, or another identifiable performer. |

---

## Score sheet for future subjective listening

| Voice | Intelligibility | Naturalness | Dry/comedic fit | Timbre | Pronunciation | Artifacts/noise | Fatigue | Originality | Total | Notes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Bella |  |  |  |  |  |  |  |  |  |  |
| Jasper |  |  |  |  |  |  |  |  |  |  |
| Luna |  |  |  |  |  |  |  |  |  |  |
| Bruno |  |  |  |  |  |  |  |  |  |  |
| Rosie |  |  |  |  |  |  |  |  |  |  |
| Hugo |  |  |  |  |  |  |  |  |  |  |
| Kiki |  |  |  |  |  |  |  |  |  |  |
| Leo |  |  |  |  |  |  |  |  |  |  |

Recommended acceptance threshold:

- Default candidate should score at least 4 in intelligibility.
- Default candidate should score at least 4 in dry/comedic fit.
- Default candidate should not score below 3 in artifacts/noise or fatigue.
- A lower total can still win if the owner explicitly prefers it after listening.

---

## Future subjective override record

Fill this section only if the owner later performs subjective listening and wants to override the ASR-proxy default.

```text
Audition date:
App/build SHA:
Playback device:
Selected Local KittenTTS default voice:
Decision: accept / reject / keep Luna
Reason:
Rejected finalists:
Follow-up fixes required before default change:
Owner initials/approval:
```

Do not update the Rust default from a future worksheet unless the decision is explicit and the selected voice is one of the eight catalog IDs.

---

## Follow-up implementation if a future voice override is accepted

After the owner records a new selected voice:

- [ ] Update `DEFAULT_LOCAL_TTS_VOICE` if the accepted voice is not already the default.
- [ ] Update `docs/VOICE_SELECTION.md` to record the final Local default and audition decision.
- [ ] Update closeout/reconciliation docs only as evidence, without reopening technical work.
- [ ] Run ordinary CI.
- [ ] Run Local TTS production CPU acceptance if the default change affects real Local TTS output exercised by acceptance.
- [ ] Run ASR smoke if the accepted default change could alter documented all-voice/default-voice evidence.
- [ ] Merge with expected-head guard and verify exact master.

If the owner rejects all voices in a future subjective pass, keep `Luna` as the ASR-proxy default and create a new task for alternate Local TTS voice/model exploration rather than pretending another catalog voice was accepted.
