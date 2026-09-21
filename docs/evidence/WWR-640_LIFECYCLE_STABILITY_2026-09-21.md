# WWR-640 lifecycle stability evidence

Date: 2026-09-21
Current evidence base: `df24417717542a4b3774a0b0b363ac71988c30f7`
Implementation commit: `93b24d60d497b01c835c97879bbe7be96912ee52` (`test(wake): add lifecycle stability regression coverage (#337)`)

## Deterministic stability coverage now on master

`src-tauri/src/app/wake_word_lifecycle_stability_tests.rs` exercises the authoritative application Wake runtime through bounded repeated lifecycle transitions.

- `repeated_lifecycle_cycles_remain_bounded_and_return_to_listening` executes 100 Listening → Triggered → SuspendedTalking → Listening cycles. Every cycle verifies ring and handoff pre-roll counts stay at or below the fixed capacity, Talking clears retained audio, and resume returns to Listening with zero stale retained samples. The final trigger count is exactly 100.
- `repeated_disable_enable_cycles_do_not_leave_stale_audio_or_state` executes 50 disable/enable cycles, verifies disable clears ring/pre-roll, and verifies each re-enable follows Loading → Listening rather than bypassing initialization.
- `shutdown_while_listening_is_terminal_and_clears_retained_audio` verifies shutdown from Listening clears retained PCM and rejects re-enable.
- `shutdown_during_triggered_handoff_is_terminal_and_clears_pre_roll` verifies shutdown after trigger clears handoff pre-roll and remains terminal.

## Exact-head CI evidence

The specialized `Wake Word lifecycle stability` workflow passed on PR #338 exact head `27961b516b8d09e05c4db7ec90a2a2d71f85d64e` as run `35654320418`.

The same specialized workflow passed again on merged master `0bb26753d4b3f1acffa9c6f5fe90dd5c6a115040` as run `35655168166`.

These runs execute the lifecycle stability test target rather than relying on ordinary CI alone.

## WWR-640 checklist evidence

Objective coverage exists for:

- repeated wake-state lifecycle cycles;
- ring-buffer memory remaining bounded;
- repeated disable/enable cycles;
- shutdown while Listening;
- shutdown during triggered handoff;
- state/resource-count observations on every deterministic cycle (phase, ring samples, handoff samples, trigger count).

Existing composition tests also cover repeated terminal resolution for success/cancellation/recoverable-failure policy at the Wake owner boundary, but this document does not upgrade those component checks into full production TTS end-to-end acceptance.

## Still open

WWR-640 is not complete. Production-integrated acceptance must still prove no native session growth, no capture-stream multiplication, end-to-end TTS success/cancellation/failure resume behavior, and an appropriate bounded soak/false-trigger run. Those claims depend on completion of WWR-300/310/400 production routing and real KWS acceptance and must not be inferred from manager-level deterministic tests.
