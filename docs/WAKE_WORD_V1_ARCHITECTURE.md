# Wake Word V1 architecture

This document describes the Wake Word subsystem that exists on `master`. It deliberately distinguishes implemented component behavior from production integration and acceptance work that is still open in `WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`.

## User-visible policy

Wake Word V1 uses the fixed phrase **`Hey, Moose`** and is disabled by default. Arbitrary phrase editing and sensitivity controls are not V1 features. Keyword spotting is designed to run locally/offline. When Wake Word is eventually enabled end-to-end, the microphone must remain locally active while the runtime is listening for the phrase; this does not mean continuous cloud transcription is active.

V1 has no barge-in. Wake activation must be suspended before/during Moose speech so playback cannot wake the application. After a terminal interaction/TTS outcome, the Wake Word runtime may resume only if the latest setting still enables it.

The wake phrase and immediately following prompt are intentionally retained in the memory-only pre-roll/live handoff. V1 does not acoustically trim `Hey, Moose` before command ASR. Depending on the selected command-ASR provider, that command audio may later be processed by that provider; local Wake Word detection itself must not silently introduce a cloud dependency.

## Authoritative source ownership

The authoritative runtime state machine is `src-tauri/src/asr/wake_word_runtime.rs` (`WakeWordRuntimeManager`). The application composition wrapper is `src-tauri/src/app/wake_word_composition.rs` (`WakeWordApplicationRuntime`). KWS configuration/engine policy lives under the consolidated `src-tauri/src/app/wake_word*` and `src-tauri/src/asr/wake_word*` modules; removed legacy stacks are not alternative authorities.

`WakeWordRuntimeManager` owns the bounded canonical-PCM ring/pre-roll state and lifecycle phases. Accepted triggers transition out of `Listening`, which makes repeated positive KWS frames non-activating until the interaction lifecycle explicitly returns the manager to listening. Trigger count and monotonic trigger age are diagnostics; raw PCM is not part of the diagnostics representation.

`WakeWordApplicationRuntime` is intentionally not a microphone owner. `AppState::audio_capture` remains the production capture owner. WWR-300 must route that one canonical capture stream into Wake Word rather than opening a second capture device.

## Current integration status

The runtime and application-composition components exist and have deterministic component tests, including disabled startup, enabled startup through `Loading`, Talking suspension, stale-audio clearing, terminal-outcome resume, disable-during-Talking precedence, recoverable error re-entry, and one-manager clone semantics.

Production integration is **not yet complete**. At the time this document was added, `AppState` does not yet own `WakeWordApplicationRuntime`, the live settings transaction does not yet drive that owner, the canonical microphone stream is not yet routed through KWS, and wake-to-command-ASR pre-roll/live handoff is not yet wired end-to-end. Therefore this document does not claim that Wake Word V1 is currently usable end-to-end.

## Audio and privacy invariants

The canonical Wake Word input is 16 kHz mono PCM. Retained wake audio is bounded and memory-only by design. Entering Talking, disabling Wake Word, resuming an interaction, runtime errors, and shutdown must clear stale ring/pre-roll state at the defined lifecycle boundary. Diagnostics and errors must not serialize raw audio, credentials, or unnecessary absolute filesystem paths.

Wake Word must never create an independent full-time ASR path merely to detect the phrase. It must not silently fall back to cloud recognition if local KWS is unavailable. Manual interaction must remain available when Wake Word is disabled or has a recoverable error.

## Platform and acceptance claims

Do not infer platform support merely because source compiles or artifacts are described. Linux x86_64 and macOS arm64 Wake Word support claims require the real sherpa KWS acceptance tracks in the remediation TODO, including exact artifact/runtime hashes, native architecture verification, positive and negative fixtures, offline inference, and recorded run/platform evidence.

Likewise, do not make subjective accuracy or performance claims until deterministic corpus and performance evidence exists. The final feature head requires the specialized Wake gates in addition to ordinary CI.

## Troubleshooting boundaries

Useful diagnostics include enabled/runtime state, exact model/runtime identity, platform/architecture, one-thread policy, canonical sample format, ring capacity, score/threshold, trigger count, last-trigger age, initialization duration, Talking suspension, and sanitized last error. Optional CPU, memory, inference, and handoff timing fields should be populated only from measured evidence.

If Wake Word remains `Loading`, investigate artifact/runtime preparation rather than claiming it is listening. If it enters `Error`, manual interaction should remain usable. A stuck `SuspendedTalking` state, duplicate capture stream, duplicate command activation, or retained stale pre-roll is a lifecycle defect and must not be hidden by an arbitrary cooldown timer.

## Related evidence

See `docs/evidence/WWR-400_APPSTATE_RUNTIME_OWNER_AUDIT_2026-09-19.md` for the production composition gap, `docs/evidence/WWR-410_DEBOUNCE_TRIGGER_SEMANTICS_2026-09-19.md` for debounce evidence, and the WWR-510 evidence documents for privacy-safe diagnostic/error audits. The remediation TODO remains authoritative for incomplete work and final acceptance.