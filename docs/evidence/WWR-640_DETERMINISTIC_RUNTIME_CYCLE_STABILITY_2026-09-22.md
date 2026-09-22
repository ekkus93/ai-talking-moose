# WWR-640 deterministic runtime-cycle stability evidence

Date: 2026-09-22
Baseline master: `7b9801a4eb26a8106042c36a88129762a7375fdf`

## Scope

This evidence records the deterministic, hardware-independent portion of WWR-640 that is already exercised by the authoritative Wake Word application runtime. It intentionally does **not** claim real native-session, physical capture-device, or end-to-end command-ASR/TTS soak acceptance.

The production-facing test module `src-tauri/src/app/wake_word_lifecycle_stability_tests.rs` is compiled through `src-tauri/src/app/mod.rs` and exercises the same `WakeWordApplicationRuntime` / `WakeWordRuntimeManager` state owner used by `AppState`.

## Deterministic coverage

`repeated_lifecycle_cycles_remain_bounded_and_return_to_listening` executes 100 trigger -> Talking suspension -> resume cycles. Every cycle asserts:

- the runtime reaches `Triggered` after one accepted trigger;
- retained ring and handoff sample counts never exceed the configured ring capacity;
- Talking suspension clears ring and handoff audio;
- terminal interaction resolution returns to `Listening`;
- resumed state contains no stale ring or handoff audio;
- the final privacy-safe trigger count is exactly 100.

`repeated_terminal_tts_outcomes_resume_cleanly_without_retained_audio` executes 64 cycles for each terminal semantic outcome represented by the shared production policy: success, cancellation, and recoverable failure. Each cycle proves the runtime leaves `SuspendedTalking`, returns to `Listening`, and retains no stale PCM.

`repeated_disable_enable_cycles_do_not_leave_stale_audio_or_state` executes 50 disable/enable cycles and proves disable clears retained audio, re-enable enters `Loading`, and successful load returns to `Listening`.

`shutdown_while_listening_is_terminal_and_clears_retained_audio` proves shutdown from Listening clears retained audio and cannot be re-enabled afterward.

`shutdown_during_triggered_handoff_is_terminal_and_clears_pre_roll` proves shutdown during an active trigger/handoff clears pre-roll and cannot resume afterward.

## WWR-640 items objectively supported by this deterministic layer

The source tests provide objective component-level evidence for:

- ring-buffer memory remaining bounded across repeated runtime cycles;
- successful terminal interaction/TTS resolution repeatedly resuming Wake Word;
- cancellation terminal resolution repeatedly resuming Wake Word through the same production policy;
- recoverable-failure terminal resolution repeatedly resuming Wake Word through the same production policy;
- repeated disable/enable cycles;
- shutdown while Listening;
- shutdown during trigger/handoff;
- privacy-safe state/resource counters (ring samples, handoff samples, trigger count) checked on every cycle or terminal boundary.

## Claims deliberately left open

WWR-640 remains incomplete until specialized acceptance exercises the real native sherpa session, the authoritative physical/OS microphone capture owner, normal command ASR, Thinking, real TTS playback, and resource/session/capture-stream counts under a bounded soak. In particular, these deterministic tests do not prove absence of native-session growth or physical capture-stream multiplication.

This distinction prevents component tests from being promoted into unsupported end-to-end acceptance claims.
