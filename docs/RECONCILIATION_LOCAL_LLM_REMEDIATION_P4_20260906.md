# Local LLM Remediation P4 Reconciliation — 2026-09-06

## Status

**P4 settings compatibility and request-consistency remediation is complete and validated on merged `master`.**

This record closes `LLMR-400` through `LLMR-403` and the P4-owned `LLMR-003` future-settings-version destructive-downgrade and split-settings-snapshot request-race probes. The stale frontend full-object overwrite probe and overall `LLMR-003` acceptance remain open for P5.

Implementation base: `b8ac4d5c0b23ef2f2f513263f04b369823963672`, the exact `master` that passed CI `34048541984` after the P3 closure and SettingsModal race repair.

## LLMR-400 — Future settings versions fail closed

`AppSettings::from_persisted_json()` inspects `settings_version` on the raw JSON value before deserializing or normalizing it. A version greater than `CURRENT_SETTINGS_VERSION` returns typed `PersistedSettingsError::FutureVersion` with safe schema-version metadata only. Startup propagates that compatibility error before any normalized settings write. A persistent-database regression fixture proves a future-version document containing an unknown future field remains byte-for-byte unchanged after startup rejects it.

## LLMR-401 / LLMR-402 — One immutable request snapshot

`TextRequestSettingsSnapshot` captures one `AppSettings` clone plus a character configuration whose settings-derived fields are overwritten from that same clone. Typed and ambient text-generation paths derive provider selection, provider-specific model ID, memory inclusion, prompt/personality configuration, and typed transcript-retention policy from that snapshot. `AppState::get_text_model_for(&AppSettings)` constructs the provider from supplied settings and never re-reads mutable global settings.

## LLMR-403 — Deterministic concurrency proof

Typed and ambient regression tests use Tokio barriers to pause after snapshot capture and before prompt/provider invocation. While paused, settings change from A to B. The in-flight request retains A provider/error semantics, model/privacy/memory/configuration and retention snapshot; the next request observes B consistently. Ambient delivery deliberately preserves the existing second current-state privacy gate after generation, so a newly disabled observation setting can still suppress already-generated output.

## Validation evidence

- Pre-PR implementation validator `34053015237`: success. It passed Rustfmt, `cargo fmt --check`, Clippy, the full Rust test suite, `git diff --check`, and published the workflow-free implementation tree.
- Clean implementation head: `c0b8fa536a1fd94a621c52c534ec90004479d7f0`, one commit on base `b8ac4d5c0b23ef2f2f513263f04b369823963672`.
- PR #53: `Local LLM: enforce request-scoped settings consistency`.
- Exact-head PR CI `34053798461`: success on `c0b8fa536a1fd94a621c52c534ec90004479d7f0`; all ordinary jobs passed, including Frontend quality, Rust quality, all three Local LLM compile proofs, dependency/release gates, both macOS bundles, and canonical `npm run check:all`.
- PR #53 was squash-merged with the expected-head guard.
- Merged `master`: `9ed82604b54308951bcd172ee82402729f6a60e0`, tree `4c4b00bdab91f1856b4c121901cbe0735855bef8`.
- Post-merge `master` CI `34055657716`: success on exact SHA `9ed82604b54308951bcd172ee82402729f6a60e0`; every ordinary job passed.

## Closure

`LLMR-400`, `LLMR-401`, `LLMR-402`, and `LLMR-403` are complete. The P4-owned future-settings-version destructive-downgrade and split-settings-snapshot request-race probes in `LLMR-003` are complete. `LLMR-003` itself remains open because P5 still owns the stale frontend full-object overwrite probe and its final aggregate acceptance.
