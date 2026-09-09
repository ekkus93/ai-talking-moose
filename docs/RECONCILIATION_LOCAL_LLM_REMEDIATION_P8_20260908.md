# Local LLM Remediation P8 Reconciliation — 2026-09-08

## Status

**P8 contract/CI/cross-platform integrity work is prepared for exact-head validation on the accepted P7-closure generation.**

This record covers `LLMR-800` through `LLMR-803`. It does not close the authoritative TODO until the P8 exact-head ordinary CI, expected-head merge, and exact post-merge `master` CI are accepted.

Implementation base: `e3533f5cab2c8730683701b11277871e8042c7fb`, the P7 tracker-closeout `master` produced by guarded squash merge of PR #62. Exact post-merge master CI `34287904554` completed successfully on that SHA, including canonical `npm run check:all`.

## LLMR-800 — Generated frontend contract reconciliation

The authoritative Rust exporter already emits representative Local LLM IPC objects for the diagnostics path, including `LocalModelDiagnostics`, `LocalRuntimeDiagnostics`, and `LocalLlmDiagnostics`. The tracked generated contract contains a standalone `LocalRuntimeDiagnostics` representative with `generation_in_progress`, and `src/types/moose.ts` contains the corresponding TypeScript interface field.

P8 adds `scripts/check_local_llm_contract_negative_probes.mjs` and wires it into `check:frontend`. The script creates a temporary miniature checkout containing only the files consumed by the production contract/command checkers. It first requires both production checkers to pass against the real checkout. It then renames `LocalRuntimeDiagnostics.generation_in_progress` only in the temporary generated Rust representative and requires the real `check_frontend_contract_shapes.mjs` gate to fail with both the Rust-only renamed key and the TypeScript-only original key. The temporary directory is removed unconditionally and no tracked file is mutated.

The existing `check:generated-backend-contract` path remains authoritative for Rust-to-generated-JSON drift: it regenerates `src/generated/backendContract.json`, requires a clean diff, and then runs the frontend shape checker. In combination, the drift gate catches a stale generated artifact and the new negative probe proves a regenerated diagnostics-field rename cannot silently agree with stale TypeScript.

## LLMR-801 — Tauri diagnostics command registration and fixtures

`get_local_llm_diagnostics` is already:

- implemented as a production Tauri command;
- present in `tauri::generate_handler!`;
- invoked by `tauriBridge.getLocalLlmDiagnostics()`;
- represented in the frontend test dispatcher and bridge tests.

The new P8 negative-probe script copies `src/lib/tauriBridge.ts` and `src-tauri/src/lib.rs` into its temporary checkout, deliberately renames only the Rust `get_local_llm_diagnostics` registration, and requires the real `check_tauri_command_contract.mjs` gate to fail because the frontend-invoked command is no longer registered. This is diagnostics-specific negative evidence rather than relying only on the older generic command mutation proof.

## LLMR-802 — Model-weight-free ordinary CI

The new P8 proof is text/AST/JSON-only. It copies TypeScript, generated JSON, bridge source, and Rust registration source into a temporary directory; it downloads or loads no GGUF artifact and invokes no Local runtime inference.

Existing ordinary CI retains the Linux/macOS Local LLM compile proofs, both unsigned macOS bundle smoke jobs, dependency/security/release gates, frontend quality, Rust quality, and canonical `npm run check:all`. No P8 implementation requires catalog model weights or live Gemini access.

## LLMR-803 — P12 real-model rerun decision

**Decision: no P12 real-model rerun is required for the remediation as currently implemented.**

Across P0-P8, the changes address installer cancellation/integrity, runtime-use artifact admission verification, diagnostics exposure, immutable settings snapshots, frontend patch-oriented settings writes, truthful cancellation/template documentation, regression matrices, and contract/CI proof. They do not change the pinned model identities, llama.cpp generation backend, tokenizer/model loading semantics after successful admission, chat-template bytes, application-owned SmolLM2/Qwen prompt framing, Qwen non-thinking control semantics, provider routing contract, or decode/generation algorithm.

The P2 runtime-use SHA revalidation can reject a tampered artifact earlier, but an accepted pinned artifact is handed to the same runtime and generation path. That is an integrity/admission change rather than a real-model generation-semantic change. P6/P7/P8 likewise make no prompt-byte/control-token or generation-semantic change.

P9/P10 are documentation/reconciliation/final-audit phases. If later implementation work unexpectedly changes model identity, runtime loading semantics, template rendering, prompt framing, tokenization, or generation behavior, this decision must be reopened and the affected P12 acceptance rerun before final closure. Otherwise the existing accepted P12 real-model evidence remains applicable.

## Required closure evidence

- the new diagnostics field negative probe passes by proving the production shape checker rejects the deliberate temporary rename;
- the new diagnostics command-name negative probe passes by proving the production command checker rejects the deliberate temporary registration rename;
- ordinary exact-head P8 CI passes, including generated-contract drift, frontend/Rust quality, dependency/security/release gates, all Local LLM compile proofs, both macOS bundle smokes, and canonical `npm run check:all`;
- expected-head guarded merge succeeds;
- exact post-merge `master` CI succeeds before the P8 tracker section is closed.
