# ELSV non-Live event backpressure and stale-generation remediation

**Date:** 2026-10-14  
**Status:** Open implementation defect; no fix claimed  
**Scope:** ELSV-400 generation identity, barge-in, privacy and event ordering

## Reproduction and root cause

The non-Live adapter at `src-tauri/src/ai/local/voice.rs` owns a generation counter and a cancellation token per finalized text turn. Generation and standalone TTS are cancelled on interruption or replacement. However, publication of `LiveServerEvent::ModelTranscript` uses an unconditional awaited send to a bounded Tokio channel.

The existing test `interruption_during_transcript_backpressure_never_starts_cloud_tts` creates a one-slot event channel. `Connected` occupies that slot, so the generated transcript waits for capacity. Calling `interrupt()` cancels the token and attempts to queue `Interrupted`, but the transcript send is not cancellation-aware. After the consumer drains `Connected`, the old `ModelTranscript` can be delivered before `Interrupted`. The test currently **expects** that stale transcript, while verifying that standalone TTS is not started.

This is not an external cloud-text retry or TTS fallback. It is a stale UI/transcript event delivered after barge-in, contrary to ELSV-400's generation-identity and stale-response-suppression requirement.

## Required code changes

1. In `LocalConversationSession::send_text_turn`, publish the model transcript through a cancellation-aware `tokio::select!` with a biased cancellation branch. Drop the pending send when its generation is interrupted or replaced; retain the existing generation/active-synthesis check before invoking TTS.
2. Change the backpressure regression to require `Interrupted` as the first event after the consumer drains `Connected`. Assert no subsequent stale transcript, audio, or standalone TTS request.
3. Audit the analogous awaited sends of provider `Error`, `AudioData`, and `TurnComplete`. A generation check *before* a blocked send is insufficient if interruption can occur while awaiting capacity. Keep cancellation ownership alive until the final event is published or abandoned.
4. Add negative tests for cancellation while an error/audio/completion send is backpressured, including replacement by a newer turn. Ensure new-turn events are not discarded by cancellation of the old token.
5. Run exact-head Rust formatting/Clippy, the complete Rust test suite, and generated-contract checks. Confirm the scoped CI is **passed**, not superseded or cancelled, before marking the ELSV-400 cancellation subtask complete.

## Current qualification and integration boundaries

The separate regression for dropping an interrupted external text-generation future landed at `8a18ba78630e14e201cca3a1f36ca08e92575de7`. Unsupported System TTS preflight and immutable text-provider selection landed at `264aeba85894b01898a999a22c095f7aeb8f3828` and `9fc700e7ed928e54480644a1f5f43ce88dfc8e95`. Exact-head CI `38684700623` passed for the latter SHA. Those changes do **not** resolve the stale transcript event described here.

A direct source change for the cancellation-aware transcript send was attempted during the 2026-10-14 loop, but Ralph Bridge's source-write call was rejected by an upstream safety check. The working tree and `master` remain unchanged for this specific fix. Reattempt through an authorized source-write path in a future run; do not mark this defect resolved merely because this report exists.

## Platform and privacy constraints

Do not connect to Gemini Live to work around non-Live event backpressure. Preserve the selected text provider and standalone TTS provider without fallback. Do not emit stale transcripts into persistence, UI, diagnostics, or audio synthesis after a user interrupt. Native macOS System TTS and its physical audition remain independent, unfinished ELSV-500 work.
