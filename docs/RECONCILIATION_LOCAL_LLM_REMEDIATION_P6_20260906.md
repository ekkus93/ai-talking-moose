# Local LLM Remediation P6 Reconciliation — 2026-09-06

## Status

**P6 runtime-cancellation/template truthfulness remediation is prepared for validation.**

This record covers `LLMR-600` through `LLMR-603`. It does not close the authoritative TODO until exact-head CI, expected-head merge, and exact merged-master CI succeed.

## LLMR-600 — Cancellation ownership audit

Normal `TextModel::generate()` has no caller cancellation parameter. `LocalTextModel::generate()` creates a fresh private `CancellationToken` and passes it to `LocalRuntimeManager::generate()`. The runtime supports cooperative checks, including direct/test use of a pre-cancelled token, but normal typed/ambient callers cannot signal the private token.

Model changes do not cancel active inference: the next serialized request may load a different model. Delete waits on the shared operation lock, then unloads/removes. Shutdown marks the runtime shutting down so new work is rejected, then waits on that same operation lock subject to the application's five-second outer timeout. Speech cancellation controls synthesis/playback, not Local LLM generation.

## LLMR-601 — No new user-facing cancellation API

**N/A by design for current V1 product semantics.** There is no current typed/ambient “cancel generation” product action and the provider-neutral `TextModel` trait has no cancellation ownership contract. Adding Local-only external cancellation would create asymmetric provider semantics and is outside this remediation. Documentation/comments are corrected so cooperative runtime capability is not overstated as application-exposed cancellation.

## LLMR-602 — Chat-template ownership

The application—not llama.cpp's generic Jinja executor—renders the supported family ChatML shape after validating the embedded GGUF template's required family invariants. Qwen3 non-thinking prefill and defensive reasoning-output sanitization remain explicit application runtime policy. Misleading comments are corrected. No prompt framing changes are made.

## LLMR-603 — Template compatibility hardening decision

A separate normalized/template-only fingerprint is intentionally not introduced. Catalog identity already pins the complete GGUF SHA-256 and P2 runtime-use verification rehashes the whole artifact before first load. That cryptographic boundary is stronger and less brittle than hashing one metadata field. The embedded-template check is therefore explicitly a semantic compatibility gate. Existing fixtures prove supported SmolLM2/Qwen3 family semantics are accepted and deliberately altered/missing invariants fail closed.

Because prompt rendering, model identity, loading, and generation semantics are unchanged by P6, this phase alone does not require a P12 real-model rerun. The final `LLMR-803` decision must still consider the complete remediation diff.

## Required closure evidence

- Rust formatting/tests and ordinary repository gates on the exact P6 head;
- source review confirming no misleading native-template or externally-cancellable-generation claim remains in active Local LLM docs/code comments;
- expected-head merge and exact merged-master CI before tracker closure.
