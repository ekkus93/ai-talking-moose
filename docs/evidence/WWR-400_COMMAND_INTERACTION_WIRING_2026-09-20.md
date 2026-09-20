# WWR-400 — Production command-interaction lifecycle wiring evidence

Date: 2026-09-20
Baseline master: `d9b9133c5c138d4b7d54a599da698b7357d0bf43`
Implementation PR: #279
Qualified PR head: `d96a3eb84389baab95d356c848f95ec22ece508c`
Exact PR-head ordinary CI: `35516672507` (success)
Exact merged-master ordinary CI: `35517020086` (success)

## Production wiring established

`src-tauri/src/commands/conversation/core.rs` now invokes the AppState-owned Wake Word command lifecycle guard at the normal production conversation command boundary.

Before `ConversationManager::start_session` can take command-ASR microphone ownership, `start_conversation` calls `suspend_for_command_interaction` on `AppState::wake_word_runtime`. A Listening or Triggered Wake runtime therefore enters `SuspendedTalking` before normal command ASR starts. Disabled, Loading, Error, and ShuttingDown Wake states do not falsely become enabled by a manual command interaction.

The production `ConversationLifecycle` callback keeps the guard associated with that exact command interaction. Terminal `Idle` or `Failed` outcomes resolve the Wake runtime through `resume_after_command_interaction` using the *current* persisted `wake_word_enabled` setting rather than the stale setting captured at command start. Consequently, disabling Wake Word while an interaction is active wins over an otherwise normal resume.

Conversation-start failures that occur before a terminal lifecycle callback are also bounded: `start_conversation` checks for the still-suspended runtime and resolves it against the current enable setting before returning the error. Explicit `stop_conversation` performs the same terminal resolution after command capture/session teardown.

The guard implementation in `src-tauri/src/app/wake_word_command_lifecycle.rs` already has focused regression coverage for enabled suspend/resume, disable-during-interaction, disabled/manual behavior, and repeated suspension while ASR/Thinking owns the interaction.

## Objectively supported WWR-400 requirements

This production wiring supports the following implementation-level requirements:

- prevent wake activation while command ASR is active;
- prevent wake activation while Thinking is active;
- suspend Wake Word for the no-barge-in command interaction boundary;
- clear retained Wake pre-roll when entering the suspension boundary (performed by the authoritative runtime transition);
- resume after a recoverable command interaction terminal outcome;
- honor disabling Wake Word during an interaction instead of unintentionally resuming;
- preserve disabled/manual command behavior.

## Limits intentionally not claimed

This evidence does **not** claim the remaining end-to-end Wake trigger → command activation path, production continuous microphone routing into KWS, wake pre-roll/live PCM transfer into command ASR, real positive/negative KWS fixture acceptance, or complete TTS success/cancellation/failure acceptance. Those remain governed by WWR-300, WWR-310, the remaining WWR-400 acceptance items, and WWR-600+.

A separate source review also found that `update_settings` persists `wake_word_enabled` but does not yet directly call `WakeWordApplicationRuntime::apply_enabled_setting`; that runtime-setting synchronization remains an implementation item and must not be inferred complete from this evidence.
