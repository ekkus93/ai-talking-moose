# Local LLM Post-Review Remediation — P2 Implementation Record — 2026-09-05

## Status

**P2 installer integrity/network/diagnostics implementation is prepared for exact-head CI validation.**

This record covers `LLMR-200` through `LLMR-204` from `docs/TODO(20260905-141500).md`. It builds on the merged P1 cancellation work and does not close later runtime-diagnostics, settings, frontend-write, template, or final-gate tasks.

Implementation base: `6f9aa2dac6f99d69eaa86d8e9ea666173124e7eb` (the exact merged P1 `master` that passed post-merge CI `34010580951`; the P0/P1 closure PR is documentation-only).

## LLMR-200 — HTTPS redirect invariant

The production reqwest client no longer uses a generic redirect-count policy by itself. Every proposed redirect target passes through one policy decision that:

- follows only `https` targets;
- rejects any non-HTTPS target before redirected response bytes can be trusted;
- preserves the existing five-hop bound;
- converts reqwest redirect/send failures into the existing safe `Network` installer error rather than exposing the redirect URL or native error text;
- keeps the catalog's initial HTTPS validation unchanged.

The regression probe exercises the same decision helper used by the production reqwest policy and requires no public Internet.

## LLMR-201 — Chronological installer errors

Installer error state is now an explicit ordered state object:

- each recorded per-model error receives a monotonic sequence;
- descriptors still retrieve the safe error for their own model;
- installer diagnostics select the unresolved error with the greatest sequence rather than relying on `HashMap` iteration order;
- successful install/delete clears only that model's unresolved error.

A deterministic two-model probe records errors in known order and proves diagnostics return the newest unresolved error, then proves clearing it reveals the previous unresolved error.

## LLMR-202 — Runtime-use current-byte verification

The runtime load path now delegates artifact path safety and cryptographic verification to the installer-owned runtime verification path before constructing the llama.cpp model spec.

The runtime verification policy is:

1. ordinary UI/status checks remain marker/shape-only and never hash a large GGUF;
2. first runtime use in a process hashes the current artifact and compares it with the pinned catalog SHA-256;
3. successful verification is cached only in memory;
4. the cache fingerprint contains canonical path, exact byte length, modification metadata, and device/inode plus change-time identity on Unix platforms;
5. a changed fingerprint forces a new full hash;
6. fingerprint/path metadata is read again after hashing and must match the pre-hash fingerprint, otherwise verification fails closed;
7. install replacement and delete operations clear the cache;
8. a verification mismatch returns a Local runtime unsafe-artifact failure before llama.cpp receives the path, with no provider/model fallback introduced.

The small-fixture regression proves:

- an unchanged valid artifact hashes once and then hits the cache;
- a same-size in-place mutation still remains marker/shape-valid;
- the changed file metadata invalidates the cache;
- the mutated artifact is re-hashed and rejected with `Sha256Mismatch`.

Existing runtime concurrency tests that intentionally use sparse catalog-sized fixture files seed only the test cache after proving marker/shape validity; the cryptographic behavior itself is exercised against the production verification function using the three-byte artifact fixture.

## LLMR-203 — Validity terminology

Code and maintenance documentation now distinguish:

- **marker/shape-valid install** — fast filesystem/marker/size checks used for model-list and Settings refreshes;
- **verified for runtime use** — current artifact bytes have matched the pinned SHA-256 under the runtime verification/cache policy.

`LocalModelInstallState::Installed` continues to mean the fast marker/shape-valid state. It does not imply that a fresh SHA-256 was computed during the current UI refresh.

## LLMR-204 — Responsibility reconciliation

P2 avoids a second implementation of runtime path validation in `runtime/manager.rs`. Canonical-root containment, plain-file/size checks, fingerprinting, and current-byte verification are owned by the installer verification path. The runtime manager consumes only the resulting verified canonical path.

Installer concerns remain locally separated by dedicated state/helpers for:

- redirect policy;
- in-flight phase/cancellation state;
- ordered error state;
- marker/shape validity;
- runtime artifact fingerprint/cache verification;
- streaming SHA verification;
- promotion/marker commit;
- deletion/cleanup.

The public Tauri command and frontend contract surfaces are unchanged by P2.

## Validation state

Local dependency-independent checks before push:

- `git diff --check` — pass;
- generated frontend tree hygiene — pass;
- Tauri command registration contract — pass (`43/43`), including the negative rename probe.

This environment has no Rust toolchain, so Rustfmt, Clippy, Rust tests, and the complete canonical repository gate must be supplied by exact-head CI before P2 is marked complete.
