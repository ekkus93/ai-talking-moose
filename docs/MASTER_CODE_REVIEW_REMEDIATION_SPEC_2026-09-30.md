# AI Talking Moose Master Code Review Remediation Spec

**Date:** 2026-09-30  
**Status:** Draft remediation specification  
**Review baseline:** master at 9fc10b488c47ad52bb9960bc9181dfe7763a6d9c  
**Review source:** comprehensive static and executable review of the 2026-09-30 master snapshot  
**Companion TODO:** docs/MASTER_CODE_REVIEW_REMEDIATION_TODO_2026-09-30.md

## 1. Purpose

This specification defines the remediation required for issues found during the 2026-09-30 comprehensive review of current master.

The repository has strong artifact verification, privacy boundaries, lifecycle generation checks, generated frontend/backend contracts, and broad acceptance infrastructure. The remaining problems are concentrated at subsystem boundaries where concurrency, cancellation, timeout semantics, security policy, and CI truthfulness cross otherwise well-designed components.

This work must fix those boundary defects without weakening existing Wake Word artifact verification, local inference privacy, model/runtime identity pinning, command contract checks, or existing acceptance requirements.

## 2. Goals

The remediation must achieve all of the following:

1. Make Wake listener ownership race-free and explicitly stateful.
2. Make Wake listener startup cancellable and shutdown nonblocking from the async application runtime.
3. Allow conversation stop/cancel to preempt local ASR startup rather than waiting behind the full startup timeout.
4. Make tool execution timeouts actually bound synchronous built-in work.
5. Unify HTTP timeout policy across Moonshine, local LLM, and local TTS artifact installers.
6. Make Wake performance CI truthfully represent production-listener qualification rather than a stale pending-measurement schema.
7. Reduce the Tauri attack surface with a real CSP and least-privilege capabilities.
8. Make failed settings persistence observable to callers after rollback/reconciliation.
9. Make frontend event-listener initialization exception-safe and leak-free.
10. Replace the placeholder settings rollback test with real regression coverage.
11. Reduce architectural risk in the largest lifecycle modules where decomposition directly supports the above fixes.
12. Finish with exact-head and exact-master evidence bound to immutable SHAs.

## 3. Non-goals

- Do not redesign the entire application.
- Do not reopen completed Wake Word corpus, native KWS, artifact identity, packaging architecture, privacy, or first-command-word work unless a remediation change directly touches those paths.
- Do not weaken fail-closed model/runtime verification for convenience or test speed.
- Do not replace deterministic lifecycle tests with sleep-heavy timing tests.
- Do not add hidden network fallbacks for Wake, ASR, TTS, LLM, or tool execution.
- Do not broaden Tauri permissions as a workaround for CSP or capability failures.
- Do not make unrelated UI/visual refactors part of this remediation.
- Do not create a new branch or PR for each checklist item when work is being executed directly on master under explicit user instruction.

## 4. Findings and required design

### 4.1 CI truthfulness: Wake performance evidence is contradictory

Current repository state records WPCR-500 as closed with production-listener performance evidence, while docs/wake-word-performance-evidence.json still records:

- production_listener_measurement_required = true
- production_listener_status = pending_measurement
- accepted_wwr630_is_full_wpcr500_closeout = false

The checker scripts/check_wake_word_performance_evidence.mjs validates this stale pending state, and .github/workflows/wake-word-performance-evidence.yml only validates the JSON report on ubuntu-latest. It does not itself execute or verify a production native-listener measurement.

This means the human closeout record and the machine-readable required gate no longer express the same truth.

#### Required remediation

Create one canonical performance-evidence contract that distinguishes:

1. standalone real-KWS session measurements;
2. deterministic cross-cutting lifecycle measurements;
3. production native-listener measurements;
4. optional comparison measurements such as continuous-ASR idle cost.

The canonical JSON must record the accepted production-listener evidence that actually closed WPCR-500, including exact source SHA, platform, workflow/run/job or artifact identity, measurement path, and required metrics.

The checker must reject:

- a final accepted state whose production-listener status is still pending;
- production-listener evidence without exact SHA/platform/source identity;
- missing required production metrics;
- a claimed closeout that only contains standalone KWS measurements.

The required performance workflow must either:

- execute the authoritative native production-listener measurement on the required native platforms; or
- verify immutable artifacts/reports produced by the authoritative native workflow, including exact SHA and required report schema.

A filename-only required-gates audit is insufficient. The required-gates audit must verify the expected job/command/report contract for performance qualification.

### 4.2 Wake listener lifecycle has a check-spawn-store race

The current native listener slot uses a process-global optional handle and performs a non-atomic sequence:

1. lock and check whether a handle exists;
2. release the lock;
3. create/spawn the listener;
4. reacquire the lock;
5. store the handle.

Concurrent starts can both observe no listener and both spawn. One handle can then overwrite another while the overwritten listener thread continues running.

A concurrent stop can also observe no listener while a start is between the check and store phases, return successfully, and then allow the start to publish a live listener after stop completed.

#### Required remediation

Replace the global optional-handle model with one explicit listener lifecycle controller owned by application state.

Minimum states:

- Stopped
- Starting(generation)
- Running(generation, handle)
- Stopping(generation)

Equivalent naming is acceptable, but the state machine must reserve Starting atomically before expensive construction.

Every start, stop, restart, Settings change, command-ownership transfer, application shutdown, and listener failure transition must flow through this controller.

A start may publish a running handle only if its reserved generation is still current. A stale completed start must immediately dispose of its handle instead of publishing it.

The controller must guarantee:

- no more than one native listener owns Wake capture;
- stop cannot be followed by publication from an older start;
- concurrent start requests coalesce or return a truthful already-starting/running result;
- restart produces one new generation;
- diagnostics can distinguish Stopped, Starting, Running, Stopping, and failure state without inferring physical ownership solely from a logical Wake phase.

### 4.3 Wake startup cancellation and shutdown blocking

WakeLocalListenerHandle::shutdown currently joins a native OS thread synchronously. Listener initialization may perform expensive native KWS/model/runtime construction before the worker reaches its command-processing loop.

A stop issued while native initialization is blocked can therefore synchronously wait for initialization to finish before the join returns.

#### Required remediation

The async application runtime must never perform an unbounded std::thread::JoinHandle::join on a Tokio/Tauri worker path.

Required behavior:

- start receives a cancellation/generation token that can become stale during native initialization;
- initialization checks cancellation at practical phase boundaries;
- stale initialization cannot publish a listener;
- native thread join is moved to a blocking isolation boundary such as spawn_blocking or an equivalent supervisor;
- the public async stop path is bounded by explicit policy;
- timeout does not detach an untracked listener: timed-out shutdown must remain represented in controller state and diagnostics until the thread actually terminates or a terminal failure is recorded;
- application shutdown performs a deterministic final drain.

Tests must use deterministic barriers/channels around initialization and publication rather than relying primarily on wall-clock sleeps.

### 4.4 Conversation stop can be blocked behind local-ASR startup

ConversationManager local-ASR preparation is awaited while operation_lock is held. Production local-ASR worker startup can wait up to its configured startup timeout. stop_session also needs operation_lock.

This means Stop can be forced to wait behind local-ASR startup instead of promptly invalidating the pending operation.

#### Required remediation

Use a reserve-await-commit lifecycle:

1. under operation_lock, reserve a new generation and record provisional startup state;
2. release operation_lock;
3. perform local-ASR initialization;
4. reacquire operation_lock;
5. verify the generation is still current and the session is still eligible;
6. commit the initialized pipeline;
7. if stale, dispose of the initialized pipeline without exposing it.

stop_session must be able to acquire the lifecycle lock promptly, invalidate the generation, and transition toward terminal state while local-ASR initialization is still outstanding.

The same rule applies to any future expensive provider/model preparation added to the locked region.

### 4.5 Tool execution timeouts do not bound synchronous built-ins

The tool router wraps BuiltinTools::execute in tokio::time::timeout. Some built-ins perform synchronous work before yielding.

A Tokio timeout cannot preempt arbitrary blocking work executing inside a single future poll, so a configured timeout can be exceeded by a blocking built-in even though the timeout test for a pending async future passes.

#### Required remediation

Classify built-ins as asynchronous or blocking.

Blocking built-ins must run behind spawn_blocking or an equivalent bounded worker isolation layer. Apply the timeout around the join/future representing that isolated work.

The router must distinguish at least:

- completed success;
- tool error;
- timeout;
- worker panic/cancellation.

Timeout errors must remain sanitized and must not expose secrets or raw provider/tool payloads.

Add a regression test using a deliberately blocking test built-in. The test must prove that a configured short timeout returns before the blocking body finishes and that late completion cannot mutate user-visible router state.

### 4.6 Installer transport policy is inconsistent

Moonshine installer transport configures explicit connect and overall request timeouts. Local LLM and local TTS installer clients enforce redirect/security behavior but do not apply the same explicit timeout policy.

#### Required remediation

Create or reuse one shared secure-download HTTP client policy for model/runtime installers.

The policy must define:

- connect timeout;
- overall request timeout;
- HTTPS/redirect policy;
- bounded redirect count;
- cancellation behavior;
- response/body size enforcement at the existing artifact layer;
- stable user-facing timeout errors that do not leak secrets.

Moonshine, local LLM, and local TTS must all use the same policy unless a documented artifact-specific exception is required.

Do not reduce existing SHA, archive, architecture, staging, or atomic-install verification.

### 4.7 Tauri security configuration is broader than necessary

Current Tauri configuration has CSP disabled and default capabilities scoped to wildcard windows while granting webview/opener capabilities broader than observed application needs.

The frontend also uses dangerouslySetInnerHTML for application-owned sprite SVG. It is not currently fed user/remote HTML, but no-CSP plus broad capabilities increases future blast radius.

#### Required remediation

Define and enforce a minimal CSP compatible with actual application behavior.

Minimum requirements:

- no blanket wildcard source directives;
- scripts limited to application requirements;
- network connect destinations explicitly enumerated or mediated by Rust/Tauri where feasible;
- images/fonts/media limited to required local/data/blob sources only as necessary;
- unsafe directives require an explicit documented justification.

Narrow default capabilities to the actual application window(s), normally main unless another required window is documented.

Remove unused opener and webview-management permissions/plugins where repository usage proves they are unnecessary.

For MooseSprite SVG rendering, prefer a structural/sanitized rendering path. If dangerouslySetInnerHTML is retained, add an explicit invariant/test proving the source is application-controlled and cannot accept remote/user-provided markup.

Add a static security-policy check so CSP cannot silently return to null and wildcard window capabilities cannot reappear without an intentional test/update.

### 4.8 Settings persistence hides backend failure

The settings write queue performs optimistic updates and reconciliation. On backend write failure it reconciles state and then resolves the queued Promise as if the operation completed successfully.

Callers therefore cannot distinguish persisted success from rollback-after-failure.

#### Required remediation

Change the settings persistence contract so callers receive an explicit result.

Acceptable designs include:

- Promise rejection after reconciliation;
- Promise<SettingsWriteResult> with persisted/rolled_back status and sanitized error category.

The store must still reconcile to authoritative backend state before surfacing completion.

UI call sites that initiate settings changes must handle persistence failure without duplicating raw backend errors. They should show a bounded privacy-safe status/toast/message and remain consistent with reconciled state.

Tests must cover:

- successful write;
- failed write with rollback;
- multiple queued writes where an earlier write fails;
- latest optimistic intent after reconciliation;
- Wake-setting failure where listener/runtime state must return to the authoritative persisted configuration.

### 4.9 Frontend event-listener initialization is not exception-safe

Event listeners are registered sequentially. If registration fails after some listeners have succeeded, the function can reject before returning the final aggregate unlisten closure.

Already-registered listeners then remain active and can duplicate on retry/remount.

#### Required remediation

During listener initialization, collect each disposer immediately after successful registration.

If any later registration fails:

1. invoke every disposer already collected;
2. tolerate and aggregate/sanitize cleanup failures;
3. rethrow or return a truthful initialization failure.

Component cleanup must also attach rejection handling to listener initialization so unhandled Promise rejections are avoided.

Add a deterministic test that injects failure at the Nth registration and proves all earlier registrations are disposed exactly once.

### 4.10 Placeholder rollback test provides false confidence

src/stores/mooseStore.settingsRollback.test.ts contains an expect(true).toBe(true) placeholder while its filename implies rollback coverage.

#### Required remediation

Delete the placeholder or replace it with substantive rollback regression tests.

The final suite must not contain pass-by-construction tests whose only purpose is keeping a suite name registered.

The required-gates audit or a lightweight repository policy check should reject known placeholder patterns in production test directories, including trivial expect(true).toBe(true fixtures unless explicitly allowlisted for framework smoke testing.

### 4.11 Architectural decomposition

Several critical modules have accumulated multiple lifecycle responsibilities. The remediation must reduce coupling where it directly supports correctness, but must avoid a broad rewrite.

Required targeted decomposition:

#### Wake lifecycle

Move process-global native listener ownership and its transitions behind one AppState-owned controller module. Separate:

- lifecycle state and generation;
- native thread construction/supervision;
- capture ownership transitions;
- diagnostics projection.

#### Conversation lifecycle

Separate provisional session reservation/commit logic from expensive local-ASR preparation enough that no expensive await requires holding the main lifecycle operation lock.

#### Installer transport

Extract common secure HTTP policy used by Moonshine, local LLM, and local TTS instead of maintaining divergent clients.

#### Tool execution

Separate blocking built-in execution adapter from pure async routing/policy logic.

No decomposition task is complete merely because files became smaller. Completion requires a simpler ownership boundary, fewer independent state sources, or removal of a proven blocking/race hazard.

## 5. Compatibility and invariants

The following existing behavior must remain true:

- Wake detection remains local/offline during idle listening.
- Raw Wake PCM remains bounded and memory-only.
- Wake model/runtime artifacts remain fail-closed and identity verified.
- Local ASR/TTS/LLM artifact hashes and architecture checks remain enforced.
- Settings rollback remains authoritative and cannot leave UI state permanently diverged from backend state.
- One wake event creates at most one command interaction.
- Manual conversation behavior remains available when Wake is disabled.
- Existing command registration/frontend contract remains exact.
- No new silent cloud fallback is introduced.
- Errors remain sanitized for user-visible diagnostics.

## 6. Test and qualification requirements

### 6.1 Focused development tests

During implementation, use the cheapest deterministic tests that prove each local change.

Required focused areas include:

- Wake concurrent start/start;
- Wake start/stop during blocked initialization;
- stale generation completion after stop;
- repeated start/stop/restart with exactly one live listener;
- async stop while native join is delayed;
- conversation stop during blocked local-ASR startup;
- stale local-ASR completion after cancellation;
- blocking built-in timeout;
- late blocking built-in completion;
- installer timeout policy for all three installer families;
- settings persisted success versus rollback failure result;
- queued settings failure/reconciliation ordering;
- partial frontend listener-registration rollback;
- CSP/capability policy checks;
- no placeholder tests.

### 6.2 Ordinary CI

Before final closeout, ordinary CI must pass on the exact qualification head and exact final master.

### 6.3 Wake-specific gates

Because this remediation changes Wake lifecycle ownership, the final exact head must run all Wake gates required by repository policy, including:

- artifact verification;
- deterministic corpus validation and contract;
- native packaging/architecture;
- lifecycle stability;
- production performance evidence;
- privacy audit;
- source-security audit;
- documentation audit;
- required-gates audit;
- real KWS acceptance where changed scope or policy requires it.

Skipped required gates do not count as passing.

### 6.4 Installer qualification

Any shared transport refactor must run focused installer tests plus existing packaging/release metadata/artifact verification tests for Moonshine, local LLM, and local TTS.

### 6.5 Security qualification

Final security evidence must verify:

- CSP is non-null and passes the repository security policy;
- window/capability scope is least privilege;
- unused opener/webview permissions are removed or explicitly justified;
- raw HTML rendering source remains controlled/sanitized;
- no secret-bearing logs were added;
- tool timeout errors remain sanitized.

## 7. Evidence requirements

Every completed TODO section must record:

- exact implementation SHA;
- exact relevant test/workflow run IDs;
- report/artifact names where applicable;
- the requirement proven by each test/gate;
- any intentionally deferred non-goal.

Do not mark an item complete based only on infrastructure existing. The stated runtime behavior must have deterministic evidence.

## 8. Final closeout bar

This remediation is complete only when:

1. every checklist item in docs/MASTER_CODE_REVIEW_REMEDIATION_TODO_2026-09-30.md is reconciled;
2. Wake listener ownership is race-free under deterministic concurrent start/stop tests;
3. Wake initialization cancellation and shutdown do not block the async application runtime;
4. conversation Stop can invalidate pending local-ASR startup without waiting for the full startup timeout;
5. blocking built-in tools obey real timeout boundaries;
6. all model/runtime installers use explicit secure timeout policy;
7. Wake performance machine-readable evidence matches the accepted production-listener closeout;
8. Tauri CSP/capabilities satisfy least-privilege policy;
9. settings write failure is observable after authoritative reconciliation;
10. listener registration failure leaks no handlers;
11. no placeholder rollback test remains;
12. exact-head required gates pass;
13. the exact resulting master SHA passes ordinary CI and every required scope-specific gate;
14. final documentation states the actual support and qualification state without contradictory machine-readable evidence.
