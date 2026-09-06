# Local LLM Post-Review Remediation — P2 Implementation Record — 2026-09-05

## Status

**P2 installer integrity/network/diagnostics remediation is complete and validated on merged `master`.**

This record covers `LLMR-200` through `LLMR-204` from `docs/TODO(20260905-141500).md`. It builds on the merged P1 cancellation work and does not close later runtime-diagnostics, settings, frontend-write, template, or final-gate tasks.

Final P2 implementation base: `43881dca61fac165cd8472029d63d3c69d67e8df` (merged P0/P1 closure `master`).

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

## Validation and closure evidence

Dependency-independent checks before the implementation PR:

- `git diff --check` — pass;
- generated frontend tree hygiene — pass;
- Tauri command registration contract — pass (`43/43`), including the negative rename probe.

CI and merge evidence:

- pre-rebase P2 head `8dbbd957ae6a3d2e00fec5e786894d507d926b46` — CI `34013368816` passed after rerunning one transient macOS arm64 runner I/O failure on the same SHA;
- merged P0/P1 closure base `43881dca61fac165cd8472029d63d3c69d67e8df`;
- final P2 PR head `9a6ea2a6d090450c87ac01589aa4493caef8e617` — exact-head CI `34015871421` passed;
- PR #48 `Local LLM: harden installer integrity before runtime use` — squash merged with the expected-head guard;
- merged P2 `master` SHA `51ad339b8c5faf129b5f06a03da153b657988ae9`;
- post-merge `master` CI `34017283156` — passed on that exact SHA, including Frontend quality, Rust quality, dependency/security audits, release/static gate, Linux/macOS Local LLM compile proofs, both macOS unsigned bundle smoke jobs, and canonical `npm run check:all`.

## Tracker closure

The evidence above closes the implementation and acceptance requirements for:

- `LLMR-200` — HTTPS redirect enforcement;
- `LLMR-201` — chronological installer `last_error`;
- `LLMR-202` — runtime-use current-byte reverification and cache invalidation;
- `LLMR-203` — install-validity terminology;
- `LLMR-204` — installer/runtime responsibility reconciliation.

It also closes the P2-owned `LLMR-003` regression probes:

- chronological last-error probe;
- HTTPS downgrade redirect probe;
- same-size post-install mutation probe.

The remaining `LLMR-003` probes stay open for their owning later phases: future-settings-version destructive downgrade and split-settings-snapshot race (P4), stale frontend full-object overwrite (P5), and runtime-diagnostics production reachability/privacy sentinel (P3). Therefore `LLMR-003` as a whole is not yet complete.
