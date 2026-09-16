# Wake Word V1 lifecycle stability evidence

Date: 2026-09-15

This record captures the deterministic WW-820 lifecycle-stability slice merged by PR #144. It is deliberately narrower than final real-native KWS acceptance: it validates the engine-independent runtime lifecycle and bounded-memory invariants already implemented in Rust.

## Exact-head evidence

- PR head: `46f643813151d513d109bd221a51212eab04a1c4`
- Ordinary CI run: `35037532050`
- Ordinary CI conclusion: success
- Guarded squash merge: PR #144
- Merged master SHA: `7cad28f7a73e80b4d6f4df5f2736f82bbfdfa318`

The path-scoped P21-P23 Rust Stability Acceptance workflow was skipped because the change did not touch its governed paths; it is not counted as Wake Word qualification evidence.

## Deterministic coverage

`src-tauri/src/asr/wake_word_stability.rs` exercises:

- 256 repeated wake -> Talking suspension -> resume cycles;
- duplicate-trigger suppression while a command handoff is already active;
- bounded ring/pre-roll storage across repeated cycles;
- 128 disable -> enable cycles; and
- clean shutdown from both Listening and Triggered/command-handoff states.

These tests run inside the ordinary complete Rust test suite, so the exact PR head is covered by the successful ordinary CI run above.

## Claims this evidence supports

This slice supports the engine-independent portions of WW-820: repeated lifecycle transitions do not leave the runtime stuck, duplicate command activation is suppressed, retained PCM remains bounded, disable/enable cycling is stable, and shutdown from listening/handoff states is clean.

It does **not** by itself prove native sherpa session memory stability, physical microphone-stream multiplicity, real TTS cancellation behavior, real-device disconnect behavior, or false-trigger soak performance. Those claims remain open until the corresponding native/integration acceptance exists. This distinction prevents deterministic state-machine tests from being misrepresented as real-platform acceptance.
