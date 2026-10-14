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
