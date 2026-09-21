# WWR-400 / WWR-410 lifecycle and trigger evidence

Date: 2026-09-21
Baseline master: `a05df5286c4b3e65508658d3f60567e62c8ea882`
Baseline merged-master CI: `35640329089` (success)

This evidence records only behavior directly demonstrated by the current source and tests. It does not claim production microphone composition, real-audio acceptance, or end-to-end command activation where those remain open in the remediation TODO.

## WWR-400 lifecycle evidence

`src-tauri/src/app/wake_word_command_lifecycle.rs` provides the single command-interaction guard boundary used to suspend and resume Wake Word ownership.

Objective test evidence currently present on master:

- `command_interaction_suspends_wake_until_terminal_resume` proves an enabled Listening runtime enters `SuspendedTalking` for command ownership and returns to `Listening` after the terminal boundary.
- `suspended_runtime_remains_guarded_across_asr_and_thinking` proves repeated guard application while command ASR/Thinking owns the interaction remains suspended rather than reactivating Wake Word.
- `every_recoverable_terminal_outcome_returns_to_listening_when_enabled` proves success, cancellation, and recoverable failure all return an enabled runtime to `Listening`.
- `every_terminal_outcome_honors_disable_during_interaction` proves disabling during an interaction wins over success, cancellation, and recoverable-failure resume paths and leaves the runtime `Disabled`.
- `disabled_manual_interaction_never_enables_wake` proves the command guard preserves manual interaction semantics when Wake Word is disabled.
- `wake_error_does_not_block_manual_command_guard`, merged in PR #326 at master `4f16b892ed6f2bfbb9807ac78fac26eeb8baef1b`, proves Wake Word `Error` does not block the manual command guard and that a recoverable terminal outcome moves an enabled runtime back through `Loading` rather than stranding command ownership.

`src-tauri/src/asr/wake_word_runtime.rs` additionally proves that `suspend_for_talking` clears retained audio and rejects Wake activation while suspended, while `resume_after_interaction` clears stale audio before returning to Listening.

These tests support lifecycle checkbox reconciliation, but they do not by themselves prove the final WWR-400 end-to-end acceptance statement that one integrated production lifecycle controls Wake Word through the physical microphone, command ASR, Thinking, and TTS stack.

## WWR-410 debounce and trigger evidence

`src-tauri/src/asr/wake_word_runtime.rs` test `repeated_positive_frames_create_one_trigger_until_interaction_resets` proves that a second trigger attempt while the first interaction is `Triggered` is rejected, trigger count remains one, and a later trigger is accepted only after `resume_after_interaction` returns ownership to Listening.

`src-tauri/src/app/wake_word_pcm_router.rs` provides stronger router-level evidence:

- `repeated_positive_frames_after_trigger_do_not_duplicate_command_activation` proves that after the first accepted detection, subsequent PCM is retained only as live handoff audio and is not re-fed to KWS or accepted as another trigger. Trigger count remains one.
- `handoff_transfer_is_single_use_and_return_resumes_listening` proves the handoff is transferred once, KWS is not fed while command ownership is active, and `return_to_wake_listening` resets the KWS stream before Listening resumes.
- `later_phrase_after_return_to_listening_yields_second_trigger_without_cooldown`, merged in PR #327 at master `a05df5286c4b3e65508658d3f60567e62c8ea882`, proves a later phrase after lifecycle return to Listening produces a second accepted trigger, increments trigger count to two, and does so without a cooldown timer.

The current implementation therefore has objective evidence for repeated-positive suppression, reset-at-resume semantics, later-phrase acceptance after resume, and correctness without a cooldown. This evidence does not upgrade the broader one-wake-event-to-one-normal-command-interaction acceptance until production command activation wiring is itself proven.

## Qualification references

- PR #326 exact-head ordinary CI passed before merge; merged master `4f16b892ed6f2bfbb9807ac78fac26eeb8baef1b` subsequently passed ordinary CI run `35635649071`.
- PR #327 exact head `31b47094f713d18a2fe7f6bda5e796a3763a72b5` passed ordinary CI run `35639339678` before guarded merge.
- PR #327 merged master `a05df5286c4b3e65508658d3f60567e62c8ea882` passed ordinary CI run `35640329089`.

No real-audio, specialized-runner, platform-acceptance, or production-microphone claim is inferred from these ordinary CI results.
