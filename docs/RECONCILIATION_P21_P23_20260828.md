# P21–P23 Reconciliation — Verification Integrity, IPC Fidelity, and Settings Consistency

**Date:** 2026-08-28
**Active plan:** `docs/TODO(20260828-125605).md`
**Planning baseline:** `b9aa3ccaf660bc266aaa4fbe03357991740e266c`
**Status:** **Implementation substantially complete; final verification gate intentionally OPEN**

This audit records the Ralph-loop work for P21–P23 and, critically, attributes each piece of evidence to the environment that produced it. It does not convert missing Rust/current-toolchain evidence into a pass, and it does not substitute CI for physical-macOS/release acceptance owned by earlier phases.

## Environment attribution

### Local sandbox — source/diagnostic environment

The implementation work was performed in the ChatGPT Linux sandbox from the uploaded `master` snapshot corresponding to planning baseline `b9aa3cca`.

Available/reliable here:

- Git/index/source inspection and source-only checks.
- Node 22.x.
- A recovered **stale diagnostic frontend installation** from the previously committed dependency tree: Vitest 2.1.9 / Vite 5.4.21. It was used only to expose frontend test/harness defects and to exercise deterministic store tests. It is **not** acceptance evidence for the reconciled lockfile, which pins Vitest 4.1.11 / Vite 8.2.2.
- Dependency-independent/index checks and contract source checks.

Unavailable/unreliable here:

- A complete current `npm ci`: outbound npm access is unavailable and the local cache is missing at least `zustand@4.5.7`.
- A Rust toolchain for current Rustfmt/Clippy/test/exporter execution.
- Physical macOS/audio/TCC/release acceptance.

After the diagnostic frontend runs were completed, the temporary recovered `node_modules` symlink became unusable; a later repeat failed before Vitest startup because `.bin/vitest` resolved into a removed `/tmp` target. That environment failure does not replace or invalidate the earlier completed diagnostic runs, and generated dependencies were not repaired or committed.

### GitHub CI — authoritative clean-install environment

PR CI is the intended source of current-lockfile clean-checkout evidence because it performs `npm ci` before the frontend/Rust quality gate. The user monitors GitHub Actions; this reconciliation does not poll or presume CI outcomes.

---

## V1R-210 — Untrack generated dependency and build trees

**Implementation:** complete.
**Local implementation commit:** `52d70bd0` (`build: untrack generated frontend trees`).
**Published PR:** #12, head `827b204ad7ad7dae0d45b7eaf609224e47f8b9e8` (`ralph/v1r-210-generated-trees-20260828`).

### Evidence

**Local sandbox:**

- Removed 12,845 tracked `node_modules/` files and 6 tracked `dist/` files.
- `.gitignore` now owns both generated trees.
- `git ls-files node_modules dist` returned no tracked entries after the change.
- Added `check:generated-trees`, which inspects the Git index and therefore runs before dependency installation.
- Force-added probes under both generated trees caused the guard to fail; probes were reverted and the guard returned to success.
- `src-tauri/tauri.conf.json` already runs `npm run build` through `beforeBuildCommand` and packages `../dist`, so removing committed `dist` does not change bundle ownership.

**Accepted CI evidence:**

- GitHub CI run `33214810988` passed on PR #12 head `827b204ad7ad7dae0d45b7eaf609224e47f8b9e8`.
- PR #12 merged to `master` as `efce9ead4e29b8d4768f10b58055715ed94c4c13`.

---

## V1R-211 — Deterministic, non-vacuous Rust privacy log capture

**Implementation:** complete; measured acceptance open.
**Local commit:** `2e2e6324` (`test: make privacy log capture deterministic`).

### Evidence

**Source review/local sandbox:**

- Starts from the already-established `886de2b4f` finding: tracing callsite interest is process-global while the formatter subscriber used by tests is thread-local.
- Consolidates the six privacy log-capture proofs on one serialized shared capture harness.
- Adds an explicit capture-liveness marker so negative privacy assertions cannot be accepted against an empty capture.
- Removes duplicate ad-hoc harness behavior.
- Uses structured/path evidence where available rather than reintroducing the formatted-log positive assertions previously removed as flaky.
- Adds a direct regression that rejects empty capture.
- No privacy assertion was weakened, removed, or wrapped in a retry loop.

**Still required — mandatory by the TODO:**

- Rustfmt/Clippy/current Rust suite in a real Rust environment.
- **At least 20 consecutive full Rust-suite runs on one commit with zero failures**, with the count recorded.
- `npm run check:rust` success across that measured run set.

This row remains open until that measurement exists.

---

## V1R-212 — Frontend tests exercise the native IPC branch

**Implementation:** complete; current-toolchain suite acceptance open.
**Local commit:** `1005a2e9` (`test: exercise frontend Tauri IPC branch`).

### Evidence

**Local sandbox / diagnostic frontend toolchain:**

- `src/test/setup.ts` installs Tauri internals so production-like tests enter the native IPC branch.
- The authoritative dispatcher covers all **37/37** frontend-invoked command names and throws for unhandled commands rather than returning `null`.
- Mocked module `invoke` and the fake Tauri internals delegate to the same dispatcher, closing a dynamic-import path that initially produced `undefined` fixtures in `SettingsModal.test.tsx`.
- A regression asserts that `getSettings()` actually invokes `"get_settings"`.
- Reverting production-path selection was demonstrated to fail the guard, then restored.
- The Settings failures exposed by the change were test-harness routing/isolation defects, not frontend/backend product-contract mismatches.

**Still required:**

- Full frontend suite under the reconciled Vitest 4.1.11/Vite 8.2.2 installation from a clean `npm ci`.

---

## V1R-213 — Rust↔frontend IPC registration and shared-shape gates

**Implementation:** complete; Rust-side mutation acceptance partly open.
**Local commit:** `74aec2aa` (`test: gate Rust frontend IPC contract`).

### Evidence

**Local sandbox:**

- `scripts/check_tauri_command_contract.mjs` currently reports **37/37** frontend-invoked commands registered.
- Five registered backend-only commands are informational: `dismiss_moose`, `get_live_outbound_diagnostics`, `hide_moose`, `show_moose`, `trigger_ambient_remark`.
- The registration check handles qualified Rust registration paths rather than relying on a naive one-line extractor.
- A deliberate command registration/name mismatch made the check fail and was reverted.
- Rust exporter source now emits representative IPC shared-object instances into the generated frontend contract.
- `scripts/check_frontend_contract_shapes.mjs` reports **18/18** Rust-derived representative interfaces matching TypeScript key sets/top-level JSON categories, including the 33-key `AppSettings` surface.
- A deliberate TypeScript `AppSettings.volume` category mismatch made the shape check fail and was reverted.
- No schema/binding framework was added.
- The `as AppSettings` JSON-import cast is explicitly justified by the independent shape gate rather than treated as proof itself.

### Published residual limitations

The lightweight gates intentionally do **not** prove:

- numeric narrowing such as `u32` versus `i32` versus `f64`;
- complete enum variant sets unless represented;
- optionality semantics;
- primitive command arguments;
- the association between a particular command name and its parameter names/types/return type.

The shape gate verifies shared object shapes independently; it is not a generated full command-signature binding layer.

**Still required:**

- With a Rust toolchain, demonstrate that adding/removing/retyping a Rust-side IPC field without updating TypeScript fails the regeneration/shape gate, then revert.

---

## V1R-214 — Generated backend contract drift gate

**Implementation:** complete; Rust/CI mutation acceptance open.
**Local commit:** included in `74aec2aa`.

### Evidence

**Source/local sandbox:**

- Canonical verification and CI regenerate `src/generated/backendContract.json` from Rust and fail on a dirty diff.
- Regeneration is documented in `AGENTS.md`.
- The same generated artifact contains the V1R-213 representative shape data, so drift checking protects defaults/catalogs and shape representatives together.
- No Google credential, live provider, network request at test runtime, microphone, or audio device is required by the contract logic itself.
- The generated JSON is excluded from Prettier because its canonical byte formatting is owned by the Rust exporter and the regeneration/drift gate.

**Still required:**

- Rust-toolchain demonstration: change a Rust default with stale generated JSON → gate fails → regenerate → gate passes.
- Current CI execution of that path.

---

## V1R-215 — Browser preview isolated from production contract coverage

**Implementation:** complete; clean-toolchain preview smoke open.
**Local commit:** `3347c6ac` (`test: isolate browser preview from Tauri IPC`).

### Evidence

**Local sandbox / diagnostic frontend toolchain:**

- Removed the 40 implicit per-function browser fallbacks from the production/native bridge.
- Added explicit `browserPreviewBridge` development-only adapter.
- Inventory records **18 read-only/presentation** operations and **22 simulated external-state/effect** operations.
- Native Tauri IPC wins whenever valid Tauri capability exists.
- Missing/malformed native capability in production-like selection fails closed.
- Query parameters, `localStorage`, and arbitrary runtime browser globals cannot select preview.
- Dedicated preview tests are distinct from production IPC-path tests.
- `README.md` identifies preview as development-only and non-representative of backend/provider success.
- After Settings IPC-path reconciliation, the diagnostic full frontend suite passed **74/74** tests before P23 tests were added.

**Still required:**

- Current clean-toolchain `npm run dev` frontend-only smoke proving the documented preview still renders without Tauri.
- Current clean-toolchain full frontend suite through CI evidence.

---

## V1R-216 — Ordered settings persistence

**Implementation and deterministic source-level acceptance:** complete.
**Local commit:** `2f4c50aa` (`fix: serialize and reconcile settings persistence`).

### Design

Settings persistence now uses a serialized patch coordinator rather than independent full-snapshot writes:

- continuous controls remain immediately optimistic;
- debounced continuous edits coalesce changed fields only;
- discrete and continuous writes enter one ordered persistence queue;
- an older completion never writes old state back into the frontend store;
- a discrete action absorbs a still-pending continuous patch;
- successful candidates are based on the last successfully persisted snapshot.

### Evidence

**Local sandbox / diagnostic Vitest 2.1.9:**

- Deterministic deferred-promise test holds the first discrete backend write unresolved, performs a continuous edit while it is truly in flight, lets the debounce fire, then resolves the first write. The second persisted candidate includes both changes and the final store equals the persisted snapshot.
- Reverse-order test starts with a pending continuous edit and then performs a discrete edit; one folded persistence candidate contains both changes.
- Focused `mooseStore.test.ts`: **16/16 passed** after V1R-216/217 implementation.
- Full diagnostic frontend suite after P23 additions: **78/78 passed** before the recovered temporary toolchain symlink later became unusable.

No claim is made that stale Vitest 2.1.9 is final current-toolchain acceptance.

---

## V1R-217 — Reconcile rejected settings writes

**Implementation and deterministic source-level acceptance:** complete.
**Local commit:** included in `2f4c50aa`.

### Design

On a rejected write:

1. the failed patch is removed from the queue;
2. the store re-reads authoritative persisted settings;
3. if the re-read itself fails, the last known successfully persisted snapshot is the safe fallback;
4. later queued/pending patches are rebuilt on that reconciled baseline;
5. raw backend/provider error detail is not logged to the frontend console.

The same worker handles both discrete and continuous settings writes.

### Evidence

**Local sandbox / diagnostic Vitest 2.1.9:**

- Rejected continuous-write test proves the unpersisted value is removed from the store.
- Rejected discrete-write test proves the same behavior for the discrete path.
- A private sentinel (`SECRET backend failure https://private.invalid/?key=AIzaSyDoNotLog`) is injected into the backend rejection; `console.error`, `console.warn`, and `console.log` receive none of it.
- A subsequent write proves its candidate is rebased on the authoritative reconciled snapshot rather than the failed optimistic state.
- Included in the **16/16** focused store pass and **78/78** diagnostic full frontend pass described above.

---

## V1R-218 — Legacy feature-TODO records correction

**Records work:** complete. This is explicitly **not** product implementation.

### Corrected stale annotations

`docs/TODO(20260818-163801).md` now records current evidence for the designated rows only:

- V1R-060 user-adjustable settings runtime consumers and generated defaults.
- V1R-104 reachable tool-audit diagnostics including duration.
- V1R-111 live microphone-permission refresh behavior.
- V1R-116 semantic accessibility labels.
- V1R-116 reduced-motion behavior.
- V1R-122 production Memory-Off prompt path.

### Corrected misclassifications

- V1R-110 conservative privacy choices are satisfied by defaults-Off plus accurate onboarding disclosure; no new onboarding toggles were invented.
- V1R-103 confirmation is N/A for the V1 tool set because no consequential V1 action tool ships; any future consequential `CharacterAction` reopens the requirement.
- V1R-071 is narrowed to the intended V1 **event-fingerprint** deduplication contract; semantic generated-text similarity suppression is not V1 scope.

### `min_cooldown_seconds` disposition

The existing production rationale is retained rather than converted into a new setting:

- `V1_MIN_AMBIENT_COOLDOWN_SECONDS = 300` is documented in `character/personality.rs` as a fixed V1 anti-annoyance safety floor.
- `character/behavior.rs` and `character/cooldown.rs` consume it.
- No UI represents the floor as adjustable.

Therefore no settings/UI mapping is added.

All unrelated legacy unchecked rows, especially physical-device/macOS/signing/notarization/clean-machine acceptance, remain unchanged.

---

## Current source-check snapshot

Latest dependency-independent checks in the local sandbox after the P23/records work:

```text
Tauri command contract: 37/37 frontend command names are registered.
IPC shape contract: 18 Rust representatives match TypeScript interface keys and JSON categories.
```

`git diff --check` is clean.

Earlier, while the recovered diagnostic frontend installation was intact:

- ESLint passed.
- Prettier passed.
- focused P23 store tests passed 16/16.
- full frontend diagnostic suite passed 78/78.

A later attempt to repeat `npm test` could not start Vitest because the temporary recovered `node_modules/.bin/vitest` symlink pointed to a removed `/tmp` target. This is recorded as an environment limitation, not substituted for current-toolchain evidence.

---

# Final gate status

## Accepted

- V1R-210 — clean-install CI passed and PR #12 is merged.

## Complete at source/records level

- V1R-216
- V1R-217
- V1R-218

## Implemented, but final acceptance remains open pending authoritative environment evidence

- V1R-211 — Rust quality gate plus **20+ consecutive full Rust-suite runs** on one commit.
- V1R-212 — reconciled-toolchain full frontend suite.
- V1R-213 — Rust-side shared-type mutation proof through regeneration/shape gate.
- V1R-214 — Rust-default stale-contract negative/positive regeneration proof and CI execution.
- V1R-215 — clean-toolchain Vite frontend-only preview smoke plus reconciled frontend suite.

Therefore **Gate P21–P23 remains OPEN**. The remaining work is verification evidence, not an identified source implementation defect. This document must be amended with the final PR #13 CI/run identifiers, measured 20-run Rust evidence, and any remaining mutation/smoke evidence before the gate is closed.
