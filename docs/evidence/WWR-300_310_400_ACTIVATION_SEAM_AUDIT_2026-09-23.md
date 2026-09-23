# WWR-300/310/400 — remaining activation seam audit

Commit audited: `13ce526dce43848fb8b848ce56ff27df8d5885e3`

This evidence file records the exact remaining production seam after the already-merged Wake capture, handoff, and one-shot activation building blocks. It is intentionally not a completion claim for WWR-300, WWR-310, or WWR-400.

## Already present on master

- `AppState` owns one `WakeWordApplicationRuntime` and one shared `AppState::audio_capture`.
- `AuthoritativeWakeCaptureOwner` starts/stops/restarts Wake capture through the shared `AudioCapture` owner rather than constructing another microphone owner.
- `CanonicalWakePcmRouter` validates one canonical 16 kHz mono stream before both ring-buffer retention and KWS feed.
- `WakeCommandHandoffAudio` owns a single chronological wake phrase plus post-trigger live payload.
- `WakeCommandAsrHandoff` makes command-ASR delivery single-use and consumes stale payloads on failure.
- `LocalAsrPipeline::prime_wake_handoff` can enqueue the handoff into the existing bounded Moonshine ingress before command microphone capture appends later PCM.
- `wake_word_command_activation::activate_wake_command_and_start_normal_asr_once` expresses the one-trigger/one-command invariant at the unit boundary.

## Remaining production gap

The accepted Wake trigger path still needs a production orchestration owner that performs these steps as one lifecycle transaction:

1. Observe an accepted KWS trigger from the running authoritative Wake capture loop.
2. Stop/transfer the Wake handoff from the shared capture owner.
3. Prime the selected command-ASR ingress with that handoff before normal command microphone capture starts.
4. Start the existing normal command interaction exactly once.
5. Keep Wake suspended through ASR/Thinking/Talking/TTS.
6. Resume Wake only if enabled after terminal command/TTS success, cancellation, or recoverable failure.
7. End in `Disabled` if Wake was disabled during the interaction.
8. Preserve manual listen if Wake fails closed.

## File/function targets for the next code-bearing slice

- `src-tauri/src/conversation/session.rs`
  - `ConversationStartRequest` is the production request object for the existing normal command path.
  - `ConversationManager::start_session` prepares local Moonshine before provider connection and starts command microphone capture later.
  - The safe insertion point for a local-ASR Wake handoff is after `prepare_local_asr(...)` succeeds and before `start_capture(...)` starts normal command microphone capture.

- `src-tauri/src/asr/pipeline.rs`
  - `LocalAsrPipeline::prime_wake_handoff` already implements the bounded ingress enqueue and validates that the local ASR worker is running.
  - The next slice should reuse it; it should not add another ASR path or bypass queue bounds.

- `src-tauri/src/commands/conversation/core.rs`
  - `start_conversation` currently constructs the ordinary `ConversationStartRequest` for manual and command starts.
  - Wake-triggered production orchestration should reuse the same command-start path rather than cloning conversation setup.

- `src-tauri/src/app/wake_word_authoritative_capture.rs`
  - `transfer_to_command_asr` stops the shared Wake capture owner before handing payload ownership to command ASR.
  - The production loop should call this boundary before starting normal command capture so no competing streams exist.

- `src-tauri/src/app/wake_word_command_activation.rs`
  - `activate_wake_command_and_start_normal_asr_once` currently proves one-shot activation at a deterministic seam.
  - The next slice should connect the real conversation starter to this boundary or replace it with an equivalent production path covered by tests.

## Tests required before marking TODO checkboxes

The next code-bearing PR must include deterministic tests for at least:

- one accepted Wake trigger starts exactly one normal command interaction;
- repeated positive frames during transition do not start duplicate command interactions;
- Wake handoff audio is primed before command capture appends microphone PCM;
- command-start failure consumes stale handoff and returns Wake to a recoverable enabled/disabled state;
- disabled-during-interaction resolves to `Disabled`, not `Listening`;
- Wake error/fail-closed state does not prevent manual command start.

Real/reproducible audio and native-platform acceptance remain separate WWR-600/610/620/630/640 work and must not be inferred from this audit.
