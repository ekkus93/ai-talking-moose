# Local LLM Remediation P5 Reconciliation — 2026-09-06

## Status

**P5 frontend settings-write hardening is complete.**

This record closes `LLMR-500` through `LLMR-502`, the stale frontend full-object overwrite probe under `LLMR-003`, and the two `LLMR-003` probe acceptance criteria.

## Exact evidence

- P5 implementation base: `e34ae4fde6f6648fdcd5b87b942942d407cd9ea3` — exact P4-closure `master`.
- Clean P5 implementation head: `38ff0ad2129d203bd3bd9677123d0c4c482bc7d1` — one implementation commit, no authoring workflow/scripts.
- Pre-PR P5 validator: Actions run `34071137944` — success.
- Exact-head implementation PR CI: Actions run `34071222525` — success on `38ff0ad2129d203bd3bd9677123d0c4c482bc7d1`.
- Implementation PR: #57, squash-merged with an expected-head guard.
- P5 implementation merge on `master`: `b2251de2317b6e5c8832f0b1829847668e2ad228`.
- Exact post-merge `master` CI: Actions run `34089027966` — success on `b2251de2317b6e5c8832f0b1829847668e2ad228`.

## LLMR-500 — Patch-oriented component intent

The Zustand store exposes discrete and continuous patch-intent APIs. Components send only fields they intend to change. The store applies each patch to its current optimistic/reconciled settings view and the existing persistence coordinator rebases queued patches on the last successfully persisted settings snapshot. Ordered persistence, continuous coalescing, rejected-write reconciliation, and safe error handling remain owned by the existing V1R-216/V1R-217 coordinator.

The Tauri bridge remains a complete-`AppSettings` persistence boundary; only the store constructs that complete object. This is intentional because Rust's `update_settings` command persists the authoritative complete settings document.

## LLMR-501 — Settings caller migration

AI text-provider/model controls, Local LLM model selection, Gemini text/live model selectors, ASR mode, General, Behavior, Personality, Privacy, and Voice/Audio controls now submit patches instead of spreading a rendered `settings` snapshot into a complete object. Privacy reset remains a multi-field patch because those four fields are intentionally one semantic operation.

The P5 authoring validator included a static code-search guard that rejected legacy Settings-component `updateSettings(...)` calls and patch calls containing a stale `...settings` spread. That guard passed before the clean implementation tree was published.

## LLMR-502 — Stale-caller regression

A deterministic frontend test retains the callback captured by a Local LLM model control under settings snapshot A, advances an unrelated transcript-retention setting to B, then invokes the stale callback. The Local model change persists while B survives both optimistic state and the complete object sent through the Tauri bridge. This is the direct regression for the former stale full-object overwrite behavior.

Existing store tests continue to cover continuous coalescing, continuous-to-discrete folding, in-flight write ordering, rejected-write rebase, and reconciliation using patch intent. The focused P5 suite passed 35/35 tests in `34071137944`, and the ordinary exact-head and post-merge repository CI generations also passed.

## LLMR-003 probe closure

The stale frontend full-object overwrite probe is now implemented and passing. With P1 through P5 complete, every deterministic review probe listed under `LLMR-003` is present or reconciled, and all of those probes are offline/hardware-free. The tracker acceptance boxes are therefore closed without expanding the owner-deferred real-model, physical Mac audio/TCC, signing, notarization, or voice-audition scope.
