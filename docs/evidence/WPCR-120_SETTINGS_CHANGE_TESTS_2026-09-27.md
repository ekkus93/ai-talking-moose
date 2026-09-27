# WPCR-120 Settings Change Test Evidence

**Date:** 2026-09-27
**Remediation item:** WPCR-120 — Handle input-device and ASR-mode settings changes while Wake is enabled
**Exact merged master:** `195c2df1bdb83f2e8af176579fcd25d7f1bd4273`
**PR:** #481 — `test(wake): cover enabled settings-change listener policy`

## Scope

This evidence records the focused deterministic Settings-change tests merged for WPCR-120. It is not final WPCR-120 closeout: full successful input-device restart on a real configured listener, pending restart while a conversation is active, and restart-failure behavior remain broader acceptance items.

## Evidence

PR #481 added `src-tauri/src/app/wake_word_settings_change_tests.rs` and registered it from `src-tauri/src/app/mod.rs`.

The new tests cover:

- enabled ASR-mode change from supported local Moonshine ASR to unsupported Gemini Live audio;
- fail-closed Wake state for unsupported ASR-mode changes while Wake is enabled;
- no false active-listener claim after that fail-closed transition;
- enabled input-device change through the configured listener settings boundary when no startup listener configuration is available;
- `Loading` state without an active-listener claim for that no-config restart boundary.

PR #481 also stabilized `listener_thread_keeps_non_send_engine_local_and_terminates_capture` by waiting for the listener to emit `Started` before issuing shutdown. That prevents the mock capture-close path from racing ahead of the intentional shutdown assertion and preserves the intended guarantee that intentional shutdown reports `Stopped`, not `CaptureFailed("CaptureClosed")`.

## Exact validation

Exact PR head `70b6a07835f12bda2290dafedc34bd1ed4bf54c3` passed:

- ordinary CI `36305652775`;
- Wake Word source-security audit `36305652742`.

Exact merged master `195c2df1bdb83f2e8af176579fcd25d7f1bd4273` passed:

- ordinary CI `36306132208`;
- Wake Word source-security audit `36306132212`.

## Boundaries

This evidence satisfies the focused test coverage for unsupported ASR-mode changes while Wake is enabled and adds coverage for the no-config input-device restart boundary. It does not claim:

- successful real listener restart on a new input device;
- pending restart replay after an active conversation ends;
- restart-failure diagnostics for configured hardware/runtime failures;
- final WPCR-950/WPCR-960 qualification.
