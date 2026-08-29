# P21–P23 Reconciliation — Verification Integrity, IPC Fidelity, and Settings Consistency

**Date:** 2026-08-28
**Active plan:** `docs/TODO(20260828-125605).md`
**Source baseline merged by PR #13:** `c0dbeca02b9392ee6ed76395b95c27fcf254a185`
**Generated-contract formatting repair:** `e361452830a5cd4485ac4cdc6ce90c21c491d223`
**Status:** **All identified P21–P23 implementation/records work is complete; Gate P21–P23 remains OPEN for one post-repair canonical clean-install run.**

This document reconciles the source, trackers, and already-completed acceptance evidence for V1R-210 through V1R-218. It distinguishes source inspection, local diagnostic checks, and GitHub CI evidence. It does **not** convert physical-macOS, device, signing, notarization, or release acceptance from earlier phases into automated evidence.

---

## Evidence environments

### Fresh uploaded `master` snapshot / local sandbox

The reconciliation was repeated against the user-provided `master` archive at `c0dbeca02b9392ee6ed76395b95c27fcf254a185`.

Available locally:

- Git/source/index inspection.
- Node 22.x for dependency-independent checks.
- `bash -n` and `git diff --check`.
- `scripts/check_generated_trees_untracked.mjs`.
- `scripts/check_tauri_command_contract.mjs`.

Unavailable locally for authoritative acceptance:

- Rust/Cargo is not installed in this sandbox.
- The clean archive has no `node_modules/`; an attempted `npm ci` did not complete within the sandbox execution limit, so no local current-toolchain frontend result is claimed.
- Physical macOS/audio/TCC/signing/notarization acceptance remains outside this phase.

After repair `e3614528`, the dependency-independent local checks reported:

```text
Generated frontend trees are absent from the Git index.
Tauri command contract: 37/37 frontend command names are registered.
```

`bash -n scripts/generate_frontend_contract.sh` and `git diff --check` also pass. The TypeScript-backed shape checker is intentionally not claimed locally because the clean snapshot has no completed dependency installation.

### GitHub CI — completed evidence only

No result below is inferred from a pending run.

**PR #13 exact head:** `4eac4fb2e5d1ad0a78246ec38b874ba72a84b285`

P21–P23 acceptance run `33220650275`:

- job `99013788943`, **20 consecutive canonical Rust gates** — **success**;
- job `99013788800`, **Rust contract negative/positive mutations** — **success**;
- job `99013788864`, **Exact-head canonical check:all** — **failure**, later traced to generated-contract formatting ownership rather than product behavior.

**Merged `master`:** `c0dbeca02b9392ee6ed76395b95c27fcf254a185`

CI run `33220660594`:

- Frontend quality — **success**;
- Rust quality — **success**;
- Dependency audit — **success**;
- macOS Tauri bundle (arm64) — **success**;
- macOS Tauri bundle (x86_64) — **success**;
- Release metadata static gate — **failure** only at `Verify generated backend contract is current`.

ASR-015 Native Acceptance run `33220660589` on the same merged `master` commit — **success**.

The one failing CI step regenerated `src/generated/backendContract.json` into a different whitespace/layout form and then failed `git diff --exit-code`; the data itself was not shown to be semantically different. That defect is repaired by `e3614528` and is the only reason the final canonical clean-install gate remains open in this reconciliation.

---

# P21 — Trustworthy verification baseline

## V1R-210 — Untrack generated dependency and build trees

**Status:** accepted.

### Implementation evidence

- Removed the previously tracked `node_modules/` tree (12,845 files) and `dist/` tree (6 files).
- `.gitignore` owns both generated trees.
- The repository guard checks the Git index directly and fails if either generated tree is force-added.
- `AGENTS.md` documents the guard.
- Tauri still owns bundle generation through the normal frontend build; neither macOS bundle architecture depends on a committed `dist/` tree.

### Acceptance evidence

- PR #12 head `827b204ad7ad7dae0d45b7eaf609224e47f8b9e8` passed GitHub CI run `33214810988` after clean dependency installation.
- PR #12 merged as `efce9ead4e29b8d4768f10b58055715ed94c4c13`.
- Current merged-master frontend quality in run `33220660594` again installed from the lockfile and passed typecheck, lint, formatting, frontend tests, preview smoke, and frontend build using the reconciled toolchain.
- The passing current-toolchain frontend suite used **Vitest 4.1.11**.
- Both current-master macOS bundle architecture jobs passed, confirming that untracking `dist/` did not break bundle construction.
- Local force-add negative probes were demonstrated during V1R-210 implementation and reverted.

### Consequent V1R-006 record correction

`docs/TODO(20260818-163801).md` now states explicitly that npm audit jobs run **after `npm ci`**. The obsolete committed Linux-specific `node_modules/` tree — including esbuild 0.21.5, which was absent from the lockfile — therefore was never the tree CI audited. The authoritative dependency state is `package-lock.json` reproduced by `npm ci`, with generated dependency/build trees prohibited from the index.

The V1R-210-local criteria are closed. The separate phase-wide exact `npm run check:all` criterion remains open only because P22 later added the generated-contract gate that exposed the formatting defect fixed by `e3614528`.

---

## V1R-211 — Deterministic, non-vacuous Rust privacy log capture

**Status:** accepted.

### Implementation evidence

- The repair starts from prior commit `886de2b4f`, which documented the process-global tracing callsite-interest/thread-local formatter interaction that made formatted-log positive assertions flaky in parallel tests.
- Privacy log-capture tests use the shared serialized capture harness.
- Negative privacy assertions have explicit capture-liveness proof rather than being allowed to pass against an empty capture.
- The previously removed fragile tool-router formatted-log assertions were not reintroduced.
- No privacy assertion was weakened, deleted, or hidden behind retries.

### Acceptance evidence

Dedicated PR #13 run `33220650275`, job `99013788943` executed the canonical command:

```text
npm run check:rust
```

**20 consecutive times on the same exact PR head**, with **20/20 successful**. This satisfies the mandatory measured-run criterion.

Current merged-master CI run `33220660594` independently passed Rust formatting, Clippy with warnings denied, Rust tests, the backend failure matrix, and the backend stress matrix.

---

# P22 — Test fidelity and contract integrity

## V1R-212 — Frontend tests exercise the native IPC branch

**Status:** accepted.

### Implementation evidence

- Production-like frontend tests install Tauri internals and enter the native IPC branch rather than forty browser fallbacks.
- `invoke` remains mocked at the module boundary, so tests stay offline/hardware-free.
- One authoritative dispatcher covers all **37** frontend-invoked command names and fails on an unhandled command rather than returning `null`.
- A regression asserts that the bridge actually invokes the expected Tauri command.
- Reverting native-path selection was demonstrated to fail the guard during implementation and was restored.
- Hidden Settings harness/routing failures exposed by enabling the path were repaired in the fixtures rather than by restoring fallbacks.

### Acceptance evidence

Current-master CI run `33220660594` passed the complete Frontend quality job after clean `npm ci`, including:

- Tauri command registration contract;
- frontend IPC shape contract;
- typecheck;
- lint;
- formatting;
- the full Vitest 4.1.11 suite;
- frontend-only preview smoke;
- production frontend build.

---

## V1R-213 — Rust↔frontend IPC registration and shared-shape integrity

**Status:** accepted within the explicitly documented lightweight-gate scope.

### Registration integrity

- The source check verifies that all **37/37** frontend-invoked Tauri command names are registered in Rust.
- Five registered-but-not-frontend-invoked commands are informational, not failures: `dismiss_moose`, `get_live_outbound_diagnostics`, `hide_moose`, `show_moose`, and `trigger_ambient_remark`.
- Qualified registrations are handled rather than missed by a naive single-line extractor.
- A deliberate registration/name mismatch was demonstrated to fail and then reverted during implementation.

### Shared-shape integrity

- The Rust exporter emits representative instances of IPC-crossing shared object types.
- The TypeScript checker compares representative Rust-derived JSON object keys/categories against the hand-written interfaces.
- The current gate covers **18/18** representative interfaces, including **33/33** `AppSettings` keys.
- The cast in `backendContract.ts` is no longer treated as proof by itself; the independent shape gate is the protection.

Dedicated PR #13 mutation run `33220650275`, job `99013788800` changed a Rust-side serialized IPC field without updating TypeScript. The shape gate rejected the mutation as required and the mutation was reverted.

### Published residual limitations

The lightweight gate intentionally does **not** claim to prove:

- numeric narrowing such as `u32` versus `i32` versus `f64`;
- complete enum variant sets unless represented;
- optionality semantics in every form;
- primitive command argument shapes;
- a complete generated command-signature binding layer.

Those limitations are documented rather than implied closed.

---

## V1R-214 — Generated backend-contract drift gate

**Status:** implementation and mutation proof complete; post-repair canonical clean-install confirmation pending.

### Implemented protection

- Canonical verification/CI regenerates `src/generated/backendContract.json` from Rust and fails on a dirty diff.
- Regeneration is documented in `AGENTS.md`.
- The generated artifact carries defaults/catalogs plus V1R-213 representative shape data.
- The exporter requires no Google credential, live provider request, microphone, or audio hardware for the contract data.

### Negative/positive mutation proof

Dedicated PR #13 run `33220650275`, job `99013788800`:

1. deliberately changed a Rust `AppSettings` default;
2. verified the stale committed generated contract caused the gate to fail;
3. regenerated/staged the contract;
4. verified the gate passed on that same deliberate mutation.

That proves the intended stale-contract behavior.

### Post-merge formatting defect and repair

Merged-master CI run `33220660594`, job `99013818024` exposed a separate deterministic formatting-ownership bug:

- `export_frontend_contract` already serializes with `serde_json::to_string_pretty`;
- `.prettierignore` states that canonical formatting is owned by the Rust exporter/drift gate;
- `scripts/generate_frontend_contract.sh` nevertheless piped the exporter output through Prettier;
- the committed contract used the exporter-owned representation, so regeneration changed only layout/whitespace and `git diff --exit-code` failed.

Repair commit `e361452830a5cd4485ac4cdc6ce90c21c491d223` removes the redundant Prettier pipe and writes the Rust exporter output directly to the generated artifact. This aligns the script with the already-declared formatting owner and the committed artifact.

A **fresh post-repair exact-head `npm run check:all`** is still required before Gate P21–P23 may be marked closed.

---

## V1R-215 — Browser preview isolated from production contract coverage

**Status:** accepted.

### Implementation evidence

- The forty implicit production bridge fallbacks were replaced by an explicit development-only `browserPreviewBridge` adapter.
- The inventory distinguishes read-only/presentation behavior from operations that simulate external state/effects.
- Valid native Tauri capability always wins.
- Missing or malformed native capability in production-like selection fails closed.
- Query parameters, `localStorage`, and arbitrary runtime browser globals cannot select preview behavior.
- Production-like IPC tests and preview tests are distinct.
- `README.md` labels frontend-only preview as development-only and non-representative of backend/provider success.
- The withdrawn claim that `hasGoogleApiKey()` controlled onboarding was not reintroduced; onboarding is controlled by versioned acknowledgement state.

### Acceptance evidence

Current-master Frontend quality in run `33220660594` passed both:

- the full production-like frontend test suite; and
- `Smoke frontend-only Vite preview`.

Thus the documented `npm run dev`/frontend-only workflow remains viable while production-like tests cannot gain manufactured success from the preview adapter.

---

# P23 — Runtime and record correctness

## V1R-216 — Ordered settings persistence

**Status:** accepted.

Settings persistence uses one serialized patch coordinator:

- continuous controls stay immediately optimistic;
- debounced continuous edits coalesce changed fields;
- discrete and continuous writes share one ordered persistence queue;
- older completions do not overwrite newer frontend state;
- pending continuous changes can be folded into a discrete persistence candidate;
- successful candidates are based on the last successfully persisted snapshot.

Deterministic deferred-promise tests cover both interleavings:

1. discrete write in flight → continuous edit; and
2. pending continuous edit → discrete action.

They assert the final store matches the last successfully persisted snapshot. These tests are part of the current-toolchain frontend suite that passed in CI run `33220660594`.

---

## V1R-217 — Reconcile rejected settings writes

**Status:** accepted.

On a rejected settings write:

1. the failed patch is removed from the queue;
2. authoritative persisted settings are re-read;
3. if the re-read fails, the last known successfully persisted snapshot is used as the safe fallback;
4. later pending/queued patches are rebased on the reconciled baseline;
5. raw backend/provider error details are not logged to the frontend console.

Tests cover continuous and discrete rejection, prove the optimistic unpersisted value does not remain, inject a private sentinel to prove backend error detail is not logged, and verify the next write uses the reconciled base. These tests are included in the current-master frontend suite that passed in run `33220660594`.

---

## V1R-218 — Correct stale/misclassified feature-TODO records

**Status:** complete records correction; not product implementation.

Only the designated stale/misclassified rows in `docs/TODO(20260818-163801).md` were corrected.

### Corrected stale annotations

- **V1R-060:** `volume`, `hide_delay_seconds`, and `tts_model` have production runtime consumers; generated frontend defaults replace the divergent hand-written blob.
- **V1R-104:** the bounded tool audit is reachable through `get_tool_audit`, including `duration_ms`.
- **V1R-111:** microphone-permission status refreshes on focus/visibility changes and avoids a false unavailable first paint.
- **V1R-116:** ASR progress/transcript semantic accessibility roles are present.
- **V1R-116:** reduced-motion preference is observed by `MooseController` and tested.
- **V1R-122:** Memory-Off prompt filtering uses production `model_prompt_memories`, consumed by conversation and ambient prompt construction.

### Corrected misclassifications

- **V1R-110:** conservative privacy choices are satisfied by default-Off active-app observation, memory, and transcript retention plus accurate onboarding disclosure. No onboarding-time privacy toggles are invented.
- **V1R-103:** per-invocation confirmation is **N/A for the shipped V1 tool set** because no consequential confirmation-requiring action tool ships. Any future consequential `CharacterAction` reopens it.
- **V1R-071:** V1 requires event-fingerprint deduplication; semantic generated-text similarity suppression is not V1 scope under the recorded owner decision.

### `min_cooldown_seconds` disposition

`V1_MIN_AMBIENT_COOLDOWN_SECONDS = 300` remains a deliberate fixed V1 anti-annoyance safety floor. `character/personality.rs` now documents that rationale; the value is consumed by behavior/cooldown policy, while talkativeness and hourly budget remain adjustable. No UI represents the safety floor as a user setting.

### V1R-006 consequence

The dependency-advisory row now explains why the old committed dependency tree was not CI audit evidence: npm audit runs after clean `npm ci`, and V1R-210 removed generated dependency/build trees from version control.

Unrelated unchecked legacy rows — including physical-device/macOS/signing/notarization/clean-machine/manual acceptance and other independent product work — remain unchanged.

---

# Gate P21–P23 status

## Satisfied

- [x] V1R-210 and V1R-211 complete.
- [x] V1R-212, V1R-213, V1R-214, and V1R-215 implementation/targeted acceptance complete.
- [x] V1R-216, V1R-217, and V1R-218 complete.
- [x] Rust gate completed **20/20 consecutive** exact-head runs successfully.
- [x] Registration and Rust-side shape/default mutation proofs passed.
- [x] Current-master frontend quality passed on clean `npm ci` / Vitest 4.1.11.
- [x] Current-master Rust quality, dependency audit, both macOS bundle architectures, frontend-only preview smoke, and ASR-015 Native Acceptance passed.
- [x] This reconciliation records the environment/run attribution for each closure.

## Still open

- [ ] **One post-repair canonical clean-install run:** `npm ci` followed by `npm run check:all` must exit 0 on a commit containing `e361452830a5cd4485ac4cdc6ce90c21c491d223` (or its descendant).

The last open item is deliberately not pre-closed. PR #13 proved that targeted green jobs are not a substitute for the canonical repository gate. Once the user reports the post-repair exact-head canonical run green, Gate P21–P23 can be closed without further known source implementation work.
