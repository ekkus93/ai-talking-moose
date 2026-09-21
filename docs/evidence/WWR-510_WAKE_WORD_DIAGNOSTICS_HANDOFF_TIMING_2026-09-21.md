# WWR-510 — Wake Word Diagnostics Handoff Timing Evidence

**Date:** 2026-09-21
**Scope:** Add an available privacy-safe handoff timing field to Wake Word V1 diagnostics.

## Change

`WakeWordDiagnostics` now reports `handoff_pre_roll_duration_ms`, derived only from the already-exposed `handoff_pre_roll_samples` count and the fixed V1 `16_000 Hz` policy.

This is intentionally metadata-only. The field does not expose raw PCM, transcripts, file paths, credentials, model paths, runtime paths, or audio content.

## Regression coverage

The diagnostics regression now verifies:

- disabled diagnostics report zero handoff duration;
- a 1,600-sample triggered pre-roll reports `100 ms`;
- serialized diagnostics do not contain raw audio field names;
- serialized diagnostics do not contain the repeated synthetic sample value used by the test;
- Talking suspension clears both handoff sample count and handoff duration.

## TODO reconciliation note

This satisfies the handoff-timing portion of WWR-510's optional measured CPU/memory/inference/handoff timing item. CPU, memory, and real inference timing remain open until representative acceptance environments are available and measured.
