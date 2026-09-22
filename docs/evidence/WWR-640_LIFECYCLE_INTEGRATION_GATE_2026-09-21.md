# WWR-640 lifecycle integration gate evidence — 2026-09-21

This change expands the existing deterministic Wake Word lifecycle stability workflow so it exercises the integration boundaries that own the single microphone stream and command lifecycle, rather than only the runtime-manager stability module.

The specialized workflow now runs four focused Rust test groups on the exact PR head:

- `wake_word_stability`: repeated trigger/Talking/resume cycles, bounded ring/pre-roll state, repeated terminal TTS outcomes, repeated disable/enable, and shutdown while Listening/Triggered;
- `wake_word_authoritative_capture`: reuse of the exact shared application `AudioCapture`, capture stop during command handoff, deterministic return/cancellation behavior, failed start, and reconnect/error ownership;
- `wake_word_capture_orchestrator`: serialized capture routing/recovery and fail-closed device/capture behavior;
- `wake_word_command_lifecycle`: suspension across command ASR/Thinking/Talking and common success/cancel/recoverable-failure terminal ownership policy.

The workflow path filter now includes those production integration modules, so changes to capture ownership, routing, recovery, or command lifecycle cannot silently bypass the specialized lifecycle gate.

This is deterministic integration/stability evidence only. It does **not** claim completion of WWR-640's real-audio bounded soak, native-session resource measurements, real false-trigger acceptance, or representative performance evidence. Those remain pending their required acceptance environments/corpus.
