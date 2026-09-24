# WWR-600/610/620 — deterministic corpus and real KWS acceptance

Date: 2026-09-24
Merged master: `e5ff47a3dd08377298783969aa0dacbc8583dcfb`

## Exact-master evidence

- Ordinary CI: run `35992665763`, success.
- Wake Word corpus contract: run `35992665769`, success.
- Wake Word corpus validation: run `35992665914`, success.
- Wake Word privacy audit: run `35992665883`, success.
- Wake Word real KWS acceptance: run `35992665781`, success.

Real KWS acceptance run `35992665781` included successful jobs for corpus generation, `linux-x86_64`, and `macos-arm64`, and uploaded these artifacts:

- `wake-word-v1-corpus` (`artifact_id=10804224693`, size `532756` bytes)
- `wake-word-real-kws-linux-x86_64` (`artifact_id=10804244911`, size `1694` bytes)
- `wake-word-real-kws-macos-arm64` (`artifact_id=10804659037`, size `1703` bytes)

## Corpus and harness

`docs/wake-word-corpus.json` is schema version 2 and records the fixed wake phrase, deterministic generator, model/runtime identity, score `1.0`, threshold `0.25`, positive recall minimum `0.67`, and maximum negative false accepts `0`.

The deterministic recipes cover positive wake phrase variants, wake phrase plus command variants, varied gain/distance/noise variants, ordinary speech negatives, `Moose` alone, `Hey Bruce`, explicit `Hey Moosey`, phonetically similar phrases, sentences containing `moose` without the full phrase, and a synthetic media/background-style negative. Generated PCM remains a CI artifact and is not committed as repository audio.

## Linux x86_64

Run `35992665781` proves real Linux x86_64 KWS acceptance on exact merged master `e5ff47a3dd08377298783969aa0dacbc8583dcfb`. The Linux job prepared the exact pinned model/runtime, verified hashes and runtime architecture before inference, used the CPU-only one-thread policy, exercised real positive and negative inference, and uploaded privacy-safe evidence as `wake-word-real-kws-linux-x86_64`.

## macOS arm64

Run `35992665781` proves real macOS arm64 KWS acceptance on exact merged master `e5ff47a3dd08377298783969aa0dacbc8583dcfb`. The macOS job prepared the exact pinned model/runtime, verified hashes and runtime architecture before inference, used the CPU-only one-thread policy, exercised real positive and negative inference, and uploaded privacy-safe evidence as `wake-word-real-kws-macos-arm64`.

## Boundaries

This evidence supports WWR-600/610/620 reconciliation. It does not close WWR-630 performance evidence, WWR-640 integrated lifecycle acceptance, WWR-950 final qualification, or WWR-960 final closeout.
