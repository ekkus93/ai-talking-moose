# AI Talking Moose Master Code Review Remediation TODO

**Date:** 2026-09-30  
**Status:** Open  
**Authoritative specification:** docs/MASTER_CODE_REVIEW_REMEDIATION_SPEC_2026-09-30.md  
**Review baseline:** master at 9fc10b488c47ad52bb9960bc9181dfe7763a6d9c  
**Execution model:** work directly on master when explicitly instructed; preserve exact-head compare-and-swap discipline

This is the canonical checklist for the 2026-09-30 comprehensive master code-review remediation. A checkbox may be marked complete only when its stated implementation and acceptance evidence exist. Supporting infrastructure alone is not completion.

## MCR-000 — Freeze baseline and preserve known-good behavior

### Tasks

- [ ] Record exact remediation baseline SHA 9fc10b488c47ad52bb9960bc9181dfe7763a6d9c.
- [ ] Record the review findings and affected source/workflow paths in one baseline evidence document.
- [ ] Confirm ordinary CI state for the baseline.
- [ ] Confirm the existing Wake closeout TODO remains historically closed and is not being rewritten as the new canonical checklist.
- [ ] Confirm the new remediation does not weaken artifact hash verification, runtime identity verification, privacy rules, or generated Tauri command contracts.
- [ ] Identify focused tests/gates required by each MCR section before implementation begins.

### Acceptance

- [ ] Baseline evidence is committed.
- [ ] Existing known-good invariants are explicitly listed.
- [ ] No unrelated cleanup is mixed into the first implementation slice.

## MCR-100 — Repair Wake performance evidence and required-gate truthfulness

### Tasks

- [ ] Reconcile docs/wake-word-performance-evidence.json with the accepted WPCR-500 production-listener evidence.
- [ ] Replace production_listener_status = pending_measurement with a truthful accepted/final state only after exact production-listener evidence is mapped.
- [ ] Record exact production-listener source SHA, platforms, run/job/artifact identities, measurement path, and required metrics.
- [ ] Preserve standalone KWS measurements as a distinct measurement class rather than relabeling them as production-listener measurements.
- [ ] Update scripts/check_wake_word_performance_evidence.mjs to reject a final closeout whose production-listener measurement is missing or pending.
- [ ] Make the checker validate required production metrics and exact evidence identity.
- [ ] Update .github/workflows/wake-word-performance-evidence.yml so the required performance gate executes authoritative native production-listener measurement or verifies immutable authoritative native reports.
- [ ] Update the Wake required-gates audit so it validates the expected performance job/command/report contract, not only workflow filename presence.
- [ ] Add negative tests for stale pending status, missing production metrics, wrong SHA/platform identity, and standalone-only evidence.
- [ ] Update documentation that references the performance gate so the human and machine-readable closeout states agree.

### Acceptance

- [ ] Machine-readable performance evidence and WPCR-500 closeout state are consistent.
- [ ] The performance checker fails if production-listener evidence regresses to pending/missing.
- [ ] The required performance gate proves or verifies the production path, not merely JSON syntax/policy.
- [ ] Focused tests pass.
- [ ] Exact-head performance evidence workflow passes.

## MCR-200 — Replace the global Wake listener slot with an AppState-owned lifecycle controller

### Tasks

- [ ] Introduce one authoritative native Wake listener controller owned by application state.
- [ ] Replace the process-global optional listener handle as the source of lifecycle ownership.
- [ ] Implement explicit Stopped state.
- [ ] Implement explicit Starting(generation) state.
- [ ] Implement explicit Running(generation, handle) state.
- [ ] Implement explicit Stopping(generation) state.
- [ ] Reserve Starting atomically before native construction begins.
- [ ] Coalesce or reject duplicate concurrent starts truthfully.
- [ ] Publish a running handle only when the reserved generation is still current.
- [ ] Dispose of stale completed starts instead of publishing them.
- [ ] Ensure stop invalidates a pending start generation.
- [ ] Ensure restart creates exactly one new generation and exactly one live listener.
- [ ] Route startup, Settings changes, command capture transfer, post-command resume, listener failure, and application shutdown through the controller.
- [ ] Project listener ownership into diagnostics without inferring physical ownership solely from Wake runtime phase.
- [ ] Remove obsolete global-slot helpers after all call sites migrate.

### Deterministic regression tests

- [ ] concurrent start/start cannot create two live listeners.
- [ ] start blocked before publication plus stop cannot resurrect a listener after stop completes.
- [ ] start blocked before publication plus restart publishes only the newest generation.
- [ ] stale generation completion disposes its native handle.
- [ ] repeated start/stop cycles leave no orphan listener and no duplicate capture owner.
- [ ] diagnostics report Starting/Running/Stopping/Stopped truthfully.

### Acceptance

- [ ] All native listener lifecycle ownership flows through one controller.
- [ ] No check-spawn-store TOCTOU path remains.
- [ ] Deterministic concurrency tests pass without relying primarily on sleeps.
- [ ] Wake lifecycle stability gate passes for the exact implementation head.

## MCR-210 — Make Wake initialization cancellable and shutdown nonblocking

### Tasks

- [ ] Add cancellation/generation awareness to native listener initialization.
- [ ] Check cancellation at practical native model/runtime/capture initialization boundaries.
- [ ] Prevent cancelled initialization from publishing a listener.
- [ ] Move native thread join off the async/Tauri worker path using spawn_blocking or an equivalent blocking supervisor.
- [ ] Define a bounded async stop policy.
- [ ] Preserve state/diagnostics if shutdown exceeds the async bound; do not silently forget the native thread.
- [ ] Ensure late native termination reconciles controller state exactly once.
- [ ] Ensure application shutdown performs a deterministic final listener drain.
- [ ] Sanitize initialization/shutdown errors.

### Deterministic regression tests

- [ ] stop during blocked native initialization returns through the async lifecycle path without waiting for the full initialization body.
- [ ] cancelled initialization never publishes Running.
- [ ] delayed join does not block the Tokio/Tauri worker executing the async command.
- [ ] late thread exit reconciles Stopping to Stopped exactly once.
- [ ] repeated cancellation/start cycles do not leak native threads.

### Acceptance

- [ ] No unbounded std thread join runs directly on the async application path.
- [ ] Stale/cancelled native initialization cannot resurrect Wake.
- [ ] Exact-head Wake lifecycle gate passes.

## MCR-300 — Allow conversation cancellation during local-ASR startup

### Tasks

- [ ] Identify every expensive await currently performed while ConversationManager operation_lock is held.
- [ ] Refactor local-ASR startup to reserve-await-commit sequencing.
- [ ] Under the lock, reserve the new session/generation and provisional state.
- [ ] Release the lock before local-ASR model/worker preparation.
- [ ] Reacquire the lock after preparation.
- [ ] Commit the prepared ASR pipeline only if the generation/session is still current.
- [ ] Dispose of a stale prepared pipeline deterministically.
- [ ] Ensure stop_session can acquire the lifecycle lock and invalidate pending startup promptly.
- [ ] Ensure provider/session callbacks from a stale startup cannot revive the cancelled session.
- [ ] Apply the same rule to any adjacent expensive preparation discovered in the locked region.
- [ ] Preserve Wake handoff ordering and first-command-word invariants.

### Deterministic regression tests

- [ ] block local-ASR initialization with a test barrier, call stop_session, and prove Stop invalidates the pending generation without waiting for the production startup timeout.
- [ ] release the blocked initialization and prove stale completion is disposed, not committed.
- [ ] cancellation followed immediately by a fresh start commits only the newest generation.
- [ ] Wake handoff audio from a cancelled startup is not replayed into a later session.
- [ ] normal successful local-ASR startup remains unchanged.

### Acceptance

- [ ] operation_lock is not held across local-ASR startup.
- [ ] Stop is not serialized behind the full local-ASR startup timeout.
- [ ] Conversation lifecycle/unit tests and ordinary CI pass.

## MCR-400 — Make tool timeouts real for blocking built-ins

### Tasks

- [ ] Classify built-in tools by blocking versus async execution.
- [ ] Introduce a blocking execution adapter using spawn_blocking or equivalent bounded worker isolation.
- [ ] Apply router timeout around the isolated blocking operation.
- [ ] Distinguish timeout, tool error, worker panic, and success.
- [ ] Ensure a timed-out blocking tool cannot later mutate router/user-visible state.
- [ ] Preserve tool declaration/policy authorization before execution.
- [ ] Preserve sanitized timeout/error reporting.
- [ ] Review desktop/system and memory built-ins for other hidden blocking calls.
- [ ] Document the timeout contract for built-in tools.

### Regression tests

- [ ] deliberately blocking built-in exceeds a short configured timeout and router returns timeout promptly.
- [ ] blocking body may finish later but cannot produce a second result or mutate completed request state.
- [ ] worker panic becomes a bounded sanitized error.
- [ ] async pending future timeout behavior continues to work.
- [ ] successful blocking built-in still returns normally.

### Acceptance

- [ ] Configured tool timeout bounds both async and synchronous/blocking built-ins.
- [ ] Focused router tests pass.
- [ ] Ordinary CI passes.

## MCR-500 — Unify secure installer HTTP timeout policy

### Tasks

- [ ] Extract or reuse one shared secure installer HTTP client policy.
- [ ] Use the existing Moonshine timeout/security behavior as the baseline unless an exception is documented.
- [ ] Apply explicit connect timeout to local LLM installer transport.
- [ ] Apply explicit overall request timeout to local LLM installer transport.
- [ ] Apply explicit connect timeout to local TTS installer transport.
- [ ] Apply explicit overall request timeout to local TTS installer transport.
- [ ] Preserve HTTPS redirect restrictions and bounded redirect behavior.
- [ ] Preserve installer cancellation semantics.
- [ ] Preserve byte limits, SHA verification, staging, architecture checks, and atomic install behavior.
- [ ] Standardize sanitized timeout error categories/messages across installers.
- [ ] Remove duplicated client-construction policy after migration.

### Tests

- [ ] shared client policy unit tests cover timeout and redirect configuration.
- [ ] Moonshine installer tests still pass.
- [ ] local LLM installer tests still pass.
- [ ] local TTS installer tests still pass.
- [ ] cancellation remains distinguishable from timeout.
- [ ] artifact/package/release metadata validation still passes.

### Acceptance

- [ ] All three installer families use explicit secure timeout policy.
- [ ] No artifact verification behavior is weakened.
- [ ] Focused installer and packaging tests pass.

## MCR-600 — Tighten Tauri CSP and capabilities

### Tasks

- [ ] Inventory actual frontend network, image, font, media, script, IPC, opener, and webview requirements.
- [ ] Replace csp = null with a minimal explicit CSP.
- [ ] Avoid wildcard CSP source directives unless a narrow documented exception is unavoidable.
- [ ] Scope default capabilities to the actual required window set instead of windows = ["*"] where possible.
- [ ] Remove unused opener permissions/plugin if no production call site requires them.
- [ ] Remove unused webview-management permissions.
- [ ] Review every remaining capability and document why it is required.
- [ ] Review MooseSprite dangerouslySetInnerHTML usage.
- [ ] Prefer structural/sanitized SVG rendering if practical.
- [ ] If raw HTML/SVG insertion remains, encode and test the invariant that the source is application-controlled and cannot be populated from remote/user content.
- [ ] Add a static repository security-policy checker that rejects null CSP.
- [ ] Add a policy check that rejects wildcard window capabilities unless explicitly allowlisted.
- [ ] Add a policy check for the retained raw-HTML source invariant or remove raw HTML entirely.

### Acceptance

- [ ] Application launches and required functionality works under the new CSP.
- [ ] CSP is non-null and least privilege.
- [ ] Capability window scope is minimal.
- [ ] Unused opener/webview powers are removed.
- [ ] Security policy tests and source-security audit pass.

## MCR-700 — Make settings persistence failure observable

### Tasks

- [ ] Define a typed settings-write result or rejection contract.
- [ ] Keep optimistic state application.
- [ ] On backend failure, reconcile to authoritative backend state before resolving/rejecting the caller-facing operation.
- [ ] Make the caller distinguish persisted success from rollback-after-failure.
- [ ] Update Settings UI call sites to handle persistence failure.
- [ ] Display a bounded privacy-safe user-facing failure state.
- [ ] Ensure error handling does not overwrite newer queued optimistic intent incorrectly.
- [ ] Ensure Wake settings failure restores listener/runtime state consistent with persisted settings.
- [ ] Document queue semantics for multiple pending settings patches.

### Tests

- [ ] single successful settings write reports success.
- [ ] single failed write reconciles and reports failure.
- [ ] earlier queued failure followed by later intent preserves/reapplies the correct latest intent according to queue policy.
- [ ] multiple sequential failures remain deterministic.
- [ ] Wake enable persistence failure restores authoritative disabled/listener state.
- [ ] Wake disable persistence failure restores authoritative enabled/listener state when appropriate.
- [ ] backend error strings remain sanitized.

### Acceptance

- [ ] No caller can mistake rollback-after-failure for persisted success.
- [ ] UI and backend remain reconciled after failure.
- [ ] Focused frontend/store tests pass.

## MCR-710 — Make frontend event-listener setup exception-safe

### Tasks

- [ ] Refactor event-listener initialization to collect each unlisten disposer immediately after successful registration.
- [ ] On registration failure, invoke all previously collected disposers.
- [ ] Ensure each disposer runs at most once.
- [ ] Aggregate or sanitize cleanup failures without hiding the original initialization failure.
- [ ] Ensure component cleanup handles rejection from listener initialization.
- [ ] Eliminate unhandled Promise rejection paths from loadSettings/initEventListeners startup.
- [ ] Preserve normal successful listener teardown.

### Tests

- [ ] inject failure on the first registration.
- [ ] inject failure on a middle registration and prove all earlier listeners are disposed exactly once.
- [ ] inject failure on the final registration and prove all earlier listeners are disposed.
- [ ] successful initialization returns a disposer that unregisters every listener exactly once.
- [ ] component unmount during pending initialization does not leak handlers.

### Acceptance

- [ ] Partial registration cannot leak listeners.
- [ ] Remount/retry cannot duplicate handlers after a failed initialization.
- [ ] Frontend tests pass.

## MCR-720 — Replace the placeholder settings rollback test

### Tasks

- [ ] Remove src/stores/mooseStore.settingsRollback.test.ts if redundant, or replace it with real rollback regression coverage.
- [ ] Ensure every test in the replacement suite has a falsifiable assertion against production behavior.
- [ ] Add or update repository test-policy tooling to reject known placeholder patterns such as expect(true).toBe(true in production test directories, unless explicitly allowlisted.
- [ ] Search the repository for equivalent placeholder tests and reconcile any additional findings.

### Acceptance

- [ ] No misleading settings rollback placeholder test remains.
- [ ] Rollback behavior is covered by substantive tests.
- [ ] Placeholder-test policy check passes.

## MCR-800 — Targeted architecture decomposition

### Wake lifecycle

- [ ] Separate lifecycle state/generation from native thread construction.
- [ ] Separate native supervision from capture ownership transitions.
- [ ] Separate diagnostics projection from lifecycle mutation.
- [ ] Keep the AppState-owned controller as the only public lifecycle mutation boundary.

### Conversation lifecycle

- [ ] Extract reserve/commit/cancel helpers or equivalent so expensive initialization cannot accidentally creep back under the main operation lock.
- [ ] Centralize stale-generation disposal logic.

### Installer transport

- [ ] Centralize secure HTTP client policy shared by Moonshine/local LLM/local TTS.

### Tool execution

- [ ] Separate blocking execution adapter from router authorization/policy logic.

### Architecture acceptance

- [ ] No decomposition is marked complete solely because line counts decreased.
- [ ] Each decomposition removes a proven race/blocking hazard, duplicate policy, or independent state source.
- [ ] Existing public behavior and tests remain stable.
- [ ] Update architecture documentation where ownership boundaries changed.

## MCR-900 — Final source, concurrency, privacy, and security audit

### Tasks

- [ ] Audit all Wake listener start/stop/restart call sites and confirm they use the controller.
- [ ] Audit for any remaining process-global listener ownership.
- [ ] Audit ConversationManager for expensive awaits under lifecycle operation locks.
- [ ] Audit built-in tools for synchronous work bypassing blocking isolation.
- [ ] Audit all installer Reqwest clients for shared timeout/security policy.
- [ ] Audit Tauri config/capabilities against actual production use.
- [ ] Audit settings writes for swallowed persistence failures.
- [ ] Audit frontend event registration for partial-registration leaks.
- [ ] Audit test directories for placeholder pass-by-construction tests.
- [ ] Re-run privacy/source-security checks after error-path changes.
- [ ] Confirm no raw PCM, credentials, API keys, model URLs with secrets, or provider payloads were added to logs.
- [ ] Record all findings and evidence in one final audit document.

### Acceptance

- [ ] No mandatory audit finding remains open.
- [ ] Audit evidence is bound to an exact SHA.
- [ ] Any deferred non-goal is explicitly documented and is not required for correctness/security closeout.

## MCR-910 — Documentation and checklist reconciliation

### Tasks

- [ ] Update Wake performance documentation to match canonical machine-readable evidence.
- [ ] Update architecture docs for the AppState-owned Wake listener controller.
- [ ] Document conversation reserve-await-commit cancellation behavior.
- [ ] Document blocking-tool timeout semantics.
- [ ] Document shared installer HTTP timeout/security policy.
- [ ] Document Tauri CSP/capability policy.
- [ ] Document settings write failure/result semantics.
- [ ] Reconcile this TODO with exact implementation SHAs and test/run evidence.
- [ ] Do not rewrite historical Wake closeout evidence except where a current document must link to the corrected performance contract.

### Acceptance

- [ ] Documentation matches actual runtime and CI behavior.
- [ ] No contradictory pending-versus-closed performance state remains.
- [ ] This TODO is mechanically complete except final qualification sections.

## MCR-950 — Exact-head final qualification

### Preflight

- [ ] Reload latest master before final qualification.
- [ ] Record exact qualification head SHA.
- [ ] Review diff from baseline/current master and confirm scope is limited to this remediation plus required evidence/docs.
- [ ] Confirm no unrelated ASR/TTS/LLM/settings behavior regression is introduced.

### Required exact-head gates

- [ ] Ordinary CI.
- [ ] Wake Artifact Verification.
- [ ] Wake deterministic corpus validation.
- [ ] Wake corpus contract.
- [ ] Wake native packaging/architecture.
- [ ] Wake lifecycle stability.
- [ ] Wake production performance evidence.
- [ ] Wake privacy audit.
- [ ] Wake source-security audit.
- [ ] Wake documentation audit.
- [ ] Wake required-gates audit.
- [ ] Wake real KWS acceptance when repository policy/scope requires it.
- [ ] Focused conversation cancellation/local-ASR lifecycle tests.
- [ ] Focused blocking-tool timeout tests.
- [ ] Focused Moonshine/local-LLM/local-TTS installer tests and packaging checks.
- [ ] Frontend settings rollback/result tests.
- [ ] Frontend listener-registration cleanup tests.
- [ ] Tauri CSP/capability security-policy tests.
- [ ] Placeholder-test policy check.

### Acceptance

- [ ] Every required exact-head gate passes.
- [ ] No skipped required gate is counted as passing.
- [ ] Evidence is bound to the exact qualification head.
- [ ] Run IDs/artifacts/reports are recorded.

## MCR-960 — Exact-master verification and closeout

### Tasks

- [ ] Re-read current master immediately before final closeout.
- [ ] Verify the qualified head is the exact content being committed/merged according to repository policy.
- [ ] Record exact final master SHA.
- [ ] Verify ordinary CI on exact final master.
- [ ] Verify every Wake-specific gate required by the final changed scope on exact final master.
- [ ] Verify final performance evidence reports the accepted production-listener state.
- [ ] Verify final concurrency regression suites pass.
- [ ] Verify installer transport qualification passes.
- [ ] Verify final security policy checks pass.
- [ ] Verify frontend persistence/listener cleanup suites pass.
- [ ] Verify final source/privacy/security audit is current for the exact final source.
- [ ] Reconcile every checkbox in this TODO with precise evidence.
- [ ] Mark final status closed only after exact-master verification is complete.

### Final acceptance

- [ ] Wake listener ownership is race-free under deterministic concurrent-start/stop tests.
- [ ] Wake startup cancellation cannot publish stale listeners.
- [ ] Wake shutdown does not perform an unbounded join on the async application path.
- [ ] Conversation Stop can invalidate pending local-ASR startup without waiting for the full startup timeout.
- [ ] Blocking built-in tools obey real timeouts.
- [ ] Moonshine/local LLM/local TTS share explicit secure HTTP timeout policy.
- [ ] Wake performance CI and machine-readable evidence truthfully represent production-listener qualification.
- [ ] Tauri CSP/capabilities are least privilege.
- [ ] Settings persistence failure is observable after authoritative reconciliation.
- [ ] Partial frontend event registration leaks no listeners.
- [ ] No placeholder rollback test remains.
- [ ] Architectural ownership boundaries are updated where required by the fixes.
- [ ] Exact-head and exact-master required gates pass.
- [ ] No mandatory finding from the 2026-09-30 comprehensive code review remains open.

## Final status

Open. Close only after MCR-950 and MCR-960 are objectively satisfied and reconciled with exact evidence.
