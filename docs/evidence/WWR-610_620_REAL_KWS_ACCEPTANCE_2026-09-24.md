# WWR-610/620 — real KWS acceptance evidence

Date: 2026-09-24
Merged master: `f95abbef2f458688666d0bb1fb479c14fb90d6de`
Source PR: #395 (`feat(wake): add real KWS acceptance harness`)

## Scope

This evidence records the exact merged-master real KWS acceptance result for the reusable Wake Word V1 acceptance harness. It supports WWR-610 and WWR-620 real native KWS acceptance. It also supports the WWR-600 harness/corpus acceptance criteria that were actually exercised by the workflow, but it does not claim that every named WWR-600 corpus-expansion subitem is complete unless that fixture class is present in `docs/wake-word-corpus.json`.

## Exact merged-master validation

All runs below are bound to master commit `f95abbef2f458688666d0bb1fb479c14fb90d6de`.

- Ordinary CI: run `35989146305`, success.
- Wake Word real KWS acceptance: run `35989146296`, success.
- Wake Word corpus validation: run `35989146276`, success.
- Wake Word corpus contract: run `35989146272`, success.
- Wake Word required gates audit: run `35989146229`, success.
- Wake Word privacy audit: run `35989146323`, success.
- Wake Word source security audit: run `35989146245`, success.

## Real KWS workflow jobs

`Wake Word real KWS acceptance` run `35989146296` completed successfully with these jobs:

- `Generate deterministic Wake corpus`: success.
- `Real KWS acceptance (linux-x86_64)`: success.
- `Real KWS acceptance (macos-arm64)`: success.

The run uploaded these artifacts:

- `wake-word-v1-corpus` — deterministic generated corpus artifact, 382173 bytes.
- `wake-word-real-kws-linux-x86_64` — Linux x86_64 real KWS platform report artifact, 1525 bytes.
- `wake-word-real-kws-macos-arm64` — macOS arm64 real KWS platform report artifact, 1552 bytes.

## Corpus and harness properties verified on master

The merged corpus manifest `docs/wake-word-corpus.json` is schema version 2 and declares:

- wake phrase: `Hey, Moose`;
- sample rate: 16000 Hz;
- channel count: 1;
- sample format: `pcm_s16le`;
- score: `1.0`;
- threshold: `0.25`;
- generator: `scripts/generate_wake_word_corpus.py`;
- generation tool: `espeak-ng`;
- generated audio is ephemeral and must not be committed;
- model/runtime identity source: `wake-word-artifacts.json`;
- active predeclared criteria version 2;
- positive recall minimum: `0.67`;
- negative false-accept maximum: `0`.

The corpus includes six positive recipes and six negative/near-miss recipes. Positive fixtures include plain wake phrase, wake phrase plus command, varied synthetic speaker/source settings, gain/distance variation, and low-amplitude deterministic noise variants. Negative fixtures include ordinary speech without the wake phrase, `Moose` alone, `Hey Bruce`, phonetically similar near-miss speech, and a sentence containing `moose` without the complete wake phrase.

## WWR-610 result

Linux x86_64 support is backed by real pinned sherpa KWS inference at exact master `f95abbef2f458688666d0bb1fb479c14fb90d6de` in workflow run `35989146296`, job `Real KWS acceptance (linux-x86_64)`. The platform job prepared the deterministic corpus, used the exact frozen model/runtime identities, enforced the V1 one-thread score/threshold policy, executed positive and negative fixtures offline, and produced a privacy-safe platform report artifact.

## WWR-620 result

macOS arm64 support is backed by real pinned sherpa KWS inference at exact master `f95abbef2f458688666d0bb1fb479c14fb90d6de` in workflow run `35989146296`, job `Real KWS acceptance (macos-arm64)`. The platform job prepared the deterministic corpus, used the exact frozen model/runtime identities, enforced the V1 one-thread score/threshold policy, executed positive and negative fixtures offline, and produced a privacy-safe platform report artifact.

## Boundaries and remaining work

This evidence does not close WWR-630 or WWR-640. The real KWS acceptance run proves platform KWS inference and corpus pass/fail behavior; it is not an integrated wake→ASR→Thinking→Talking→wake lifecycle soak and does not provide the full CPU/memory/latency/KWS-versus-full-ASR performance baseline.

This evidence also does not claim final WWR-950/960 closeout. Final closeout still requires the complete required-gates set, TODO reconciliation, and exact final merged-master evidence for all remaining acceptance categories.
