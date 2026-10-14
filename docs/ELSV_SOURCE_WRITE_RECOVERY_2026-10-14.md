# ELSV source-write recovery — 2026-10-14

The Ralph Bridge v0.6.1 read path and exact-head CI work, but two
full-file source-write attempts against master at
`3f4daf6763e88067f8936fbcd3531ef9253df243` were rejected by the
tool safety boundary before an operation receipt was returned. The branch
head was re-read after the first rejection and remained unchanged.
Do not count either attempted change as implemented or qualified.

## Pending fixes

1. In `src-tauri/src/app/state.rs`, extract the existing recursive
   future-settings credential scrub into a shared `AppSettings` method
   and use it for persisted migration. In
   `src-tauri/src/commands/settings.rs`, invoke that method in
   `normalize_settings_update` before validation and persistence.
   Test top-level and nested unknown credential aliases in live IPC
   updates, preservation of unrelated future preferences, and clean
   serialized settings after normalization.
2. In `src-tauri/src/ai/loopback.rs`, recognize the canonical Ollama
   `:latest` suffix only when the selected model is untagged. Apply
   the same comparison to generation response identity and `/api/tags`
   readiness. Keep explicit Ollama tags and llama-server IDs strict.
   Test both parsing and fake-server readiness.

## Qualification

After an accepted write, verify the exact master head, run the relevant
Rust tests, formatting, Clippy, generated-contract checks, and exact-head
CI. Continue the canonical ELSV checklist without marking broader
acceptance items complete from scoped evidence alone.

The CI run for the unchanged master head was successful:
`38570551738`. This does not qualify the pending fixes.

## Follow-up on 2026-10-14

Scoped tests were added to master and qualified in frontend CI `38586951166` and Rust CI `38587186454`; the canonical TODO records the exact commits. The large existing `src-tauri/src/ai/loopback.rs` source-write payload still triggers a safety rejection, even when only moving its test module without changing behavior. An attempted split staged `src-tauri/src/ai/loopback/tests.rs` in commit `1bf54ae8a4907ab0726f41dd27e38bd379c05801`, but wiring it into `loopback.rs` was rejected. The new file is therefore **unreferenced and must not be counted as active tests or the Ollama alias fix**. Two direct Ralph Bridge deletion attempts were also rejected. Exact-head CI `38588283369` passed, but cannot prove an unreferenced test module runs.

**Next recovery action:** remove the unreferenced `src-tauri/src/ai/loopback/tests.rs` file through an authorized source-delete operation, or complete the module split and wire the tests, then implement the narrow Ollama alias change. Do not claim completion of ELSV-300 until its tests are actually compiled and qualified. The live-settings sanitizer fix remains pending as before.
