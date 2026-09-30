# MCR-000 — Master Code Review Baseline Evidence

**Date:** 2026-09-30
**Canonical TODO:** `docs/MASTER_CODE_REVIEW_REMEDIATION_TODO_2026-09-30.md`
**Specification:** `docs/MASTER_CODE_REVIEW_REMEDIATION_SPEC_2026-09-30.md`
**Reviewed baseline:** `9fc10b488c47ad52bb9960bc9181dfe7763a6d9c`
**Baseline ordinary CI:** run `36392672193` — passed
**Evidence scope:** freeze the reviewed source state, affected paths, preserved invariants, and required qualification map before production remediation begins

## 1. Baseline identity

The comprehensive 2026-09-30 code review was performed against exact `master` commit:

`9fc10b488c47ad52bb9960bc9181dfe7763a6d9c`

The commit message was `docs(wake): close WPCR post-closeout TODO`.

Ordinary CI for that exact SHA completed successfully in GitHub Actions run `36392672193`.

The remediation specification and TODO were added after the reviewed baseline. They do not change the source findings being remediated.

## 2. Historical Wake closeout remains closed

The new MCR program is intentionally a new canonical checklist rather than a rewrite of prior Wake closeout history.

At current master, both historical documents still explicitly report closed state:

- `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md` — `Status: Closed on master`.
- `docs/WAKE_WORD_V1_POST_CLOSEOUT_REMEDIATION_TODO_2026-09-25.md` — `Status: Closed on master`.

The MCR remediation may correct current machine-readable evidence or current implementation behavior discovered after those closeouts, but it must not falsify the historical evidence or silently reinterpret prior run IDs.

## 3. Review findings and affected paths

### MCR-100 — Wake performance evidence / CI truthfulness

Primary affected paths:

- `docs/wake-word-performance-evidence.json`
- `scripts/check_wake_word_performance_evidence.mjs`
- `.github/workflows/wake-word-performance-evidence.yml`
- Wake required-gates audit script/workflow
- `docs/evidence/WPCR-500_PRODUCTION_LISTENER_PERFORMANCE_2026-09-27.md`
- current Wake performance/CI documentation

Finding frozen at baseline:

The human WPCR-500 closeout records production-listener performance as complete, while the machine-readable performance evidence still requires `production_listener_status = pending_measurement`. The required performance workflow validates the stale JSON contract on `ubuntu-latest` rather than itself proving or immutably verifying the accepted production native-listener evidence.

### MCR-200 / MCR-210 — Native Wake listener lifecycle

Primary affected paths:

- `src-tauri/src/app/wake_word_state.rs`
- `src-tauri/src/app/wake_word_local_listener_thread.rs`
- application-state composition and diagnostics paths that own/project Wake listener state
- Wake lifecycle tests and acceptance workflows

Finding frozen at baseline:

Native listener ownership uses a check-spawn-store pattern around a process-global optional listener handle. The check and publication are not one atomic lifecycle transition. Concurrent start/start or start/stop operations can therefore create an orphaned listener or publish a listener after a stop already returned.

Shutdown also synchronously joins the native thread, while native initialization can occur before the listener reaches its normal command loop.

### MCR-300 — Conversation cancellation during local-ASR startup

Primary affected paths:

- `src-tauri/src/conversation/session.rs`
- `src-tauri/src/asr/pipeline.rs`
- related conversation lifecycle tests

Finding frozen at baseline:

Local-ASR preparation is awaited while the conversation operation lock is held. `stop_session` needs the same lock. A stalled local-ASR startup can therefore serialize Stop behind the production startup timeout rather than allowing prompt generation invalidation.

### MCR-400 — Blocking built-in tool timeout semantics

Primary affected paths:

- `src-tauri/src/tools/router.rs`
- `src-tauri/src/tools/builtin/mod.rs`
- blocking desktop/system/memory built-ins
- tool router tests

Finding frozen at baseline:

The router applies `tokio::time::timeout` around an async function that may perform synchronous blocking work before yielding. Tokio timeout cannot preempt arbitrary synchronous work inside one future poll, so configured tool timeouts do not reliably bound blocking built-ins.

### MCR-500 — Installer HTTP timeout policy

Primary affected paths:

- Moonshine installer transport
- local LLM installer transport
- local TTS installer transport
- shared installer/download policy introduced by the remediation

Finding frozen at baseline:

Moonshine configures explicit connect/request timeout policy while local LLM and local TTS installer clients do not use the same explicit timeout contract.

### MCR-600 — Tauri security configuration

Primary affected paths:

- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/default.json`
- Tauri plugin registration
- `src/components/Moose/MooseSprite.tsx`
- new static security-policy checks

Finding frozen at baseline:

Tauri CSP is null and default capabilities use wildcard window scope with broader opener/webview permissions than observed application requirements. MooseSprite uses application-controlled SVG via `dangerouslySetInnerHTML`; it is not currently a user/remote markup injection path, but it increases blast radius when combined with a null CSP and broad capability policy.

### MCR-700 — Settings persistence result semantics

Primary affected paths:

- `src/stores/mooseStore.ts`
- Settings UI call sites
- settings rollback/reconciliation tests

Finding frozen at baseline:

Settings persistence failure triggers reconciliation but resolves the queued Promise the same way as success. Callers cannot distinguish persisted success from rollback-after-failure.

### MCR-710 — Frontend event listener registration cleanup

Primary affected paths:

- `src/stores/mooseStore.ts`
- `src/windows/MooseWindow.tsx`
- listener initialization/cleanup tests

Finding frozen at baseline:

Sequential event registration can reject after some registrations have succeeded but before the aggregate disposer is returned. The already-registered handlers can leak and later duplicate after retry/remount.

### MCR-720 — Placeholder rollback test

Primary affected path:

- `src/stores/mooseStore.settingsRollback.test.ts`

Finding frozen at baseline:

The named rollback test contains a pass-by-construction `expect(true).toBe(true)` placeholder. Substantive settings tests exist elsewhere, but this file itself provides no falsifiable rollback coverage.

### MCR-800 — Targeted architecture decomposition

Primary affected modules:

- Wake lifecycle ownership/control
- conversation lifecycle reserve/commit/cancel boundary
- installer transport client construction
- tool blocking execution adapter

The objective is not line-count reduction. Decomposition is required only where it removes an independent state source, duplicate policy, blocking boundary, or proven race.

## 4. Known-good invariants that must be preserved

The remediation must preserve all of the following unless a later reviewed specification explicitly replaces an invariant:

1. Wake idle detection remains local/offline.
2. Wake raw PCM remains bounded and memory-only.
3. Wake model/runtime inputs remain identity verified and fail closed.
4. Local ASR/TTS/LLM artifact hash, architecture, staging, and atomic-install checks remain enforced.
5. No hidden cloud/full-ASR fallback is introduced for Wake idle detection.
6. One Wake event creates at most one command interaction.
7. Wake command handoff ordering and first-command-word acceptance remain protected.
8. Manual conversation remains available when Wake is disabled.
9. Settings reconciliation remains authoritative: frontend state cannot permanently diverge from persisted backend state after failure.
10. User-visible/provider/tool errors remain sanitized and must not expose credentials, raw PCM, secret URLs, or provider payloads.
11. Generated frontend/backend Tauri command contracts remain exact and checked by CI.
12. Existing privacy, source-security, documentation, artifact, corpus, packaging, lifecycle, and real-KWS gates are not weakened merely to accelerate remediation.

## 5. Focused test and gate map

| Section | Minimum focused development evidence | Final scope-specific gates |
| --- | --- | --- |
| MCR-100 | performance schema/checker positive and negative tests; required-gates contract tests | Wake performance evidence; required-gates audit; docs audit |
| MCR-200 | deterministic start/start, start/stop, stale-publication, restart tests | lifecycle stability; source-security; ordinary CI |
| MCR-210 | blocked-init cancellation; delayed-join isolation; late-exit reconciliation | lifecycle stability; ordinary CI |
| MCR-300 | blocked local-ASR startup + Stop; stale completion disposal; fresh-generation restart | ordinary CI; relevant Wake handoff/lifecycle gates |
| MCR-400 | deliberately blocking built-in timeout; late completion; panic mapping | ordinary CI; focused tool tests |
| MCR-500 | shared client policy; timeout/cancel distinction; three installer families | ordinary CI; packaging/artifact/release metadata checks |
| MCR-600 | CSP/capability policy tests; production UI smoke/compile; controlled SVG invariant | source-security; privacy where applicable; ordinary CI |
| MCR-700 | success/failure typed result; queued-write reconciliation; Wake settings rollback | frontend/store tests; ordinary CI; Wake lifecycle when touched |
| MCR-710 | failure at first/middle/final registration; exact-once cleanup | frontend tests; ordinary CI |
| MCR-720 | substantive rollback assertions; placeholder-pattern rejection | frontend tests; placeholder-policy check |
| MCR-800 | ownership-boundary tests from the sections above | architecture docs plus all affected final gates |
| MCR-900 | source/concurrency/privacy/security audit bound to exact SHA | privacy; source-security; ordinary CI |
| MCR-910 | documentation reconciliation checks | documentation audit; required-gates audit |
| MCR-950 | all required exact-head gates enumerated in canonical TODO | exact qualification-head matrix |
| MCR-960 | exact-master rerun/verification of required gates | exact final-master matrix |

## 6. Baseline acceptance

MCR-000 is satisfied when this evidence document is committed with the canonical TODO reconciliation.

No production source, workflow behavior, security policy, or acceptance threshold is changed by MCR-000 itself.
