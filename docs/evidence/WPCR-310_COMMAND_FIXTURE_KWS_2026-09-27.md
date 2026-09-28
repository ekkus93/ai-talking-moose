# WPCR-310 Command Fixture Native-KWS Evidence

**Date:** 2026-09-27
**Remediation item:** WPCR-310 — downstream first-command-word acceptance
**Exact master:** `138384250a7133465e8b890701de4cbf3f93eb34`

## Scope

This evidence records the generated-spoken-fixture portion of WPCR-310. It complements `docs/evidence/WPCR-310_FIRST_COMMAND_WORD_BOUNDARY_2026-09-26.md`, which proves deterministic downstream handoff continuity. This evidence proves that the generated `Hey Moose. Tell me the time` fixture is part of the deterministic corpus and is required to trigger native sherpa KWS detection on both accepted native platforms.

## Implemented gate hardening

`.github/workflows/wake-word-real-kws.yml` now includes a `Require command fixture detection` step after the real pinned sherpa KWS report is generated for each platform. The step reads the platform report and fails unless fixture `positive-command-sc`:

- is present in the real-KWS report;
- is declared as an expected positive fixture;
- reports `detected: true`.

The fixture is defined in `docs/wake-word-corpus.json` as repository-authored deterministic synthetic speech text `Hey Moose. Tell me the time`, generated ephemerally by `scripts/generate_wake_word_corpus.py`.

## Exact validation

All runs below are bound to exact master `138384250a7133465e8b890701de4cbf3f93eb34`:

- Ordinary CI passed: run `36377539641`.
- Wake Word required-gates audit passed: run `36377539545`.
- Wake Word real KWS acceptance passed: run `36377539575`.

The real-KWS run completed these jobs successfully:

- `Generate deterministic Wake corpus`;
- `Real KWS acceptance (linux-x86_64)`;
- `Real KWS acceptance (macos-arm64)`.

The run uploaded these artifacts:

- `wake-word-v1-corpus` — artifact `10951486579`, 532756 bytes;
- `wake-word-real-kws-linux-x86_64` — artifact `10950959897`, 2890 bytes;
- `wake-word-real-kws-macos-arm64` — artifact `10951248452`, 2891 bytes.

Because the new workflow step is inside each platform job and the workflow passed, both Linux x86_64 and macOS arm64 proved real native-KWS detection of the generated command fixture.

## Boundaries

This evidence proves native KWS detection of the generated command fixture and deterministic downstream boundary receipt when combined with the earlier WPCR-310 boundary evidence. It does not claim real Moonshine transcription correctness for `tell me the time` and does not replace WPCR-950/960 final exact-head/exact-master closeout.

## Privacy

The fixture text is repository-authored and approved for acceptance. Generated PCM remains ephemeral CI artifact content; no user microphone audio, private transcript, credential, or private path is committed.
