# WWR-640 component lifecycle stability evidence — 2026-09-22

## Scope

This evidence records deterministic bounded lifecycle stability already exercised on master. It deliberately distinguishes component/runtime stability from the still-required integrated production Wake→ASR→Thinking→Talking→wake acceptance.

## Existing deterministic coverage

`src-tauri/src/app/wake_word_lifecycle_stability_tests.rs` runs 100 repeated trigger→Talking→resume cycles and verifies every cycle returns to Listening with ring/pre-roll storage bounded by the configured capacity and cleared across Talking/resume boundaries.

The same suite runs 64 repeated cycles for each terminal TTS semantic outcome (success, cancellation, recoverable failure), verifying all outcomes return to Listening without retained audio. It also runs 50 repeated disable/enable cycles and verifies Disabled→Loading→Listening transitions do not retain stale ring or handoff samples.

Shutdown coverage verifies both shutdown while Listening and shutdown during a Triggered handoff are terminal, clear retained audio, and reject unintended resume/re-enable operations.

`src-tauri/src/app/wake_word_authoritative_capture.rs` separately verifies start/disable, command transfer/return, cancellation, failed return, and reconnect all reuse the exact same shared `Arc<Mutex<AudioCapture>>` owner.

## Validation baseline

Exact merged-master ordinary CI for `ec8f746ec8ffa0656c1186625d4975fcd6e9ae92` passed in run `35774661341`.

## Remaining WWR-640 acceptance

This is not final WWR-640 acceptance. The repository still needs an integrated production listener path using the real native KWS session and normal command/TTS path, with resource/session/capture counts observed across repeated end-to-end cycles. A bounded false-trigger soak also remains pending real corpus/native acceptance. Those items must remain open until exact-head evidence exists.
