# Phase P10 reconciliation — 2026-08-22

This overlay reconciles the active V1 tracker without rewriting the large monolithic TODO through the
GitHub contents API.

## Accepted closure: V1R-100 through V1R-105

P10 is **accepted complete**.

Acceptance evidence:

- implementation commit: `9b57c213d12e9b151afff596818adbc539ef24a6` (`feat: harden V1 tool policy and routing`)
- Clippy repair commit: `d0d87e4cfbb15c428c2a06a3c89e3f4ed69b15ec` (`fix: satisfy Clippy tool audit membership check`)
- GitHub Actions run: `32621352577`
- workflow: `.github/workflows/ci.yml`
- tested head SHA: `d0d87e4cfbb15c428c2a06a3c89e3f4ed69b15ec`
- result: `completed / success`

### V1R-100 — Provider-neutral tool schema

- [x] Stable provider-neutral declaration carries name, description, and JSON schema.
- [x] Permission, privacy gate, and confirmation policy are explicit local metadata.
- [x] Timeout, maximum input bytes, and maximum output bytes are explicit metadata.
- [x] Gemini conversion continues to emit only function name/description/`parametersJsonSchema`; local policy metadata is not delegated to Gemini.

### V1R-101 — Router policy enforcement

- [x] Registered declaration is resolved before execution; unknown names fail closed.
- [x] Hard input-size bound is enforced.
- [x] Permission/privacy/confirmation policy is enforced locally.
- [x] Arguments are validated against the declared JSON-schema subset before execution.
- [x] Every tool executes under a timeout with a repository-wide hard ceiling.
- [x] Concurrent provider-originated executions are hard-capped; excess calls fail closed rather than waiting unboundedly.
- [x] Hard output-size bound is enforced.
- [x] Failures use stable structured error classes and discard raw backend errors.

### V1R-102 — Read-only safe tools

- [x] V1 read-only allowlist is exactly current time, battery status, and active application.
- [x] Active-app observation is denied at the router before the OS query unless its privacy setting is enabled.
- [x] Battery observer typed availability is retained.
- [x] No generic filesystem or HTTP/network escape hatch is provider-visible.

### V1R-103 — Safe action tools

- [x] The only V1 provider-visible mutation is `remember_fact`.
- [x] Memory mutation requires the user's default-off Memory setting as standing opt-in.
- [x] No provider-visible character/process/system action capability exists in V1.
- [x] The `CharacterAction` permission class requires explicit per-invocation local user confirmation before execution.
- [x] No arbitrary shell, AppleScript, command, or process execution exists.

### V1R-104 — Privacy-safe tool audit

- [x] Audit records contain tool name/fixed unregistered label, result category, timing, permission class, and permission outcome.
- [x] Raw args/results/backend errors are not retained.
- [x] Unknown model-controlled tool names are replaced by the fixed `unregistered` audit/log label.
- [x] Audit retention is bounded to 128 in-memory records and is not persisted in V1.

### V1R-105 — Prohibited capability regression

- [x] Exact V1 provider-visible tool-name allowlist is locked by tests.
- [x] Representative shell/AppleScript/file/HTTP/process tool names are rejected by router tests.
- [x] Built-in declarations are scanned by tests for prohibited generic capability labels.

See `docs/TOOLS_V1_POLICY.md` for the V1 safe-tool contract.

## 2026-08-24 current-master regression audit

A current-master audit after P7/P8/P9/P11/P12 found that the accepted provider-visible allowlist,
privacy gates, schema validation, provider declaration conversion, audit redaction, and model-call
routing are still intact. The only prior change under `src-tauri/src/tools` since accepted P10 was the
P12 logging-privacy regression expansion for active-application names and raw audio.

The audit found one resource-boundary gap not covered by the original P10 checklist: the conversation
event loop spawns a task for each provider tool call, so individually bounded calls could still create
unbounded simultaneous executions under a tool-call flood. The hardening candidate therefore:

- [x] adds a router-owned four-slot execution semaphore;
- [x] uses non-waiting admission so excess calls fail closed immediately;
- [x] returns a stable `concurrency_limit` error/result category;
- [x] records only the normal bounded privacy-safe audit metadata for rejected calls; and
- [x] tests both rejection at capacity and recovery after a slot is released.

This regression hardening remains **pending GitHub Actions acceptance**. The already-accepted V1 tool
capability surface is unchanged.
