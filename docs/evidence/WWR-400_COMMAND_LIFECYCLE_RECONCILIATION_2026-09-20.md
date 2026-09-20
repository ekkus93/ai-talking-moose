# WWR-400 — Command interaction lifecycle reconciliation

Date: 2026-09-20
Baseline master: `0663e1c132a56a6964fb271162486b5508d67d90`

## Source evidence

`src-tauri/src/commands/conversation/core.rs` now integrates the AppState-owned `WakeWordApplicationRuntime` with the normal command interaction path. `start_conversation` clones `state.wake_word_runtime`, calls `suspend_for_command_interaction` before constructing and starting the normal ASR/provider graph, and resolves the runtime on terminal conversation lifecycle events (`Idle` or `Failed`) against the current persisted `wake_word_enabled` setting. Conversation start failures and explicit `stop_conversation` also resolve a suspended runtime deterministically.

`src-tauri/src/app/wake_word_command_lifecycle.rs` provides the bounded lifecycle guard. It suspends `Listening` or `Triggered` Wake Word runtime states before command ASR/Thinking/Talking owns the interaction, treats an already suspended runtime as guarded, leaves disabled/loading/error/shutdown states unchanged for manual interactions, and resumes via `WakeWordApplicationRuntime::resume_after_interaction` using the latest enable state.

`src-tauri/src/app/runtime_preferences.rs` applies live Wake Word enable/disable changes through the AppState-owned runtime via `apply_changed_runtime_preferences`. That path applies the runtime change before persistence, rolls it back when a later reversible preference side effect or settings persistence fails, and leaves an unchanged Wake setting undisturbed.

`src-tauri/src/app/wake_word_composition.rs` now supports the important terminal edge case fixed by PR #286: if Wake Word is enabled during a manual interaction while the runtime is still `Disabled`, terminal resume enters the normal enabled `Loading` path instead of failing or remaining disabled.

## Qualification evidence

The command lifecycle implementation is covered by ordinary CI and by the Wake Word lifecycle stability gate across the relevant PRs:

- PR #278 added the command-interaction lifecycle guard and focused tests for normal resume, disabled/manual behavior, disable-during-interaction, and repeated suspension.
- PR #279 wired the guard into the production `start_conversation`, terminal lifecycle, start-failure, and explicit stop paths.
- PR #286 fixed the enable-during-manual-interaction terminal resume edge case and passed exact PR-head ordinary CI plus Wake Word lifecycle stability.
- Exact merged master `0663e1c132a56a6964fb271162486b5508d67d90` passed ordinary CI run `35527309134` and Wake Word lifecycle stability run `35527309162`.

## WWR-400 items objectively supported

The current source and qualification evidence support reconciliation of these WWR-400 lifecycle items:

- prevent wake activation while command ASR is active;
- prevent wake activation while Thinking is active;
- suspend wake activation on or before Talking;
- keep Wake Word suspended throughout TTS playback/normal command interaction;
- resume after successful terminal completion when Wake Word remains enabled;
- resume/resolve after cancellation or explicit stop;
- resume/resolve after recoverable command failure;
- ensure disabling Wake Word during an interaction results in `Disabled` rather than an unintended resume;
- preserve disabled/manual behavior when Wake Word is off;
- preserve the V1 no-barge-in policy for normal command interactions;
- avoid leaving the runtime permanently suspended across repeated lifecycle cycles.

## Still open

This evidence does not claim the unresolved production microphone routing and wake-trigger path. The following remain open until WWR-300/310 integration is implemented and accepted:

- continuous microphone PCM routing into KWS from the authoritative capture path;
- accepted Wake trigger starting exactly one normal command interaction;
- ring/pre-roll clearing at the production Talking boundary;
- stale pre-roll clearing and KWS reset at the final production handoff/resume boundary;
- complete end-to-end lifecycle acceptance with real Wake trigger input.
