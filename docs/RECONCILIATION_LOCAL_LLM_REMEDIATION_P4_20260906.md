# Local LLM Remediation P4 Reconciliation — 2026-09-06

## Status

**P4 settings compatibility and request-consistency implementation is prepared for exact-head validation.**

This record covers `LLMR-400` through `LLMR-403`. It does not mark the authoritative TODO complete before exact-head PR CI, expected-head merge, and post-merge `master` validation.

Implementation base: `b8ac4d5c0b23ef2f2f513263f04b369823963672`, the exact `master` that passed CI `34048541984` after the P3 closure and SettingsModal race repair.

## LLMR-400 — Future settings versions fail closed

`AppSettings::from_persisted_json()` inspects `settings_version` on the raw JSON value before deserializing or normalizing it. A version greater than `CURRENT_SETTINGS_VERSION` returns typed `PersistedSettingsError::FutureVersion` with safe schema-version metadata only. Startup propagates that compatibility error before any normalized settings write. A persistent-database regression fixture proves a future-version document containing an unknown future field remains byte-for-byte unchanged after startup rejects it.

## LLMR-401 / LLMR-402 — One immutable request snapshot

`TextRequestSettingsSnapshot` captures one `AppSettings` clone plus a character configuration whose settings-derived fields are overwritten from that same clone. Typed and ambient text-generation paths derive provider selection, provider-specific model ID, memory inclusion, prompt/personality configuration, and typed transcript-retention policy from that snapshot. `AppState::get_text_model_for(&AppSettings)` constructs the provider from supplied settings and never re-reads mutable global settings.

## LLMR-403 — Deterministic concurrency proof

Typed and ambient regression tests use Tokio barriers to pause after snapshot capture and before prompt/provider invocation. While paused, settings change from A to B. The in-flight request retains A provider/error semantics, model/privacy/memory/configuration and retention snapshot; the next request observes B consistently. Ambient delivery deliberately preserves the existing second current-state privacy gate after generation, so a newly disabled observation setting can still suppress already-generated output.

## Validation state

The implementation branch must pass Rustfmt, Clippy, the full Rust test suite, exact-head repository CI, expected-head merge, and post-merge `master` CI before P4 tracker closure is recorded.
