# Local LLM Remediation P5 Reconciliation — 2026-09-06

## Status

**P5 frontend settings-write hardening is prepared for exact-head validation.**

This record covers `LLMR-500` through `LLMR-502`. It does not mark the authoritative TODO complete before exact-head PR CI, expected-head merge, and post-merge `master` validation.

Implementation base: `e34ae4fde6f6648fdcd5b87b942942d407cd9ea3`, the exact P4-closure `master` that passed CI `34059510111`.

## LLMR-500 — Patch-oriented component intent

The Zustand store now exposes discrete and continuous patch-intent APIs. Components send only fields they intend to change. The store applies each patch to its current optimistic/reconciled settings view and the existing persistence coordinator rebases queued patches on the last successfully persisted settings snapshot. Ordered persistence, continuous coalescing, failure reconciliation, and safe error handling remain owned by the existing V1R-216/V1R-217 coordinator.

The Tauri bridge remains a complete-`AppSettings` persistence boundary; only the store constructs that complete object. This is intentional because Rust's `update_settings` command persists the authoritative complete settings document.

## LLMR-501 — Settings caller migration

AI text-provider/model controls, Local LLM model selection, Gemini text/live model selectors, ASR mode, General, Behavior, Personality, Privacy, and Voice/Audio controls now submit patches instead of spreading a rendered `settings` snapshot into a complete object. Privacy reset remains a multi-field patch because those four fields are intentionally one semantic operation.

## LLMR-502 — Stale-caller regression

A deterministic frontend test retains the callback captured by a Local LLM model control under settings snapshot A, advances an unrelated transcript-retention setting to B, then invokes the stale callback. The Local model change persists while B survives both optimistic state and the complete object sent through the Tauri bridge. This fails under the former full-object spread behavior. Existing store tests continue to cover continuous coalescing, continuous-to-discrete folding, in-flight write ordering, rejected-write rebase, and reconciliation using patch intent.
