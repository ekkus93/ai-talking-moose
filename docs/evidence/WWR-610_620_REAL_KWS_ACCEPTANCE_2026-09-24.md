# WWR-610/620 — real KWS acceptance evidence

Date: 2026-09-24
Accepted master: `e5ff47a3dd08377298783969aa0dacbc8583dcfb`
Corpus reconciliation master: `da3be8db9f6b1f4b26622126708a439e869fc8b9`

## Scope

This evidence records the exact-master real KWS acceptance result after the deterministic corpus contained six positive recipes and eight negative/near-miss recipes. It supports WWR-610 and WWR-620 real native KWS acceptance. It also supports the WWR-600 harness/corpus acceptance criteria that were actually exercised by the workflow. It does not claim WWR-630 performance acceptance, WWR-640 integrated lifecycle soak acceptance, WWR-950 final qualification, or WWR-960 exact-master closeout.

## Exact merged-master validation

All acceptance runs below are bound to master commit `e5ff47a3dd08377298783969aa0dacbc8583dcfb`:

- Ordinary CI: run `35992665763`, success.
- Wake Word corpus contract: run `35992665769`, success.
- Wake Word corpus validation: run `35992665914`, success.
- Wake Word privacy audit: run `35992665883`, success.
- Wake Word real KWS acceptance: run `35992665781`, success.

`da3be8db9f6b1f4b26622126708a439e869fc8b9` subsequently merged documentation-only WWR-600 acceptance evidence (#431). Ordinary CI run `35993867191` passed on that documentation-only master.

## Real KWS workflow jobs

`Wake Word real KWS acceptance` run `35992665781` completed successfully with these jobs:

- `Generate deterministic Wake corpus`: success.
- `Real KWS acceptance (linux-x86_64)`: success.
- `Real KWS acceptance (macos-arm64)`: success.

The run uploaded these artifacts:

- `wake-word-v1-corpus` — deterministic generated corpus artifact, 532756 bytes.
- `wake-word-real-kws-linux-x86_64` — Linux x86_64 real KWS platform report artifact, 1694 bytes.
- `wake-word-real-kws-macos-arm64` — macOS arm64 real KWS platform report artifact, 1703 bytes.

## Corpus and harness properties verified

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

The accepted corpus includes six positive recipes and eight negative/near-miss recipes. Positive fixtures include the plain wake phrase, wake phrase plus command, varied reproducible synthetic speaker/source settings, gain/distance variation, and deterministic background-noise variants. Negative fixtures include ordinary speech without the wake phrase, `Moose` alone, `Hey Bruce`, `Hey Moosey`, phonetically similar near-miss speech, a sentence containing `moose` without the complete wake phrase, and deterministic media/background-style ordinary speech generated from repository-authored fixture text.

## WWR-610 result

Linux x86_64 support is backed by real pinned sherpa KWS inference at exact master `e5ff47a3dd08377298783969aa0dacbc8583dcfb` in workflow run `35992665781`, job `Real KWS acceptance (linux-x86_64)`. The platform job prepared the deterministic corpus, used the exact frozen model/runtime identities, enforced the V1 one-thread score/threshold policy, executed positive and negative fixtures offline, and produced a privacy-safe platform report artifact.

## WWR-620 result

macOS arm64 support is backed by real pinned sherpa KWS inference at exact master `e5ff47a3dd08377298783969aa0dacbc8583dcfb` in workflow run `35992665781`, job `Real KWS acceptance (macos-arm64)`. The platform job prepared the deterministic corpus, used the exact frozen model/runtime identities, enforced the V1 one-thread score/threshold policy, executed positive and negative fixtures offline, and produced a privacy-safe platform report artifact.

## Boundaries and remaining work

This evidence does not close WWR-630 or WWR-640. The real KWS acceptance run proves platform KWS inference and corpus pass/fail behavior; it is not an integrated wake→ASR→Thinking→Talking→wake lifecycle soak and does not provide the full CPU/memory/latency/KWS-versus-full-ASR performance baseline.

This evidence also does not claim final WWR-950/960 closeout. Final closeout still requires the complete required-gates set, TODO reconciliation, and exact final merged-master evidence for all remaining acceptance categories.
