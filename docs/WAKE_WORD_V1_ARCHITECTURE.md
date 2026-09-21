# Wake Word V1 architecture and current support

Wake Word V1 is the local keyword-spotting subsystem for the fixed phrase **Hey, Moose**. This document describes the implementation that exists on `master`; it does not treat planned acceptance work as completed support.

## User-visible policy

Wake Word is disabled by default and can be enabled or disabled from normal Settings. The phrase is fixed to **Hey, Moose** in V1; arbitrary phrase editing and sensitivity controls are intentionally not exposed.

When enabled, keyword spotting is designed to run locally/offline against the pinned sherpa-onnx KWS model/runtime. The microphone therefore remains locally active while Wake Word is listening. A wake-triggered command may subsequently enter the normal command-ASR path, whose provider and privacy behavior are separate from idle keyword spotting. V1 does not implement barge-in: Wake Word is suspended while Moose is talking.

Manual interaction remains available when Wake Word is disabled or when the Wake Word runtime has failed closed.

## Authoritative subsystem

Production application composition owns one `WakeWordApplicationRuntime` in `AppState`. That application runtime wraps the single canonical `WakeWordRuntimeManager`. Clones share the manager state rather than constructing independent Wake Word owners.

The authoritative application microphone owner remains `AppState::audio_capture`. Wake Word composition deliberately does not open a microphone device. The intended WWR-300 production routing model is one capture stream feeding canonical PCM to the ring buffer/KWS/handoff path; a second continuous Wake Word capture stream is not part of the design.

The canonical V1 KWS policy is fixed at:

- 16 kHz mono PCM;
- feature dimension 80;
- one inference thread;
- keyword `HEY MOOSE` / user phrase `Hey, Moose`;
- keyword score 1.0;
- threshold 0.25;
- two seconds of in-memory pre-roll.

Non-canonical PCM is rejected before Wake Word retention/inference boundaries.

## Lifecycle

The runtime state machine uses Disabled, Loading, Listening, Triggered, SuspendedTalking, Error, and ShuttingDown phases.

Persisted disabled state constructs a Disabled runtime. Persisted enabled state enters Loading and does not claim Listening until the KWS runtime is actually marked loaded. Trigger acceptance snapshots chronological pre-roll and moves to Triggered. Repeated positive frames are ignored until the interaction lifecycle resets the runtime.

Command/TTS ownership suspends Wake Word. Entering suspension clears retained ring/pre-roll state. Successful completion, cancellation, and recoverable failure use the same terminal policy: return to Listening only when Wake Word remains enabled. If the user disables Wake Word during an interaction, Disabled wins over resume. Shutdown is terminal and command completion cannot resurrect the runtime.

This lifecycle is intentionally state-based rather than timer/cooldown based. No V1 cooldown is required for the deterministic one-trigger/one-interaction invariant.

## Audio retention and handoff

Wake Word pre-roll is memory-only PCM held in a bounded ring buffer. Runtime diagnostics expose sample counts/capacity, not PCM payloads. The handoff implementation is designed to snapshot pre-roll chronologically, preserve live samples after the trigger, and transfer ownership once to command ASR without duplicate ranges or inversion.

Component-level deterministic tests cover chronological snapshotting, invalid-frame rejection before mutation, one-shot pre-roll take, stale-buffer clearing, repeated-trigger suppression, and wake/command lifecycle reset behavior.

Those component tests are **not** a substitute for the still-open real/reproducible `Hey Moose, tell me the time` acceptance, first-command-word acceptance, or complete production capture→KWS→ASR integration acceptance.

## Native model/runtime identity

V1 pins immutable identities for the selected GigaSpeech KWS model inputs and sherpa-onnx native runtime artifacts. Preparation and runtime verification fail closed on byte/hash mismatch, missing artifacts, or wrong native architecture. The implementation currently contains platform-specific identity checks for Linux x86_64 and macOS arm64.

Pinned identities and packaging code do not by themselves constitute a supported-platform claim. Linux x86_64 and macOS arm64 remain subject to their dedicated real-KWS acceptance tasks before documentation may claim end-to-end platform support.

Model provenance/license and sherpa runtime license/attribution are distinct concerns and are recorded separately in repository evidence/notices.

## Privacy and diagnostics

Wake Word diagnostics are intentionally bounded. They can report enabled/runtime state, model/runtime identity, platform/architecture, one-thread policy, sample format, ring capacity, threshold/score, trigger count, last-trigger age, initialization duration, Talking suspension, and sanitized error state.

Diagnostics do not serialize raw PCM. Detection events contain only the fixed keyword identity and score. Engine error sanitization redacts path-like and token-like values, and runtime-state errors use bounded operational text. Native keyword-result handling reduces results to keyword presence instead of propagating native JSON, token arrays, transcripts, or audio.

Idle KWS is not intended to perform full transcription or silently fall back to cloud ASR. Any command-ASR activity after a trigger is part of the normal command interaction and must follow the configured ASR policy.

## Current acceptance boundary

The following work remains open and must not be described as completed merely because component tests pass:

- complete production one-stream microphone routing and capture ownership acceptance;
- end-to-end wake→command-ASR pre-roll/live handoff acceptance with real/reproducible audio;
- deterministic corpus recall/false-trigger acceptance;
- real Linux x86_64 KWS inference acceptance;
- real macOS arm64 KWS inference acceptance;
- measured CPU, memory, inference, and handoff performance evidence;
- repeated integrated lifecycle/resource stability acceptance;
- specialized final Wake CI gates and exact-head closeout.

Until those gates are complete, Wake Word V1 should be described as an implementation under qualification rather than as fully accepted cross-platform production functionality.

## Troubleshooting boundaries

If Wake Word is disabled, manual interaction should continue normally. If the runtime reports Loading, the application has not yet established a usable KWS session. Error means Wake Word failed closed; manual interaction should remain available, and re-enabling may re-enter Loading. SuspendedTalking is expected while the command/TTS lifecycle owns the interaction. ShuttingDown is terminal for that application lifetime.

Artifact/runtime failures should be diagnosed from the sanitized error and pinned identity evidence rather than by logging PCM, transcripts, credentials, or unnecessary absolute paths.
