# AI Talking Moose — Local KittenTTS Automated ASR Voice Audition

**Date:** 2026-09-14
**Scope:** Objective ASR proxy for `KCR-330` / `KTT-805`
**Status:** Complete for V1 by owner-approved ASR proxy; not a subjective listening gate
**Workflow:** `.github/workflows/kittentts-asr-voice-audition.yml`
**Underlying Rust test:** `asr::pipeline::benchmarks::kittentts_all_voices_round_trip_through_moonshine_tiny`
**Accepted V1 Local default:** `Luna`

This document defines the automated ASR-based audition requested as an alternative to purely manual listening.

The workflow synthesizes every Local KittenTTS V1 voice, transcribes the generated audio with pinned Moonshine Tiny, computes objective transcript-quality metrics, and emits machine-readable evidence with a recommended objective candidate.

This is useful for catching unintelligible or badly drifting voices. It is not a complete substitute for human perception of naturalness, comedic timing, timbre preference, or fatigue.

---

## Final ASR-proxy decision

The owner accepted the automated ASR proxy as sufficient for the V1 Local default-voice decision on 2026-09-14.

The ASR-proxy recommendation selected `Luna`.

Evidence used for the decision:

- PR #114 head: `037a0f0a61e13c76ddc0f9a16ef88adb2f38f141`.
- PR-head ASR audition workflow: `34821757481` — PASS.
- Post-merge `master`: `21fac6785f008b1d5ab7f46e9125c76ca4c087ad`.
- Post-merge ASR audition workflow: `34848251323` — PASS.

Per-voice PR-head ASR result:

| Voice | WER | Content recall | Result | Transcript summary |
| --- | ---: | ---: | :---: | --- |
| Bella | 0.0909 | 0.8571 | PASS | Moose recognized as “loose”. |
| Jasper | 0.1818 | 0.8571 | PASS | Moose recognized as “moves”; one verb inflection drift. |
| Luna | 0.0000 | 1.0000 | PASS | Exact transcript. |
| Bruno | 0.0000 | 1.0000 | PASS | Exact transcript. |
| Rosie | 0.0909 | 0.8571 | PASS | Moose recognized as “news”. |
| Hugo | 0.0000 | 1.0000 | PASS | Exact transcript with capitalization differences only. |
| Kiki | 0.0909 | 0.8571 | PASS | Moose recognized as “mousse”. |
| Leo | 0.0000 | 1.0000 | PASS | Exact transcript. |

`Luna`, `Bruno`, `Hugo`, and `Leo` tied with perfect ASR scores. `Luna` won by the documented Local KittenTTS catalog-order tie-breaker.

---

## What the automated audition measures

The ASR audition measures:

- every catalog voice can synthesize non-empty Local KittenTTS audio;
- the generated audio can be resampled into the existing Moonshine Tiny ASR path;
- transcription is non-empty;
- word error rate stays below the underlying ASR smoke limit;
- selected content words are recalled;
- the test runs under the repository network-denial guard after artifacts are installed;
- the workflow publishes per-voice objective metrics and a deterministic recommended candidate.

The selected candidate is objective only. It means “best by ASR transcript quality under this smoke corpus,” not “best sounding Moose voice.”

---

## Corpus

The automated proxy currently uses the existing all-eight-voice round-trip phrase:

```text
The talking moose reads seven blue books beside the quiet river.
```

The content-recall terms are:

```text
talking, moose, seven, blue, books, quiet, river
```

This keeps the workflow bounded and reuses the already-qualified ASR round-trip harness. A later expansion can add a longer multi-line audition corpus if the runtime cost is acceptable.

---

## Objective thresholds

The underlying Rust smoke test requires each voice to pass:

- word error rate `<= 0.40`;
- content-word recall `>= 0.70`;
- non-empty transcript;
- all eight Local KittenTTS voices exercised;
- network denied during the round-trip phase.

The ASR voice-audition workflow then ranks only passing voices.

---

## Ranking policy

Among passing voices, the recommended objective candidate is selected by:

1. lowest word error rate;
2. highest content-word recall;
3. Local KittenTTS catalog order as the deterministic tie-breaker.

The evidence artifact records the full per-voice data so a default change can cite the exact run rather than relying on a summary sentence.

---

## Evidence emitted

The workflow parses the existing Rust evidence line:

```text
KITTENTTS_ASR_ROUNDTRIP_JSON=
```

It writes:

```text
kittentts-asr-voice-audition.json
```

and uploads that artifact with the raw ASR audition log.

The JSON includes:

- exact qualified SHA;
- recommended objective voice;
- ranking basis;
- all original per-voice WER/recall/transcript evidence;
- explicit limitations.

The GitHub step summary includes a compact table of:

- voice;
- WER;
- content recall;
- pass/fail result;
- transcript;
- recommended objective candidate.

---

## How this affects `KCR-330` / `KTT-805`

`KCR-330` / `KTT-805` is closed for V1 by owner-approved ASR proxy evidence. This is not a claim of subjective human listening.

The truthful closeout statement is:

```text
The V1 Local KittenTTS default voice is Luna. Luna was selected by owner-approved ASR proxy evidence from the all-eight-voice KittenTTS-to-Moonshine Tiny audition. No subjective human listening claim is made.
```

A future subjective listening pass can still override the default if the owner prefers another catalog voice after real-device audition.

---

## Follow-up default update

The accepted follow-up implementation updates `DEFAULT_LOCAL_TTS_VOICE` from `Bella` to `Luna`, updates `docs/VOICE_SELECTION.md`, and records the ASR-proxy basis without claiming subjective listening.

The follow-up implementation should:

- [x] record the exact ASR audition workflow run ID;
- [x] record the qualified SHA;
- [x] record the recommended objective voice;
- [x] update `DEFAULT_LOCAL_TTS_VOICE` if the accepted voice is not already the default;
- [x] update `docs/VOICE_SELECTION.md` with the selection basis;
- [x] update closeout/reconciliation evidence without claiming subjective listening;
- [ ] run ordinary CI on the default-change PR;
- [ ] run Local TTS production CPU acceptance if required by the final diff scope;
- [ ] run ASR smoke/voice-audition validation if required by the final diff scope;
- [ ] merge with an exact-head guard and verify exact `master`.
