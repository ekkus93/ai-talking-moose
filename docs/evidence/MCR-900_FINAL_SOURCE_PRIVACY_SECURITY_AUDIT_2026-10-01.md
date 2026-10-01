# MCR-900 Source, concurrency, privacy, and security audit

**Audit date:** 2026-10-01
**Audited head:** `8cdec484daf01073e5df2f66446564eded8aa860`
**Baseline:** `9fc10b488c47ad52bb9960bc9181dfe7763a6d9c`
**Ordinary CI:** `36828260999` passed on `8cdec484daf01073e5df2f66446564eded8aa860`
**Latest code-bearing ordinary CI:** `36826031723` passed on `4296c35823c8a094632aa7bba7f12dfcfa7b5844`
**Latest Wake source-security audit:** `36826031739` passed on `4296c35823c8a094632aa7bba7f12dfcfa7b5844`

This audit records the current source-level MCR state. It does not close MCR-950 or MCR-960; final exact-head and exact-master gates remain mandatory.

## Scope reviewed

The reviewed remediation delta from baseline through `8cdec484daf01073e5df2f66446564eded8aa860` includes Wake listener lifecycle ownership, conversation/local-ASR cancellation, blocking tool execution isolation, shared installer HTTP timeout policy, Tauri CSP/capabilities tightening, settings persistence rollback/result semantics, frontend event-listener cleanup safety, placeholder-test policy, performance evidence truthfulness, and supporting evidence/docs.

## Audit findings

### Wake listener start/stop/restart ownership

Current Wake listener lifecycle ownership flows through `NativeWakeListenerController` in `src-tauri/src/app/wake_word_state.rs`. The controller owns explicit `Stopped`, `Starting(generation)`, `Running(generation, handle)`, and `Stopping(generation)` states. Startup reserves a generation before native construction, publishing succeeds only for the current generation, stale completed starts are disposed, and stop invalidates pending starts.

The direct-process global listener slot was replaced as the authoritative ownership source. Startup, configured Settings changes, command transfer, post-command resume, listener stop, and application shutdown route through the AppState-owned controller boundary. Diagnostics projection uses `native_wake_listener_lifecycle_phase` rather than inferring listener ownership solely from Wake runtime phase.

No mandatory Wake listener ownership finding remains open at the source level. Final lifecycle qualification remains covered by MCR-950/960.

### Wake initialization cancellation and shutdown

Native listener shutdown is supervised by `supervise_native_wake_listener_shutdown`, which requests shutdown and joins through a dedicated supervisor thread instead of an unbounded join on the async application path. `stop_native_wake_listener_thread` performs a bounded wait and preserves `Stopping` state when the native thread has not exited yet. Late native termination reconciles state through the controller exactly once.

Regression coverage in `src-tauri/src/app/wake_word_state.rs` covers concurrent reservation, stale start disposal, bounded delayed join, restart generation ownership, explicit start/stop cancellation of deferred restart intent, and diagnostics status projection.

No mandatory shutdown/cancellation source finding remains open. Final lifecycle qualification remains pending for final closeout.

### Conversation local-ASR startup cancellation

`src-tauri/src/conversation/session.rs`, `src-tauri/src/conversation/session/local_asr.rs`, and `src-tauri/src/conversation/session/tests.rs` now separate pending local-ASR preparation from final generation/session commit. Stop can invalidate a pending startup while expensive preparation is blocked, and stale completion resolves as cancellation rather than committing or mutating a newer session.

Regression coverage includes blocked local-ASR preparation, stale completion disposal, cancellation followed by fresh start, and Wake handoff from a cancelled startup not mutating a newer committed session.

No mandatory conversation cancellation source finding remains open. Final exact-head ordinary CI and focused lifecycle evidence remain part of MCR-950/960.

### Blocking built-in tool timeout isolation

`src-tauri/src/tools/execution.rs` separates execution mechanics from router authorization. Blocking built-ins are classified, executed through a bounded blocking adapter, and return distinct internal outcomes for success, tool error, worker panic, worker cancellation, and timeout. Router authorization/policy remains in `src-tauri/src/tools/router.rs`, and user-visible errors remain sanitized.

No mandatory blocking-tool source finding remains open. Final exact-head ordinary CI remains part of MCR-950/960.

### Installer HTTP timeout/security policy

`src-tauri/src/installer_http.rs` centralizes secure installer HTTP failure classification and timeout messaging. Moonshine, local LLM, and local TTS installer transports use the shared timeout classification while preserving HTTPS/redirect restrictions, cancellation distinction, byte limits, SHA verification, staging, architecture checks, and atomic install behavior.

No mandatory installer timeout/security source finding remains open. Final installer-focused qualification remains part of MCR-950/960.

### Tauri CSP/capabilities

`src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, package manifests, and `scripts/check_tauri_security_policy.mjs` tighten CSP/capability policy and reject null CSP, wildcard window capabilities without allowlist, unused opener reintroduction, and unsupported raw-HTML/SVG invariants. The unused opener plugin/dependencies were removed. Wake source-security audit `36826031739` passed on the latest code-bearing qualified master head.

No mandatory Tauri source-security finding remains open. Final source-security verification remains part of MCR-950/960.

### Settings persistence rollback/result semantics

`src/stores/mooseStore.ts`, `src/components/Settings/SettingsModalBase.tsx`, `src/test/mooseStore.test.ts`, `src/stores/mooseStore.settingsRollback.test.ts`, and `src-tauri/src/app/wake_word_settings_rollback_tests.rs` now make persistence failure observable through typed success/rollback results, authoritative reload/reconciliation, and bounded user-facing error state. Wake enable/disable rollback coverage verifies listener/runtime state is restored consistently with the persisted authoritative settings.

No mandatory settings persistence finding remains open at the source/test level. Final frontend/backend qualification remains part of MCR-950/960.

### Frontend event registration cleanup

`src/stores/mooseStore.ts`, `src/windows/MooseWindow.tsx`, `src/test/mooseStore.test.ts`, and `src/test/MooseWindow.shortcuts.test.tsx` implement and test idempotent disposer collection, partial-registration cleanup, sanitized startup failure, late disposer cleanup after unmount, and normal successful teardown. Evidence is recorded in `docs/evidence/MCR-710_FRONTEND_EVENT_LISTENER_EXCEPTION_SAFETY_2026-09-30.md` and revalidated against qualified master `4296c35823c8a094632aa7bba7f12dfcfa7b5844`.

No mandatory frontend listener finding remains open at the source/test level. Final frontend qualification remains part of MCR-950/960.

### Placeholder tests

`src/stores/mooseStore.settingsRollback.test.ts` now contains substantive rollback assertions, and `scripts/check_no_placeholder_tests.mjs` rejects known pass-by-construction patterns in production test/spec directories. Evidence is recorded in `docs/evidence/MCR-720_PLACEHOLDER_TEST_REMEDIATION_2026-09-30.md` and revalidated against qualified master `4296c35823c8a094632aa7bba7f12dfcfa7b5844`.

No mandatory placeholder-test finding remains open at the source/test level. Final placeholder policy qualification remains part of MCR-950/960.

### Performance evidence truthfulness

`docs/wake-word-performance-evidence.json`, `scripts/check_wake_word_performance_evidence.mjs`, `scripts/test_check_wake_word_performance_evidence.mjs`, `scripts/check_wake_word_production_listener_report.mjs`, `.github/workflows/wake-word-performance-evidence.yml`, `scripts/check_wake_word_required_gates.mjs`, `docs/wake-word-required-gates.json`, and `docs/WAKE_WORD_V1_CI_GATES.md` now distinguish WWR-630 standalone/composite baselines from WPCR-500 production-listener evidence. Accepted production-listener evidence records exact source SHA, platform, workflow run, job, artifact, report name, runner, timestamp, measurement path, and required report metrics for linux-x86_64 and macos-arm64.

The checker rejects stale pending status in accepted closeout, missing production metrics, non-exact source SHA, mixed source SHA, missing platform, and standalone evidence masquerading as production-listener evidence.

No mandatory performance evidence source/policy finding remains open. The exact-head performance-evidence workflow remains mandatory for MCR-950/960.

## Privacy and log audit

The remediation did not add raw Wake PCM, credentials, API keys, provider payloads, or model URLs with secrets to logs. Wake PCM remains memory-only through the existing Wake runtime/capture design. Error strings added by the remediation are bounded and sanitized, including settings rollback, tool execution timeout/panic, installer timeout classification, Wake listener startup/shutdown, and frontend listener initialization failure.

Wake source-security audit `36826031739` passed on the latest code-bearing qualified master head.

## Remaining non-final gates

The following are not closed by this audit document and remain mandatory for final closeout:

- MCR-950 exact-head full qualification gates, including Wake artifact verification, deterministic corpus, corpus contract, native packaging/architecture, lifecycle stability, production performance evidence, privacy audit, source-security audit, documentation audit, required-gates audit, and any required real-KWS acceptance.
- MCR-960 exact-master verification after the final reconciled head is established.
- Mechanical reconciliation of `docs/MASTER_CODE_REVIEW_REMEDIATION_TODO_2026-09-30.md` with precise final run IDs and evidence.

## Conclusion

No mandatory source-level finding from the 2026-09-30 review remains open in the audited areas above. Final closeout is intentionally deferred until MCR-950 and MCR-960 exact-head/exact-master qualification complete on the eventual final source.