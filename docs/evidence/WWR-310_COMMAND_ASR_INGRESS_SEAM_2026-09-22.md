# WWR-310 — Command ASR ingress seam evidence and implementation plan

Date: 2026-09-22
Base master: `35fd556d6a99f4cd4dbe2746962e7186608bc193`

## Purpose

This note records the exact remaining production seam between Wake Word handoff audio and the existing normal command-ASR path. It is intentionally **not** a completion claim for WWR-310 or WWR-400.

The repository already contains the lower-level pieces needed for the seam:

- `WakeCommandHandoffAudio` owns canonical 16 kHz PCM produced by the Wake router without acoustic trimming.
- `WakeCommandAsrHandoff` is single-use and prevents stale or duplicate replay.
- `WakeCommandAsrIngress` is implemented for `LocalAsrPipeline`.
- `LocalAsrPipeline::prime_wake_handoff` feeds the existing bounded Moonshine command-ASR ingress queue before microphone capture continues.
- `activate_wake_command_once` suspends Wake and invokes a command-ASR ingress exactly once for deterministic unit coverage.

## Remaining production gap

The normal conversation start path still constructs `ConversationStartRequest` without a Wake handoff payload. Therefore the already-implemented Wake handoff boundary is not yet wired into the existing production command-ASR session start.

The required code-bearing slice is:

1. Add `wake_handoff: Option<WakeCommandHandoffAudio>` to `ConversationStartRequest`.
2. Keep manual starts passing `None` so disabled/manual behavior stays unchanged.
3. When `Some(handoff)` is present and `asr_mode` is `MoonshineTinyStreaming` or `MoonshineSmallStreaming`, prepare the local ASR pipeline, call `prime_wake_handoff(handoff)` before authoritative microphone capture starts, then continue through the existing `start_capture` and session attach path.
4. Fail closed before provider traffic or microphone capture if a Wake handoff is supplied while command ASR is not local Moonshine.
5. Preserve existing `GeminiLiveAudio` behavior for manual starts with no handoff.
6. Add focused tests for the request field and failure policy, then rely on exact-head Rust CI for compile/type coverage.

## Acceptance boundaries preserved

This note does not mark any WWR-310 checkbox complete. The following remain open until code and acceptance evidence exist:

- Activate the existing normal command ASR path exactly once.
- Real/reproducible `Hey Moose, tell me the time` audio acceptance.
- First command word presence in downstream ASR acceptance.
- Existing command ASR receives one continuous wake phrase plus command utterance.
- No first-word clipping occurs in deterministic acceptance.

This note also does not complete WWR-400 lifecycle acceptance. Startup/listen wiring, command interaction ownership, Talking suspension, TTS terminal resume, and real integrated lifecycle stability remain open.

## Why this seam is the next safe code boundary

The seam is narrow and preserves the one-authoritative-microphone model:

- It does not create another microphone owner.
- It does not create another ASR provider stack.
- It primes the already-running local command-ASR pipeline before its existing capture attachment.
- It keeps the wake phrase in memory-only PCM form and hands it to the same command-ASR path that would otherwise receive microphone audio.
- It preserves the V1 no-barge-in and no-acoustic-trimming policies.

## Qualification required for the code-bearing slice

Before merging the implementation slice, exact PR-head ordinary CI must pass with Rust checks enabled by the changed source paths. Any Wake-specific lifecycle gate triggered by the source diff must also pass. The implementation cannot be accepted from documentation evidence alone.
