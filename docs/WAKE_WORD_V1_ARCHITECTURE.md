# Wake Word V1 architecture

This document describes the Wake Word implementation that exists on `master`. It intentionally distinguishes implemented component behavior from acceptance work that is still open.

## User-visible policy

Wake Word V1 uses the fixed phrase **Hey, Moose** and is disabled by default. Settings exposes an enable/disable control but does not expose arbitrary phrase editing or sensitivity tuning.

Keyword spotting is designed to run locally/offline. While Wake Word is enabled and listening, the microphone is expected to remain locally active for keyword spotting; that does **not** mean full-time cloud transcription is active. V1 does not support Wake Word barge-in while Moose is talking.

## Authoritative ownership

`AppState::wake_word_runtime` is the sole application-level `WakeWordApplicationRuntime` owner. The process-global duplicate runtime owner was removed. The canonical Wake Word facade is `src-tauri/src/app/wake_word.rs`, which exposes the authoritative runtime manager, engine policy, handoff, diagnostics, settings, and artifact manifest boundaries.

Wake Word does not own a second microphone capture object. `AppState::audio_capture` remains the authoritative application capture owner. Production conversation startup uses that same capture object for command ASR.

## Runtime lifecycle

The runtime phases are `Disabled`, `Loading`, `Listening`, `Triggered`, `SuspendedTalking`, `Error`, and `ShuttingDown`.

Persisted disabled settings construct a `Disabled` runtime. Persisted enabled settings enter `Loading`; the runtime must not claim `Listening` until loading has actually completed.

Normal command-conversation startup suspends an already-listening Wake runtime before command ASR takes ownership. Terminal command outcomes resolve the runtime against the latest persisted enable setting. Explicit stop and conversation-start failure also resolve a suspended runtime. Entering the Talking suspension boundary clears retained Wake audio. Wake Word errors are sanitized and fail closed rather than creating a replacement capture stream.

## Audio and privacy policy

The frozen V1 KWS format is 16 kHz mono PCM, one inference thread, score `1.0`, threshold `0.25`, and a two-second pre-roll policy. PCM is validated before Wake retention. Retained pre-roll is memory-only component state; diagnostics cannot serialize raw PCM, transcripts, credentials, or filesystem paths.

Diagnostics expose bounded operational information such as enabled/runtime state, model/runtime identity, platform/architecture, sample format, ring capacity, score/threshold, trigger count, last-trigger age, initialization duration, Talking suspension, and sanitized last error.

The Wake phrase plus immediately following command audio is intentionally not acoustically trimmed in V1. When the production wake-to-ASR handoff is complete, command ASR may therefore receive the Wake phrase together with the command utterance.

## Artifact/runtime identity

The production KWS model and sherpa runtime identities are pinned and hash verified. Preparation verifies cached artifacts rather than trusting cache presence. Supported native architecture claims must remain limited to platforms backed by the repository's real acceptance evidence.

## Current acceptance boundary

The following must **not** be inferred merely from the component architecture above:

- production continuous microphone routing into the KWS engine;
- complete wake-trigger to normal command-ASR activation;
- gap-free pre-roll/live PCM handoff and first-command-word preservation;
- real positive/negative corpus acceptance on every claimed platform;
- final repeated end-to-end lifecycle/resource stability;
- final performance baselines;
- final repository-wide privacy/security audit.

Those remain tracked by `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`. User-facing documentation must not describe Wake Word V1 as fully production-qualified until those acceptance items are complete.

## Relevant source boundaries

- `src-tauri/src/app/state.rs` — authoritative application composition.
- `src-tauri/src/app/wake_word.rs` — canonical Wake Word facade.
- `src-tauri/src/app/wake_word_composition.rs` — application runtime owner.
- `src-tauri/src/asr/wake_word_runtime.rs` — runtime state machine and bounded pre-roll ownership.
- `src-tauri/src/app/wake_word_engine.rs` — pinned sherpa KWS engine/session implementation.
- `src-tauri/src/asr/wake_word_handoff.rs` — wake-to-ASR handoff component.
- `src-tauri/src/asr/wake_word_diagnostics.rs` — privacy-safe diagnostics representation.
- `src-tauri/src/commands/conversation/core.rs` — production command-interaction lifecycle wiring.
- `src/components/Settings/WakeWordSettingsPanel.tsx` — user-facing Wake Word settings surface.
