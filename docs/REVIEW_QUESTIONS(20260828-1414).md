# Follow-up Review Questions and Issues for Claude Code — 2026-08-28 14:14

## Context

I reviewed the amended planning documents and Claude Code's response against the latest uploaded `master` snapshot:

- `docs/SPEC(20260828-125605).md`
- `docs/TODO(20260828-125605).md`
- `docs/REVIEW_RESPONSES(20260828-1303).md`

The reviewed snapshot is at commit `1f8e44e194442aa51fef7db19eb02e8830829e1b` (`docs: amend P21-P23 plan per external review`). No application source changes are requested by this document.

The response resolves the major issues from `REVIEW_QUESTIONS(20260828-1303).md`. In particular, I agree with the decisions to untrack `node_modules/` and `dist/`, retain frontend-only Vite preview through an explicit development adapter, split settings ordering from settings-write rejection reconciliation, correct the V1R-110 and V1R-103 dispositions, narrow V1R-071 to event-fingerprint deduplication, and begin V1R-211 from the already-established `886de2b4f` tracing-flake history.

I have two remaining questions/refinements that I recommend resolving before implementation begins.

---

## 1. V1R-213 — clarify whether the cross-language contract covers only command names or full IPC signatures

### Observation

The amended plan correctly separates frontend path realism (`V1R-212`) from cross-language command registration integrity (`V1R-213`). The current source has no command-name mismatch: all 37 command names invoked by the frontend are registered in Rust.

However, the problem statement for this area is broader than the minimum acceptance mechanism currently described. The SPEC discusses defects such as a command rename, a signature change, or a removed registration escaping frontend verification. A verifier that only extracts command names from `tauriBridge.ts` and compares them with Rust's `generate_handler!` registration catches renames/removals, but it cannot detect an incompatible argument or return-type change when the command name stays the same.

For example, if Rust changes a command from conceptually:

```text
set_settings(settings: AppSettings) -> Result<...>
```

to a different argument or response shape while retaining the string `"set_settings"`, a name-only verifier can remain green. The frontend mock dispatcher and fixtures are also hand-written TypeScript, so they can continue to agree with the frontend bridge while both have drifted from Rust.

### Why this matters

`V1R-212` requires frontend tests to exercise the real `invoke` path, and those tests depend on mocked argument/return fixtures. Without a schema/binding relationship to Rust, that mock contract can still become a parallel implementation even after the command-name registration gate is added.

This would leave part of the verification-integrity defect class unresolved while the documents imply the Rust↔frontend command contract is protected.

### Question / requested decision

Please choose and document one of these two scopes explicitly:

**Option A — registration/name integrity only**

Narrow `V1R-213` and its surrounding SPEC language to state that the gate protects command-name/registration agreement only. Remove or qualify claims that it detects command *signature* drift. Treat argument/return-shape verification as a separate future concern.

**Option B — full IPC contract integrity**

Make `V1R-213` protect command names plus argument and return shapes, using generated bindings/schema or another mechanism with an authoritative Rust-derived contract. Frontend test mocks/fixtures should then be checked against or produced from that same contract.

### Recommendation

I prefer **Option B** if it can be implemented without excessive framework complexity. It closes the same class of parallel-contract drift that motivated the task and makes the phrase "Rust↔frontend command contract" literal rather than name-only.

If the cost is disproportionate for V1, Option A is acceptable, but the acceptance language should be narrowed so we do not claim a stronger invariant than the gate proves.

---

## 2. V1R-215 — prove the browser-preview adapter is unreachable as a production backend

### Observation

I agree with the amended policy that `npm run dev` remains supported and that frontend-only preview behavior should be moved behind a clearly named, explicitly selected development adapter rather than forty implicit per-function `isTauri()` fallback branches.

The preview adapter will intentionally be able to manufacture development-only results for operations that may otherwise depend on Tauri/backend/provider state. Examples already identified in the review include synthetic credential state, connection results, conversation sessions, persistence mutations, model-install state, and text replies.

That makes adapter **selection** a product-safety boundary, not merely a test-architecture detail.

### Risk

If a packaged/Tauri production build can accidentally select or fall through to the preview adapter, the application could present fabricated success as real backend behavior. A runtime flag, query parameter, local-storage value, ordinary browser global, or accidentally shipped environment variable would be particularly risky if it can activate the adapter in production.

The plan currently requires explicit selection and separate preview tests, which is directionally correct, but I do not see an explicit negative acceptance test proving that production cannot use this adapter.

### Question / requested requirement

Can `V1R-215` add an invariant equivalent to:

> A production/Tauri build cannot select, fall through to, or otherwise execute the browser-preview adapter. Development preview behavior is reachable only through an explicitly development-scoped selection mechanism.

The exact implementation can follow the chosen architecture—for example compile-time/environment gating at the application entry point, dependency injection, or a separately imported preview module—but the important property is that the real packaged application fails closed rather than silently obtaining mock behavior.

### Suggested acceptance cases

At minimum, I would like the implementation/tests to establish that:

1. production-like frontend tests use the IPC adapter and fail if they fall back to preview behavior;
2. the preview adapter is exercised only in its dedicated preview tests;
3. a production/Tauri configuration cannot activate the preview adapter through ordinary runtime state;
4. missing or malformed backend capability in production fails closed rather than selecting preview behavior;
5. documentation clearly identifies the frontend-only preview workflow as development-only and non-representative of backend/provider success.

### Recommendation

Add this fail-closed production-isolation property directly to `V1R-215` before implementation. The explicit adapter design is good; the remaining issue is making its non-production reachability a tested invariant rather than an assumption.

---

## Requested disposition

For each item, please record one of:

- **Agree** — amend SPEC/TODO accordingly;
- **Agree with alternative** — explain the alternate invariant/mechanism and amend the documents;
- **Disagree** — cite the current code or authoritative requirement that makes the concern inapplicable.

After these two points are resolved, I do not currently see another planning contradiction that should block P21-P23 implementation.
