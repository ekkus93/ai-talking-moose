# Wake Word V1 — Implementation Notes

**Date:** 2026-09-14
**Implementation base:** `595016a09f33fee094045f5966bbddb2ea8eaa08`

This file records implementation-time invariants that refine, but do not replace, `WAKE_WORD_V1_SPEC_2026-09-14.md` and `WAKE_WORD_V1_TODO_2026-09-14.md`.

- `AudioCapture` remains the single authoritative physical microphone owner. Wake Word V1 must not open a second CPAL input stream.
- Wake listening uses the same canonical 16 kHz mono PCM format already consumed by Local ASR.
- `PcmRingBuffer` is engine-independent, fixed at two seconds for V1, memory-only, and never logged or serialized.
- Command handoff is legal only after an accepted KWS trigger. Ordinary listening state cannot manufacture a command handoff.
- Wake settings are additive fields in the existing settings document: `wake_word_enabled` defaults to `false`, and `wake_phrase` normalizes to the fixed V1 phrase `Hey, Moose`. Adding these fields does not change ASR/TTS provider ownership and does not require a conversation restart by itself.
- sherpa KWS remains one-thread CPU inference with the frozen threshold/score contract until measured acceptance evidence justifies a change.
- Static sherpa runtime packaging is pinned for Linux x86_64, macOS arm64, and the existing macOS x86_64 bundle target. Mandatory real KWS inference acceptance remains Linux x86_64 plus macOS arm64.
- Wake-triggered command sessions are one-shot interactions. Manual conversation sessions retain their existing persistent multi-turn behavior.
- Wake capture must be inactive while Moose is physically playing its response. Wake listening resumes only after command-session teardown and stale pre-roll clearing.
