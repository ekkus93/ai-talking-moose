# WWR-310 command activation boundary evidence — 2026-09-22

## Scope

This evidence records the current source-level Wake Word handoff-to-command-ASR activation boundary. It complements `WWR-310_HANDOFF_BOUNDARY_2026-09-22.md` by covering the delivery side of the handoff payload.

## Source evidence

`src-tauri/src/app/wake_word_command_activation.rs` exposes `activate_wake_command_once`. The boundary:

- suspends the shared Wake runtime before command ASR receives audio;
- delivers the provider-neutral `WakeCommandHandoffAudio` through `WakeCommandAsrHandoff::deliver_once`;
- keeps Wake suspended after successful delivery so command ASR, Thinking, and Talking own the interaction until a terminal lifecycle outcome;
- consumes stale handoff audio if command ASR startup fails;
- resumes or disables Wake according to the latest `wake_word_enabled` setting after startup failure.

`src-tauri/src/app/wake_word_command_asr_ingress.rs` defines the provider-neutral `WakeCommandAsrIngress` trait and implements it for the local `LocalAsrPipeline`, so the activation boundary primes the already-selected command-ASR path rather than creating a second ASR pipeline.

`src-tauri/src/app/wake_word_command_handoff.rs` owns the single-use command-ASR handoff coordinator and verifies canonical non-empty 16-kHz PCM handoff audio.

## Regression coverage

Existing deterministic tests cover:

- an accepted trigger activates command ASR exactly once;
- a second activation attempt with the same handoff payload returns false and does not replay audio;
- successful activation keeps Wake suspended for the rest of the command interaction;
- command-ASR startup failure consumes stale audio and returns Wake to Listening when still enabled;
- command-ASR startup failure honors a disabled setting instead of resuming Listening;
- synthetic pre-roll/live ranges are delivered without gaps, duplicates, or inversion.

Representative test locations include `src-tauri/src/app/wake_word_command_activation.rs`, `src-tauri/src/app/wake_word_command_asr_ingress.rs`, and `src-tauri/src/app/wake_word_command_handoff.rs`.

## Validation

Exact merged-master validation for `b7c49df5acda8614bb31f8115875b2b91b7726b7` passed ordinary CI `35764401214`.

## Non-claims

This evidence does not claim that a durable production listener task currently calls the activation boundary from real native KWS events. WWR-300/WWR-400 production listener wiring and WWR-640 integrated lifecycle soak remain open.

This evidence also does not claim real audio fixture acceptance or downstream ASR transcription of `Hey Moose, tell me the time`; WWR-600, WWR-610, and WWR-620 remain open.