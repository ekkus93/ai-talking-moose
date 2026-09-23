# Wake Word V1 architecture

Wake Word V1 provides a fixed local wake phrase, **“Hey, Moose”**, for starting the existing command interaction. The feature is disabled by default. When enabled and healthy, keyword spotting runs locally/offline; it does not continuously send microphone audio to a cloud service.

## Authoritative ownership

Production composition has one `WakeWordApplicationRuntime` in `AppState` and a single canonical `WakeWordRuntimeManager`. The authoritative application microphone owner remains `AppState::audio_capture`. Wake Word composition deliberately does not open a microphone device; `AuthoritativeWakeCaptureOwner` borrows that same shared capture used by normal command capture and serializes Wake capture, transfer to command ASR, return to Wake listening, cancellation, disable, and recovery.

The canonical Wake input is 16 kHz mono PCM. `CanonicalWakePcmRouter` validates that stream before retention or inference. The same chronological canonical samples feed both two seconds of in-memory pre-roll and the sherpa KWS engine. Invalid PCM fails before it can mutate retained Wake state.

## Native KWS

Production KWS uses the pinned sherpa-onnx runtime and model identities in `wake-word-artifacts.json`. Artifact preparation verifies byte sizes, SHA-256 identities, and native architecture before use and fails closed on mismatch. The fixed V1 policy is one inference thread, score `1.0`, threshold `0.25`, and a two-second pre-roll. The KWS engine performs keyword spotting only; it is not a full-time transcription path.

Component tests are **not** a substitute for native-platform qualification. Linux x86_64 and macOS arm64 remain subject to their dedicated real-KWS acceptance tasks. Until those tasks and the integrated acceptance sections are complete, Wake Word V1 is implementation under qualification rather than as fully accepted cross-platform production functionality.

## Wake-to-command handoff

On an accepted trigger, the runtime snapshots pre-roll chronologically and retains subsequent live canonical samples while command ASR initializes. Transfer is single-use. Wake capture stops on the shared `AudioCapture` before normal command capture replaces it, so Wake and command ASR do not open competing microphone streams.

V1 deliberately does **not** acoustically trim the wake phrase. The handoff therefore may contain both “Hey, Moose” and the immediately following command, and that wake phrase plus prompt may reach the selected command ASR. The existing Moonshine command-ASR ingress accepts the handoff before subsequent live microphone PCM so the boundary can remain gap-free and ordered.

The handoff is memory-only. Raw Wake PCM is not written to diagnostics or logs, and the ring/pre-roll is cleared at lifecycle boundaries that invalidate retained audio.

## Lifecycle

The intended integrated lifecycle is:

1. Disabled setting → `Disabled`; manual interaction remains available.
2. Enabled startup → `Loading` while verified native KWS resources are prepared.
3. Successful preparation and eligible idle state → `Listening`.
4. One accepted trigger → one normal command interaction, with Wake activation suspended before command ASR owns the microphone.
5. Thinking/Talking/TTS keep Wake activation suspended. V1 does not implement barge-in.
6. Terminal command/TTS success, cancellation, or recoverable failure clears stale retained audio, resets KWS state where required, and returns to `Listening` only if Wake is still enabled.
7. If Wake is disabled during an interaction, terminal resolution ends in `Disabled`, never an unconditional resume.
8. Wake-specific runtime/capture failure fails Wake closed without preventing the ordinary manual interaction path.

Some lifecycle wiring and real acceptance remain tracked as open work in `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`; this document describes the authoritative design and must not be read as evidence that unchecked acceptance items have passed.

## Privacy and diagnostics

Wake diagnostics expose state and bounded metadata such as enabled/runtime phase, model/runtime identity, platform/architecture, fixed policy, ring capacity, trigger count/timing, initialization timing, and sanitized errors. They must not expose raw PCM, audio content, credentials, or unnecessary absolute paths.

The microphone remains locally active while Wake is enabled and listening. This behavior is disclosed in Settings. Normal inference has no network dependency after verified artifacts are prepared.

## Source map

- `src-tauri/src/app/state.rs` — authoritative application composition and shared capture owner.
- `src-tauri/src/app/wake_word_composition.rs` — application Wake runtime composition/lifecycle boundary.
- `src-tauri/src/app/wake_word_authoritative_capture.rs` — serialized use of the shared `AudioCapture`.
- `src-tauri/src/app/wake_word_pcm_router.rs` — canonical PCM routing, pre-roll, trigger/live handoff ordering.
- `src-tauri/src/app/wake_word_command_handoff.rs` — owned single-use handoff audio.
- `src-tauri/src/app/wake_word_command_asr_ingress.rs` — provider-neutral command-ASR ingress boundary.
- `src-tauri/src/app/wake_word_command_activation.rs` — one-shot command activation boundary.
- `src-tauri/src/app/wake_word_command_lifecycle.rs` — suspend/resume lifecycle policy.
- `src-tauri/src/app/wake_word/` and `src-tauri/src/app/wake_word_engine.rs` — runtime state and native sherpa KWS implementation.
- `src-tauri/src/asr/pipeline.rs` — existing Moonshine command-ASR pipeline and Wake handoff ingress.
- `wake-word-artifacts.json` — authoritative immutable model/runtime identities.
