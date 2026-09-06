# Local LLM Remediation P3 Reconciliation — 2026-09-06

## Status

**P3 runtime-diagnostics implementation is prepared for exact-head CI validation.**

This record covers `LLMR-300` through `LLMR-303` from `docs/TODO(20260905-141500).md`. It does not mark those tracker items complete before exact-head PR CI, merge, and post-merge `master` validation.

Implementation base: `1f3d9802bd3f0f41d143e2085baa0528bc5cd952`, the exact P2-closure `master` that passed post-merge CI `34020262705`.

## LLMR-300 — Production-reachable runtime diagnostics

`LocalRuntimeManager::diagnostics()` remains the authoritative runtime telemetry source. The previous dead-code annotation is removed because the production `get_local_llm_diagnostics` Tauri command now calls it directly.

The command returns one typed `LocalLlmDiagnostics` response containing:

- installer diagnostics from `LocalModelInstaller::diagnostics()`;
- the selected model's current install phase;
- runtime diagnostics from `LocalRuntimeManager::diagnostics()`.

No parallel hand-maintained runtime state was introduced.

Runtime liveness was also tightened: after a model is successfully loaded and before native generation begins, the authoritative runtime telemetry is updated with the loaded model identity. A diagnostics snapshot taken during an in-flight generation can therefore truthfully report both `loaded = true` and `generation_in_progress = true`.

## LLMR-301 — Rust-derived frontend IPC contract

The existing Rust frontend-contract exporter now emits representative values for:

- `LocalModelDiagnostics`;
- `LocalRuntimeDiagnostics`;
- `LocalLlmDiagnostics`.

The TypeScript contract defines matching typed interfaces and enums. The existing shape gate remains responsible for checking representative Rust object keys and JSON categories against TypeScript.

The production Tauri bridge, browser-preview adapter, and production-like frontend Tauri dispatcher all use the composed diagnostics response rather than the old installer-only shape.

## LLMR-302 — Production diagnostics UI

The Local LLM settings panel polls the production diagnostics command on a bounded one-second interval while mounted and renders a dedicated `Local LLM Diagnostics` surface.

The UI exposes only diagnostic identity/state/metrics:

- selected local model ID;
- selected install phase;
- loaded model ID/revision/quantization;
- loaded/unloaded/generating/runtime phase state;
- thread count;
- context size;
- generation-in-progress state;
- safe installer/runtime error categories;
- last generation duration;
- prompt/output token counts;
- tokens per second.

It does not render prompt text, generated text, memories, transcripts, filesystem paths, native errors, or provider secrets.

## LLMR-303 — Deterministic liveness and privacy proof

`src-tauri/src/ai/local/runtime/diagnostics_tests.rs` adds a barrier-controlled runtime test that proves the transition:

1. unloaded and idle before generation;
2. loaded and generation-in-progress while the fake native engine is blocked;
3. loaded and idle with metrics after generation completes;
4. unloaded again after model deletion.

The same test injects three unmistakable private sentinels:

- prompt sentinel;
- system/private-context sentinel representing memory/transcript-derived context;
- generated-output sentinel.

Positive controls prove the fake engine actually receives the first two values and returns the output sentinel. Serialized diagnostics are then checked during and after generation to prove none of the three payloads are present.

A focused frontend test verifies the Settings diagnostics surface renders runtime identity/state/performance data and safe error categories.

## Validation state

The implementation was authored directly on a fresh branch from exact green P2 `master` after the initial experimental bootstrap workflow proved too brittle for semantic source editing. The failed bootstrap runs are tooling evidence only and are not part of the implementation history.

Before this P3 work is marked complete, the final implementation head must pass:

- Rustfmt;
- focused runtime diagnostics test;
- generated frontend contract regeneration/drift check;
- Tauri command registration contract;
- Rust-to-TypeScript IPC shape gate;
- TypeScript typecheck;
- focused frontend bridge/UI diagnostics tests;
- ordinary exact-head repository CI.

After exact-head CI passes, the implementation PR must be squash-merged with an expected-head guard and the resulting `master` SHA must pass post-merge CI before P3 tracker closure is recorded.
