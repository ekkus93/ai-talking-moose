# Local LLM Post-Review Remediation — P0/P1 Implementation Record — 2026-09-05

## Status

**P0 scope/reopening records are frozen. P1 installer cancellation remediation is complete and validated on merged `master`.**

This record implements the baseline and evidence rules from `docs/SPEC(20260905-141500).md` and tracks the first implementation tranche from `docs/TODO(20260905-141500).md`.

## LLMR-001 — Frozen baseline and scope

The source-review baseline remains exactly:

```text
bb85beb4b61c6a25d7b5935ce4f6918b65afe8d6
```

Implementation started from merged remediation-plan `master` SHA `202fd5c563c613759771d323a937411cebd222c7`; no earlier Local LLM feature branch is an implementation authority.

Scope boundaries remain unchanged:

- text provider scope remains Google or Local;
- Gemini Live remains the voice-session provider;
- Moonshine ASR behavior is not changed by this tranche;
- fully local voice remains deferred;
- signed/notarized P13 release execution remains owner-deferred;
- this remediation does not add local TTS, GPU controls, arbitrary GGUF import, RAG, tool calling, or fine-tuning.

## LLMR-002 — Narrowly reopened historical claims

The post-review findings reopen only these prior claims:

- `LLM-042` — verification-phase cancellation;
- `LLM-045` — installer cancellation semantics and runtime-diagnostics exposure;
- `LLM-054` — production reachability of Local runtime diagnostics;
- `LLM-082` — UI cancellation truthfulness;
- `LLM-114` — cancellation test end-state coverage;
- `LLM-152` — the silent-failure audit's missed acknowledged-but-ignored verification cancellation;
- only the **cancellable** portion of the prior Local LLM Final Gate installer statement.

Previous successful CI, packaging, privacy, model, and real-runtime evidence remains historical evidence. This remediation records a later-discovered gap; it does not rewrite prior runs as failures.

## LLMR-003 — Regression inventory

The complete post-review regression inventory remains authoritative in the remediation TODO. P1 supplies production-path deterministic probes for:

- cancellation during download;
- cancellation after download and before verification;
- cancellation during SHA verification;
- cancellation after verification and before promotion;
- cancellation at the promoted-artifact / marker-commit boundary;
- truthful `verifying` and `promoting` status refreshes;
- successful reinstall after cancellation;
- duplicate cancellation while an operation remains active;
- verification running on a blocking worker rather than the async executor.

The remaining LLMR-003 probes are intentionally left open for their owning later tranches: chronological installer errors, HTTPS downgrade redirects, runtime-use same-size mutation, future settings schema compatibility, request snapshot races, stale frontend writes, and runtime-diagnostics reachability/privacy.

## P1 implementation invariants

The P1 implementation uses these mechanical rules:

1. one authoritative in-flight record owns both cancellation and current phase;
2. phases are explicitly `downloading`, `verifying`, and `promoting`;
3. SHA-256 verification runs through `tokio::task::spawn_blocking` and checks cancellation once per bounded hash chunk;
4. cancellation is checked after verification and before rename;
5. a promoted GGUF is still **not installed** until the validated install marker is durably committed;
6. marker commit is the cancellation linearization point: `cancel()` can return `true` only before that commit becomes non-cancellable;
7. a cancelled post-rename operation removes the pending final artifact and leaves no valid marker/install;
8. staging cleanup applies on success, failure, and cancellation;
9. an explicit later reinstall remains supported.

## Validation state

Local sandbox validation before push:

- `git diff --check` — pass;
- generated frontend tree hygiene — pass;
- Tauri command registration contract — pass (`43/43`);
- Rust formatting/Clippy/tests — unavailable locally because this sandbox does not contain a Rust toolchain;
- TypeScript/ESLint/Prettier/Vitest/build — unavailable locally because npm dependency installation is blocked by sandbox DNS/network access.

P1 closure evidence:

- implementation PR: `#46` — **Local LLM: make installer cancellation truthful**;
- exact implementation head: `3888cf671399a67da70a7c39531e29b37922f70d`;
- exact-head PR CI: run `34009203437` — **pass**;
- merged `master`: `6f9aa2dac6f99d69eaa86d8e9ea666173124e7eb`;
- post-merge `master` CI: run `34010580951` — **pass**;
- passing gates include Rustfmt, Clippy, Rust tests, backend failure/stress matrices, frontend typecheck/lint/format/tests/build, dependency/security audits, macOS bundles, Local LLM compile proofs, and canonical `npm run check:all`.

Accordingly, `LLMR-100` through `LLMR-104` are closed. `LLMR-001` and `LLMR-002` are closed. `LLMR-003` remains intentionally partial: the verification-cancellation, pre-promotion-race, and verifying-status probes are complete, while later-tranche probes remain open under their owning P2+ tasks.
