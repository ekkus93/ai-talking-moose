# WWR-700 — Wake Word Architecture Documentation Audit Evidence

**Date:** 2026-09-21
**Scope:** Make architecture documentation truthfulness enforceable in CI.

## Change

`node scripts/check_wake_word_documentation.mjs` now audits `docs/WAKE_WORD_V1_ARCHITECTURE.md` in addition to current behavior, UI disclosures, CI-gate boundaries, and performance status.

The audit verifies that the architecture document preserves these boundaries:

- one authoritative `WakeWordApplicationRuntime` in `AppState`;
- one canonical `WakeWordRuntimeManager`;
- `AppState::audio_capture` remains the authoritative microphone owner;
- Wake Word composition does not open a competing microphone device;
- fixed `Hey, Moose` / 16 kHz mono / one-thread / two-second pre-roll policy;
- no Wake Word V1 barge-in;
- component tests are not represented as real end-to-end acceptance;
- Linux x86_64 and macOS arm64 still require real KWS acceptance;
- Wake Word remains an implementation under qualification, not fully accepted cross-platform production functionality.

The documentation audit workflow path filters now include `docs/WAKE_WORD_V1_ARCHITECTURE.md`.

## TODO reconciliation note

This advances WWR-700 developer-documentation truthfulness and authoritative-source documentation coverage. It does not complete real KWS, integrated lifecycle, or performance acceptance.
