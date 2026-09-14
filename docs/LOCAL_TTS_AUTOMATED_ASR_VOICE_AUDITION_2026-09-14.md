# AI Talking Moose — Local KittenTTS Automated ASR Voice Audition

**Date:** 2026-09-14
**Scope:** Objective ASR proxy for `KCR-330` / `KTT-805`
**Status:** Implemented as an automated evidence gate; not a subjective listening gate
**Workflow:** `.github/workflows/kittentts-asr-voice-audition.yml`
**Underlying Rust test:** `asr::pipeline::benchmarks::kittentts_all_voices_round_trip_through_moonshine_tiny`

This document defines the automated ASR-based audition requested as an alternative to purely manual listening.

The workflow synthesizes every Local KittenTTS V1 voice, transcribes the generated audio with pinned Moonshine Tiny, computes objective transcript-quality metrics, and emits machine-readable evidence with a recommended objective candidate.

This is useful for catching unintelligible or badly drifting voices. It is not a complete substitute for human perception of naturalness, comedic timing, timbre preference, or fatigue.

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

The evidence artifact records the full per-voice data so a later default change can cite the exact run rather than relying on a summary sentence.

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

There are now two valid ways to close the Local default-voice decision:

1. **Human subjective closeout:** the owner listens to the worksheet corpus and records the selected voice.
2. **Owner-approved ASR proxy closeout:** the owner explicitly accepts the automated ASR recommendation as sufficient for this project phase.

If path 2 is used, the closeout record must say that the default was selected by owner-approved ASR proxy evidence, not by subjective human audition.

Do not silently rewrite “human audition” as complete merely because ASR passed.

---

## Follow-up default update

After a validated ASR audition run exists, update `DEFAULT_LOCAL_TTS_VOICE` only if the owner explicitly accepts the ASR-selected candidate or provides a different selected voice.

The follow-up implementation should:

- record the exact ASR audition workflow run ID;
- record the qualified SHA;
- record the recommended objective voice;
- update `DEFAULT_LOCAL_TTS_VOICE` if the accepted voice is not already the default;
- update `docs/VOICE_SELECTION.md` with the selection basis;
- update the closeout/reconciliation docs without claiming subjective listening if it was not done;
- run ordinary CI and any Local TTS/ASR workflow required by the diff scope;
- merge with an exact-head guard and verify exact `master`.
