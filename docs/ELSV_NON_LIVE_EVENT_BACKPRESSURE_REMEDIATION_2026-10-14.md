# ELSV non-Live event backpressure and stale-generation remediation

**Date:** 2026-10-14
**Status:** Implemented at `89bbd6df9a52d854bc27c237ac3d4822ff99b270`; exact-head Rust qualification pending
**Scope:** ELSV-400 generation identity, barge-in, privacy and event ordering

## Reproduction and root cause

The non-Live adapter at `src-tauri/src/ai/local/voice.rs` owns a generation counter and a cancellation token per finalized text turn. Generation and standalone TTS are cancelled on interruption or replacement. However, publication of `LiveServerEvent::ModelTranscript` uses an unconditional awaited send to a bounded Tokio channel.

The existing test `interruption_during_transcript_backpressure_never_starts_cloud_tts` creates a one-slot event channel. `Connected` occupies that slot, so the generated transcript waits for capacity. Calling `interrupt()` cancels the token and attempts to queue `Interrupted`, but the transcript send is not cancellation-aware. After the consumer drains `Connected`, the old `ModelTranscript` can be delivered before `Interrupted`. The test currently **expects** that stale transcript, while verifying that standalone TTS is not started.

This is not an external cloud-text retry or TTS fallback. It is a stale UI/transcript event delivered after barge-in, contrary to ELSV-400's generation-identity and stale-response-suppression requirement.

## Implementation and remaining qualification

1. Commit `89bbd6df9a52d854bc27c237ac3d4822ff99b270` added `publish_turn_event` with a biased cancellation branch and a generation check. It covers transcript, provider-error, audio, and completion event publication.
2. The same commit introduced `ActiveTurnCleanup` so a completed/stale turn cannot clear the cancellation token for a newer turn, and cancellation ownership survives blocked event sends.
3. The transcript-backpressure test now expects `Interrupted` without stale `ModelTranscript`. A new audio-backpressure test asserts interruption suppresses queued audio and completion.
4. Remaining focused follow-up: add explicit blocked-error and blocked-completion cases and replacement-generation tests, then run full ELSV-400 routing matrix.
5. The exact-head Rust CI for `89bbd6d` (run `38686419939`) was **cancelled** when a subsequent documentation commit arrived. It does not qualify the fix. Obtain a passed exact-code-head Rust quality/test/generated-contract run before checking the ELSV-400 cancellation subtask.

## Current qualification and integration boundaries

The separate regression for dropping an interrupted external text-generation future landed at `8a18ba78630e14e201cca3a1f36ca08e92575de7`. Unsupported System TTS preflight and immutable text-provider selection landed at `264aeba85894b01898a999a22c095f7aeb8f3828` and `9fc700e7ed928e54480644a1f5f43ce88dfc8e95`. Exact-head CI `38684700623` passed for the latter SHA. Those changes do **not** resolve the stale transcript event described here.

An earlier direct source-write attempt was rejected by an upstream safety check. A subsequent authorized direct-to-`master` write landed the implementation at `89bbd6d`. The implementation is no longer absent; **CI qualification is still outstanding** because its run was superseded by documentation changes.

## Platform and privacy constraints

Do not connect to Gemini Live to work around non-Live event backpressure. Preserve the selected text provider and standalone TTS provider without fallback. Do not emit stale transcripts into persistence, UI, diagnostics, or audio synthesis after a user interrupt. Native macOS System TTS and its physical audition remain independent, unfinished ELSV-500 work.
