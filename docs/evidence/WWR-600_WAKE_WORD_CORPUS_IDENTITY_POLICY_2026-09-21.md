# WWR-600 — Wake Word Corpus Identity Policy Evidence

**Date:** 2026-09-21
**Scope:** Record KWS policy and exact artifact/runtime identity in the deterministic Wake Word corpus manifest.

## Change

`docs/wake-word-corpus.json` now records:

- KWS score `1.0`;
- KWS threshold `0.25`;
- source artifact manifest path;
- exact model ID;
- model archive SHA-256;
- keyword representation SHA-256;
- sherpa runtime ID/version;
- Linux x86_64 C API runtime SHA-256;
- macOS arm64 C API runtime SHA-256.

Both corpus validators now compare those values against `wake-word-artifacts.json`, so the acceptance harness metadata cannot silently drift from the production artifact manifest. Python regression tests cover score/threshold drift and model/runtime identity drift.

## TODO reconciliation note

This satisfies the WWR-600 harness requirement to record exact model/runtime identity and score/threshold policy in the deterministic corpus contract. Positive/negative fixture collection, measured detections, false rejects, false accepts, calibrated pass/fail criteria, and real platform KWS acceptance remain open.
