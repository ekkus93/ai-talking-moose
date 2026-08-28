# Review Questions and Issues for Claude Code — 2026-08-28

## Context

I reviewed the following Claude Code planning documents against the latest uploaded `master` snapshot, without modifying application source code:

- `docs/SPEC(20260828-125605).md`
- `docs/TODO(20260828-125605).md`

The snapshot is at commit `5145524c26bcf9246492e4db4433741a5413749c`. The source tree appears unchanged from the code Claude reviewed; the new SPEC/TODO documents are the meaningful additions since the earlier review baseline.

Overall, the P21–P23 plan is well structured. In particular, it usefully separates measured defects from hypotheses, distinguishes verification-integrity problems from product behavior, and does not equate ordinary CI success with physical acceptance.

Before implementation begins, however, I think the following questions/issues should be resolved so the TODO does not encode contradictory or unachievable acceptance criteria.

---

## 1. Decide whether `node_modules/` and `dist/` should remain tracked

### Observation

The proposed V1R-210/V1R-214 work appears to assume that the repository should continue tracking `node_modules/` and should therefore gain machinery to keep that enormous tree synchronized with `package-lock.json` / Vite.

However, repository guidance already treats the situation as suspicious rather than intentional policy:

- `CLAUDE.md` notes that committed `node_modules/` and `dist/` look like oversights and instructs assistants not to remove them *unprompted*.
- `README.md` documents the normal developer flow as clone followed by `npm ci`.

The current snapshot contains roughly 12,845 tracked `node_modules` files and platform-specific packages such as Linux Tauri/Rollup native packages. That means one committed dependency tree cannot be a clean canonical installation for every supported development host.

### Why this matters

If tracking `node_modules/` is not an explicit product/repository requirement, V1R-210 and V1R-214 may spend significant effort institutionalizing an undesirable repository state by:

- refreshing thousands of generated dependency files;
- building a drift verifier around them; and
- carrying platform-specific dependency artifacts in Git.

The same question applies to stale tracked `dist/`, which is generated output and can drift from current frontend source.

### Recommendation

Prefer:

1. stop tracking `node_modules/`;
2. ensure it is ignored;
3. use `npm ci` as the canonical dependency materialization step;
4. stop tracking stale generated `dist/` unless there is a documented distribution reason to retain it;
5. make CI validate the lockfile/install/build outputs rather than a committed dependency installation.

### Question for Claude

Was preservation of tracked `node_modules/` / `dist/` an intentional project policy in the new SPEC/TODO, or was it simply inherited from the existing repository state because the review avoided broad cleanup?

If it was not intentional policy, should V1R-210/V1R-214 be redesigned around untracking generated dependency/build trees instead of making them self-consistent?

---

## 2. V1R-212's strongest Rust→frontend contract acceptance criterion is not achievable with only an `invoke` mock

### Observation

The TODO wants frontend tests to exercise the Tauri `invoke` path rather than synthetic browser fallbacks. That is useful and should be done.

But the stronger acceptance statement—roughly, “rename a Rust `#[tauri::command]` and a frontend test must fail”—cannot be guaranteed by simply setting `window.__TAURI_INTERNALS__` and mocking `invoke`.

The frontend currently contains hard-coded command strings such as `"get_settings"`. A mocked `invoke` dispatcher in TypeScript would contain another independent hard-coded `"get_settings"`. If Rust is renamed to `get_app_settings`, both TypeScript strings still agree with each other and frontend tests remain green.

I independently checked the current contract surface:

- the frontend bridge invokes approximately 37 command names;
- those names currently match commands registered in Rust's `generate_handler!` set;
- but there is no automated cross-language source of truth enforcing that relationship.

### Why this matters

Without a generated or verified Rust↔TypeScript command contract, V1R-212 can improve frontend test realism while still leaving command-name drift undetectable.

The TODO should not claim an acceptance property the proposed mechanism cannot provide.

### Recommendation

Split the concern into two explicit pieces:

1. **Frontend path realism:** tests run through the Tauri `invoke` branch and no longer pass because browser fallbacks manufacture success.
2. **Cross-language contract integrity:** add either:
   - generated IPC bindings/metadata from the Rust command registry; or
   - a CI verifier that extracts registered Rust command names and compares them against frontend command-name usage / a shared manifest.

### Question for Claude

What mechanism is intended to make a Rust command rename fail CI/frontend verification?

If no generated/shared contract is planned, should V1R-212's acceptance language be narrowed and a separate Rust↔frontend command-contract task be added?

---

## 3. V1R-215's onboarding rationale is stale and conflicts with V1R-217/current code

### Observation

The new SPEC says `hasGoogleApiKey()` gates onboarding and that its browser fallback of `true` can suppress onboarding/privacy explanations.

That does not match the current store logic.

Current `mooseStore.ts` loads API-key status and onboarding status, but `isOnboardingOpen` is determined from `onboardingStatus.needs_acknowledgement`, not from `hasApiKey`.

The same new SPEC later appears to acknowledge the current onboarding behavior correctly while discussing V1R-110, so the document is internally inconsistent.

### Why this matters

The browser fallback of `hasGoogleApiKey() -> true` is still a real fail-open/mock-integrity problem, but its stated consequence should be accurate. Incorrect rationale can lead to the wrong regression test.

Also, the synthetic-success problem is broader than the two examples highlighted in the narrative. Existing browser fallbacks include operations that can fabricate successful external effects, for example:

- ASR model installation appearing installed;
- conversation start returning a fabricated session ID;
- memory deletion returning success;
- text-message send fabricating a response;
- other persistence/provider operations returning plausible success values.

The TODO does call for a broader sweep, which is appropriate.

### Recommendation

Rewrite V1R-215 around the true invariant:

> Production-like frontend tests must not pass because browser fallbacks manufacture backend/provider/persistence success.

Remove the stale claim that `hasGoogleApiKey()` currently suppresses onboarding.

### Question for Claude

Can V1R-215 be corrected to reflect the current onboarding authority and expand its explicit acceptance matrix to all synthetic-success operations found by the sweep?

---

## 4. Define the intended browser-only development behavior before removing/failing 40 fallbacks

### Observation

The repository explicitly supports frontend-only development through `npm run dev`.

At the same time, the frontend bridge currently contains many browser fallbacks because the Tauri backend is absent in ordinary Vite browser mode.

The new plan correctly identifies that those fallbacks undermine production-contract tests, but simply converting every fallback into a thrown error may also destroy a supported developer workflow.

### Why this matters

There are two different requirements being conflated:

- **production/test integrity:** tests should not accidentally exercise fake success instead of real IPC contracts;
- **frontend-preview ergonomics:** developers may still want a browser-only UI preview without launching Tauri.

Those should be intentionally separated rather than making one mode serve both purposes implicitly.

### Recommendation

Define an explicit policy such as:

- production-like tests default to the Tauri/IPC branch;
- browser-preview behavior is an explicitly selected development/mock adapter;
- read-only presentation defaults may be allowed where harmless;
- operations claiming external effects must either:
  - fail closed when no backend exists, or
  - execute only through a clearly named, explicitly enabled mock-development backend;
- browser-preview behavior has its own tests and is never mistaken for production contract coverage.

### Question for Claude

Is browser-only `npm run dev` still intended to be a supported workflow after P21–P23?

If yes, can the SPEC define an explicit preview/mock adapter rather than relying on implicit per-function fallback branches?

---

## 5. V1R-216's stated “cannot diverge from persisted state” end state is broader than its current race fix

### Observation

The settings-write race identified by Claude is real: discrete writes and delayed/continuous writes can interleave and leave persisted state older/different than the store expects.

However, there is another existing divergence path independent of write ordering: the continuous settings-write path catches backend write rejection and leaves the optimistic frontend value unchanged.

Therefore, even after serialization/coalescing fixes, frontend state can still diverge from persisted backend state when a write fails.

### Why this matters

The SPEC/TODO language appears stronger than the proposed acceptance tests. If the intended invariant is literally:

> The store cannot diverge from persisted settings.

then failure recovery must be part of the design, not only successful-write ordering.

### Recommendation

Either:

**A. Narrow the requirement** to successful-write ordering/coalescing, e.g.:

> Successful settings writes are serialized so an older delayed write cannot overwrite a newer persisted state.

or:

**B. Keep the stronger invariant** and add explicit behavior/tests for rejected persistence, such as:

- rollback the optimistic local setting;
- re-read authoritative persisted settings;
- surface a typed persistence error while reconciling the store;
- ensure subsequent writes do not operate on a falsely assumed persisted base.

### Question for Claude

Is V1R-216 intended to fix only the successful-write race, or to guarantee frontend/backend convergence even when persistence fails?

The acceptance criteria should be aligned with that choice.

---

## 6. V1R-217 appears to misclassify V1R-110 onboarding/privacy acceptance

### Observation

The new reconciliation plan proposes leaving part of V1R-110 open because onboarding does not present explicit toggles for memory/transcript/active-app observation.

That interpretation does not appear to match the authoritative product requirement.

The relevant behavior is:

- memory defaults Off, or user explicitly chooses otherwise;
- transcript retention defaults Off, or user explicitly chooses otherwise;
- active-app observation defaults Off;
- onboarding must accurately explain the privacy posture.

Current Rust defaults all three privacy-sensitive features Off, and current onboarding copy explicitly says they start Off and can be enabled later in Privacy Settings.

That satisfies the stated default/privacy requirement without requiring three onboarding toggles.

### Why this matters

Adding onboarding controls merely to satisfy a misread checkbox would create new UI/product scope without a demonstrated product requirement.

V1R-217 is supposed to repair stale tracker state; it should not manufacture new work from an already-satisfied requirement.

### Recommendation

Treat the V1R-110 criterion as complete if the authoritative SPEC confirms that Off-by-default plus accurate disclosure is sufficient.

Do not add onboarding privacy toggles unless product design independently wants them.

### Question for Claude

Can you re-evaluate V1R-110 against the authoritative product wording and current onboarding copy/defaults?

If you still believe it is incomplete, please identify the exact authoritative requirement that mandates an onboarding-time choice rather than Off-by-default with later settings control.

---

## 7. V1R-103 should likely be N/A/satisfied rather than left open

### Observation

`docs/TOOLS_V1_POLICY.md` states that V1 has no consequential `CharacterAction` tool requiring confirmation. The confirmation-required membership set is therefore empty, and the policy says a future consequential action must reopen the requirement.

The old tracker criterion is effectively:

> User confirmation where an action is consequential.

With no consequential V1 action, there is nothing currently requiring confirmation.

### Why this matters

Leaving V1R-103 unchecked makes the feature tracker look like there is an unresolved V1 implementation defect when the requirement is currently vacuously satisfied / not applicable by policy.

This is exactly the kind of stale-status ambiguity V1R-217 is meant to eliminate.

### Recommendation

Mark V1R-103 as **N/A / satisfied for the V1 tool set**, with an explicit note that adding any future consequential tool reopens the confirmation requirement.

### Question for Claude

Do you agree that V1R-103 should be reconciled as N/A/satisfied rather than intentionally left as an open V1 item?

If not, what current V1 action requires user confirmation and is missing it?

---

## 8. V1R-071 is a genuine unresolved requirement unless we intentionally narrow it

### Observation

Claude's review correctly found that the current ambient system prevents duplicate event fingerprints but does **not** perform semantic near-duplicate suppression on generated remark text.

The legacy V1R-071 wording appears to require near-duplicate remark suppression, not merely duplicate source-event suppression.

### Why this matters

This differs from many stale unchecked rows: it appears to be a real source-level requirement that is not implemented as literally written.

If we say “there are no remaining source implementation tasks” while leaving this requirement intact and unmet, the reconciliation is inconsistent.

### Recommendation

Make an explicit product decision:

**Option A — keep the requirement:**
Add V1R-071 semantic near-duplicate remark suppression as a real implementation task with deterministic tests.

**Option B — narrow the requirement:**
If event-fingerprint deduplication is the intended V1 behavior and semantic-text deduplication is unnecessary/undesirable, revise the requirement accordingly and close it with current evidence.

Do not simply leave the row unchecked indefinitely.

### Question for Claude

Does the authoritative V1 product behavior actually require semantic near-duplicate suppression of ambient remark text?

If yes, please add the missing implementation task to the new TODO. If no, please propose the precise requirement/reconciliation wording that closes V1R-071 based on event deduplication.

---

## 9. V1R-211 investigation should begin from the prior tracing-flake incident

### Observation

There is important repository history relevant to the V1R-211 intermittent tracing/log assertion issue.

Commit:

`886de2b4f` — `test: remove flaky tracing field assertions`

previously removed positive tracing assertions from the tool-router privacy test because tracing callsite interest is process-global while the formatter subscriber used by the test is thread-local. Parallel tests could therefore suppress/alter whether that callsite appeared in the captured output.

This is directly related to the class of failure now being investigated.

### Why this matters

The new SPEC correctly says to investigate the mechanism rather than assume the cause. However, this prior failure mode should be treated as starting evidence, not rediscovered from scratch.

It may indicate that the right repair is architectural test isolation/subscriber setup rather than retries, sleeps, or weaker assertions.

### Recommendation

Add this historical commit/rationale to the V1R-211 investigation notes and test hypotheses explicitly against it.

Potential avenues include:

- ensuring a deterministic per-test dispatch/subscriber is used via `tracing::dispatcher::with_default` / scoped dispatch where appropriate;
- avoiding assertions that depend on globally cached callsite interest interacting with parallel subscribers;
- serializing only the minimum affected tracing-capture tests if a scoped dispatcher cannot fully isolate the behavior;
- preserving strong privacy assertions rather than deleting them merely to remove flakiness.

### Question for Claude

Can V1R-211 explicitly incorporate commit `886de2b4f` and its known process-global callsite-interest/thread-local-subscriber interaction into the root-cause investigation?

---

# Proposed decisions to settle before implementation

I think P21–P23 implementation will be much cleaner if the following three policy decisions are made first:

1. **Generated dependency/build trees:** should `node_modules/` and `dist/` be removed from Git and regenerated normally, or does the project intentionally require them to remain tracked?
2. **Browser-only frontend development:** should `npm run dev` remain a supported browser-preview mode? If yes, should it use an explicit mock/preview adapter rather than implicit synthetic-success fallbacks?
3. **Ambient semantic deduplication:** does V1 truly require semantic near-duplicate suppression of generated ambient remark text, or is source-event fingerprint deduplication the intended requirement?

My recommended answers are:

- untrack `node_modules/` and stale `dist/`;
- preserve browser-only preview through an explicit development/mock adapter, while production-like tests exercise real IPC contracts and external-effect operations fail closed by default;
- make an explicit V1R-071 product decision rather than leaving the requirement in a permanently ambiguous state.

---

# Summary of requested Claude Code response

Please review each numbered item and, for each one, indicate one of:

- **Agree — update SPEC/TODO**
- **Agree in principle — alternative implementation/disposition proposed**
- **Disagree — current plan is intentional**, with the code/spec evidence supporting that decision

No application source change is requested by this document. The goal is to resolve planning/acceptance ambiguity before implementing P21–P23.
