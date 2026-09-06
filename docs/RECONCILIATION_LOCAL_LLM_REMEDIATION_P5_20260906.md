# Local LLM Remediation P5 Reconciliation — 2026-09-06

## Status

**P5 frontend settings-write hardening implementation is prepared for exact-head validation.**

This record covers `LLMR-500` through `LLMR-502`. It does not mark the authoritative TODO complete before exact-head PR CI, expected-head merge, and post-merge `master` validation.

Implementation is stacked on P4 head `c0b8fa536a1fd94a621c52c534ec90004479d7f0`. P5 must be rebased onto the exact merged P4 `master` before its final PR merge gate if P4 squash-merges to a different commit identity.

## LLMR-500 — Patch-oriented component intent

The Zustand store now exposes `updateSettingsPatch` and `updateSettingsContinuousPatch`. Components submit only fields they intend to change. The store applies those patches to its current optimistic view while preserving the existing serialized persistence queue, pending-continuous folding, persisted-baseline rebasing, and failure reconciliation. The backend IPC still receives a complete `AppSettings` candidate built by the store.

## LLMR-501 — Settings caller migration

All ordinary Settings UI callers are migrated away from `updateSettings({ ...settings, ... })` and equivalent continuous full-object spreads. Local/Google model and provider controls, ASR selection, general, behavior, privacy, voice/audio, and personality controls now submit narrow patches. Multi-field privacy reset remains a deliberate four-field patch rather than a complete replacement.

## LLMR-502 — Stale-caller proof

`AiTabSettingsPatch.test.tsx` captures an old Local-model callback from render A, advances unrelated persisted state to B, then invokes the stale callback. The resulting persisted candidate must retain B while applying only the Local-model change. Existing store tests continue to cover discrete/continuous ordering, coalescing, in-flight write serialization, and rejected-write reconciliation.

## Validation state

The implementation must pass frontend formatting/typecheck/tests, the Settings full-object-spread audit, exact-head ordinary repository CI, expected-head merge, and post-merge `master` CI before P5 tracker closure is recorded.
