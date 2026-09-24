# WWR-600 — deterministic corpus and harness acceptance

Date: 2026-09-24
Merged master: `e5ff47a3dd08377298783969aa0dacbc8583dcfb`

## Scope

This evidence records the completed deterministic Wake Word V1 corpus/harness acceptance after adding the remaining visible near-miss and media/background-style negative fixtures. It is evidence for WWR-600 and for the corpus portions of WWR-610/620 real KWS acceptance. It does not claim WWR-630 performance acceptance, WWR-640 integrated lifecycle soak acceptance, or final WWR-950/960 closeout.

## Corpus manifest

Authoritative manifest: `docs/wake-word-corpus.json`

The manifest at `e5ff47a3dd08377298783969aa0dacbc8583dcfb` is schema version 2 and declares:

- wake phrase: `Hey, Moose`;
- sample rate: `16000` Hz;
- channel count: `1`;
- sample format: `pcm_s16le`;
- score: `1.0`;
- threshold: `0.25`;
- generator: `scripts/generate_wake_word_corpus.py`;
- generation tool: `espeak-ng`;
- generated audio is ephemeral and must not be committed;
- source manifest: `wake-word-artifacts.json`;
- criteria version: `2`;
- positive recall minimum: `0.67`;
- negative false accepts maximum: `0`.

## Positive fixture coverage

The manifest contains six positive recipes:

- plain `Hey Moose` wake phrase fixtures;
- varied reproducible synthetic speakers/voices (`en-us`, `en-sc`);
- varied gain/distance/noise settings;
- wake phrase immediately followed by commands (`Tell me the time`, `Tell me a joke`);
- deterministic generated audio with repository-authored fixture text and ephemeral PCM artifacts.

## Negative / near-miss fixture coverage

The manifest contains eight negative recipes after PR #430:

- ordinary speech without the wake phrase;
- `Moose` alone;
- `Hey Bruce`;
- `Hey Moosey`;
- phonetically similar `Hey Goose`;
- phonetically similar `Hey moves`;
- a sentence containing `moose` without the full wake phrase;
- deterministic media/background-style ordinary speech generated from repository-authored fixture text.

All negative recipes declare `expected_detection: false` and all generated PCM remains artifact-only/ephemeral.

## Exact validation on merged master

All runs below are bound to exact master `e5ff47a3dd08377298783969aa0dacbc8583dcfb`:

- Ordinary CI: run `35992665763`, success.
- Wake Word corpus contract: run `35992665769`, success.
- Wake Word corpus validation: run `35992665914`, success.
- Wake Word privacy audit: run `35992665883`, success.
- Wake Word real KWS acceptance: run `35992665781`, success.

The real KWS acceptance run completed these jobs successfully:

- `Generate deterministic Wake corpus`;
- `Real KWS acceptance (linux-x86_64)`;
- `Real KWS acceptance (macos-arm64)`.

The run uploaded these artifacts:

- `wake-word-v1-corpus` — 532756 bytes;
- `wake-word-real-kws-linux-x86_64` — 1694 bytes;
- `wake-word-real-kws-macos-arm64` — 1703 bytes.

## Acceptance conclusion

WWR-600's deterministic corpus and harness are accepted for the tested V1 conditions on exact master `e5ff47a3dd08377298783969aa0dacbc8583dcfb`: the harness is deterministic and CI/report-friendly, records model/runtime/policy identity through the manifest and reports, measures recall and false-trigger behavior rather than a single happy path, keeps generated PCM out of the repository, and limits claims to the predeclared generated fixture conditions.

Final product closeout still needs the remaining non-WWR-600 categories: performance measurement, integrated lifecycle/resource acceptance, final docs/audit reconciliation, and exact final closeout evidence.
