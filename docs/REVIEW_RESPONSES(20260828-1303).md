# Responses to Review Questions — 2026-08-28 13:03

**Reviewing document:** `REVIEW_QUESTIONS(20260828-1303).md`
**Documents under review:** `SPEC(20260828-125605).md`, `TODO(20260828-125605).md`
**Snapshot reviewed:** `5145524c26bcf9246492e4db4433741a5413749c`
**Disposition:** Agree ×7, Agree in principle with alternative disposition ×2. No item disputed.

Every checkable claim in the review was re-verified against source before responding. All held. Three items identified genuine errors in the documents as published, one of which would have caused the plan to undo a prior fix.

No application source was changed. Both planning documents were amended.

---

## Verification performed before responding

| Review claim | Verified | Evidence |
| --- | --- | --- |
| `dist/` is tracked and stale | Yes | `git ls-files dist` → 6 files |
| Tracked tree carries host-specific packages | Yes | `@esbuild/linux-x64`, `@rollup/rollup-linux-x64-gnu`, `@tauri-apps/cli-linux-x64-gnu`, `@napi-rs/lzma-linux-x64-gnu` |
| README documents `npm ci` as the normal flow | Yes | `README.md:112-115` |
| Bridge invokes ~37 command names, all registered | Yes | 37 distinct names; all 37 in `generate_handler!` |
| `isOnboardingOpen` is not driven by `hasApiKey` | Yes | `mooseStore.ts:273` uses `needs_acknowledgement`; `hasApiKey` set at `:269` |
| Continuous write swallows rejection | Yes | `mooseStore.ts:40-43`, `.catch(() => {})` over the optimistic `set` at `:285` |
| `npm run dev` is a documented workflow | Yes | `README.md:129-133`, "For frontend-only development with Vite" |
| Onboarding copy discloses Off-by-default | Yes | `OnboardingModal.tsx:154-156` |
| `TOOLS_V1_POLICY.md` makes V1 confirmation vacuous | Yes | `:53-61`, "empty in V1", primitives "not an implemented end-to-end V1 confirmation feature" |
| Commit `886de2b4f` exists and is on point | Yes | Removed two log-text assertions from `tools/router/tests.rs`; rationale recorded in the replacing code |

One review claim was **checked and found more favorable than stated**: the frontend/Rust command surface has no current mismatch. An initial extraction here suggested four unregistered commands, but that was an artifact of a line-based regex missing the four registered via qualified paths (`commands::character::set_mute`, `commands::character::cancel_standalone_speech`, `commands::conversation::start_conversation`, `commands::conversation::barge_in`). Corrected extraction confirms all 37 match.

---

## 1. Tracked `node_modules/` and `dist/` — **Agree**

**Was it intentional policy?** No. It was inherited. `SPEC` §4.1 scoped the question out explicitly, citing `CLAUDE.md`'s instruction not to change the arrangement unprompted — which made the plan preserve a state nobody had chosen. The review is the prompt that instruction anticipated.

The supporting evidence is decisive, and stronger than the plan acknowledged: the tracked tree contains Linux-only native packages, so it was never a valid installation for the macOS hosts this product targets. It could not have been serving its apparent purpose.

**Decision taken by the project owner on 2026-08-28:** untrack both. Recorded as spec §3.1 decision 1.

**Changes:**
- `V1R-210` is retitled "Untrack generated dependency and build trees" and now removes `node_modules/` (12,845 files) and `dist/` (6 files), ignores both, and relies on `npm ci`.
- Spec §4.2's tracked-tree drift verifier is **void** — there is no tree to verify. What survives is a narrower obligation, folded into `V1R-210`: a check that fails if either directory is reintroduced to the index, which must not depend on the tree being present or on `npm ci` having run.
- The old `V1R-214` (divergence verifier) is dissolved; subsequent IDs shifted.
- Added a row confirming `dist/` removal does not change macOS bundle contents, which remain owned by P13.

---

## 2. `V1R-212`'s command-rename criterion — **Agree**

Correct, and this was a real error. Setting `__TAURI_INTERNALS__` and mocking `invoke` cannot make a Rust rename fail. `tauriBridge.ts` holds command names as string literals; the mock dispatcher holds its own independent literals. Rename `get_settings` to `get_app_settings` in Rust and both TypeScript copies still agree — suite stays green. The document claimed an acceptance property its own mechanism could not provide.

**Changes:**
- `V1R-212`'s acceptance is narrowed to path realism: tests run the `invoke` branch, and a test fails if the suite reverts to the fallback.
- New **`V1R-213` — Verify the Rust↔frontend command contract** (spec §6.2) carries the cross-language property, satisfied by generated bindings or a CI verifier.
- `V1R-213` records that no defect exists today (37/37 matched), which is why now is the right moment to add the gate.
- It also records the qualified-path registration detail, so an implementer does not repeat the extraction error made here.
- Registered-but-uninvoked commands (`dismiss_moose`, `show_moose`, `hide_moose`, `trigger_ambient_remark`, `get_live_outbound_diagnostics`) are reported informationally, not as failures — backend-initiated paths are legitimate.

---

## 3. `V1R-215`'s stale onboarding rationale — **Agree**

Correct, and the document contradicted itself. §6.3 asserted `hasGoogleApiKey()` gates onboarding while §9 correctly recorded that the completion flag now drives it. The false half was carried forward from the 2026-08-24 annotation at `TODO(20260818-163801).md:997` without being re-checked against the very repair that annotation's own row describes.

Verified: `mooseStore.ts:273` sets `isOnboardingOpen` from `onboardingStatus.needs_acknowledgement`. `hasApiKey` is stored at `:269` and gates nothing.

The fail-open mock remains a real integrity problem; only the stated consequence was wrong. The review is also right that the surface is wider than two examples.

**Changes:**
- The claim is withdrawn, with an explicit instruction not to reintroduce it.
- The requirement is rebuilt around the true invariant: production-like tests must not pass because fallbacks manufacture backend/provider/persistence success.
- An acceptance matrix replaces the two examples, covering the verified set — `hasGoogleApiKey` `:249`, `testAiConnection` `:255-257`, `startConversation` `:343` (`"mock-sess-123"`), `deleteMemory` `:379`, `sendTextMessage` `:397` — with a required inventory sweep of all forty branches rather than an assumed-complete list.

---

## 4. Browser-only development policy — **Agree**

Yes, `npm run dev` remains supported; `README.md:129-133` documents it as frontend-only development with Vite. The original plan risked destroying it, having conflated test integrity with preview ergonomics.

**Changes:**
- Spec §6.4 replaces the narrow fail-open section and defines the split: production-like tests take the IPC branch; preview runs through a clearly named, explicitly selected development adapter, not implicit per-function branching on ambient global state.
- Read-only presentation defaults may stay where harmless. Operations claiming an external effect — credential state, connectivity, persistence mutation, conversation lifecycle, model installation — fail closed without the adapter.
- Preview behavior gets its own tests and is never counted as production contract coverage.
- `V1R-215` requires the selection mechanism be documented in `README.md` so the workflow stays discoverable.

---

## 5. `V1R-216`'s divergence invariant — **Agree in principle; both halves kept, split apart**

The observation is correct and verified: `mooseStore.ts:40-43` catches the rejection and leaves the optimistic value from `:285` in place, so the store can diverge from persisted state independently of write ordering.

Rather than choosing between the review's options A and B, both are kept as separate requirements. Narrowing alone (A) would silently drop a real defect the review itself identified; keeping the broad invariant on one row (B) would let a failure-handling design block a well-understood ordering fix.

**Changes:**
- `V1R-216` narrows to ordering: successful writes are ordered so an older delayed write cannot overwrite newer state, and a continuous edit during an in-flight discrete write is not discarded. Acceptance now also covers the reverse interleaving.
- New **`V1R-217` — Reconcile the store when a settings write is rejected** (spec §7.2) carries the failure path: rollback, authoritative re-read, or a typed non-sensitive error while the store reconciles; subsequent writes must not compute from a falsely assumed base.
- `V1R-217` explicitly preserves the existing correct behavior of keeping provider error detail out of the frontend console. The defect is discarding the *fact* of failure, not the suppression of detail.

---

## 6. `V1R-110` onboarding classification — **Agree**

Re-evaluated as asked. The review is right and this document was manufacturing scope.

The criterion reads "Conservative memory/transcript/active-app **choices**." Read against the surrounding `V1R-014` rows — which establish defaults-Off and are already checked — "choices" means *the choices the product makes are conservative*, not *the user is presented choices at onboarding*. Verified: all three default Off in Rust, and `OnboardingModal.tsx:154-156` states "Conservative defaults: active-app observation, cross-conversation memory, and transcript retention start Off. You can enable them later from Privacy Settings."

I searched `SPEC(20260818-163800).md` for any requirement mandating an onboarding-time choice and found none. Absent one, adding three toggles would be new UI scope invented from a misreading — exactly what `V1R-218` is supposed to prevent.

**Changes:**
- Spec §9.2 records the row as **satisfied**, with the instruction not to add onboarding privacy toggles.
- `V1R-218` directs checking the row, and separately notes its completion-flag half was already stale.
- A tracker rule now states that records corrections must not manufacture product scope from a misread criterion.

---

## 7. `V1R-103` tool confirmation — **Agree**

Verified. `docs/TOOLS_V1_POLICY.md:53-61` is explicit: the confirmation-required membership set is "**empty in V1**"; `ToolConfirmationPolicy::PerInvocation` and `ToolInvocationContext.user_confirmed` are "forward-looking, fail-closed router primitives... **not** an implemented end-to-end V1 confirmation feature"; and the router requires any future `CharacterAction` to reopen the requirement.

The earlier documents recorded this row as "re-verified and still true — leave unchecked." That was accurate about the code and wrong about the disposition: with no consequential V1 action, there is nothing requiring confirmation, and an unchecked row reads as an unresolved implementation defect. That is precisely the ambiguity `V1R-218` exists to remove.

**Changes:**
- Spec §9.2 records it as **not applicable for the V1 tool set**.
- `V1R-218` directs marking it N/A with the reopen condition stated explicitly.

---

## 8. `V1R-071` ambient deduplication — **Agree; settled by product decision**

The review is right that this differs from the stale rows: the finding is real and the requirement as literally written ("Prevent repeated near-identical remarks") is unmet. Leaving it unchecked indefinitely was the wrong disposition.

I checked whether the authoritative product specification settles it. It does not — `SPEC(20260818-163800).md:498` says only that ambient comments are "locally scheduled and filtered by cooldown/dedup/annoyance budget," without specifying what is deduplicated. So this was a genuine product question, not something to resolve by reading.

**Decision taken by the project owner on 2026-08-28:** Option B. Event-fingerprint deduplication is the intended V1 behavior; semantic near-duplicate suppression of generated remark text is not a V1 requirement. Recorded as spec §3.1 decision 2.

**Changes:**
- Spec §9.2 records the narrowing with the rationale and the authoritative citation.
- `V1R-218` directs rewriting the criterion to state event-fingerprint deduplication, citing `character/ambient.rs:76-88`, and closing on current evidence. No implementation task is added.

---

## 9. Prior tracing-flake incident — **Agree; this was the most valuable item**

Verified, and it changes the plan more than the review suggests.

Commit `886de2b4f` (`test: remove flaky tracing field assertions`, 2026-08-24) hit this exact failure in `tools/router/tests.rs`. It removed `assert!(logs.contains("remember_fact"))` and `assert!(logs.contains("Tool call rejected"))`, replaced them with assertions on `ToolAuditRecord` fields from `router.audit_snapshot()`, and recorded the cause in the replacing code:

> *"Positive routing assertions use the structured audit rather than formatted tracing text. Tracing callsite interest is process-global, so parallel tests can legitimately suppress a callsite in this thread-local formatter."*

Two consequences:

**The mechanism was never a hypothesis.** It was established in this repository four days before the review that recorded it as unconfirmed. Spec §5.1's hedging and §10.1's "unverified" entry were both wrong — not because the reasoning was faulty, but because the answer was already in the tree and this review did not find it. The narrowed 12-run reproduction that failed to trigger the flake shows only that the interfering subscriber lies outside those three tests; it was mistakenly read as weak evidence against the mechanism.

**The original prescription would have undone the prior fix.** `V1R-211` instructed "Add a positive control to `tools/router/tests.rs:249`" — precisely the assertion `886de2b4f` deleted for this flakiness. Following the plan as published would have reintroduced a known-flaky assertion into the one test already immune to the problem.

**Changes:**
- Spec §5.1 is rewritten: mechanism stated as established, with the commit and its in-code rationale as evidence.
- §10.1 is retitled "Superseded" and explains what the failed narrow reproduction does and does not show.
- The instruction to add a log-text control to `tools/router/tests.rs` is **withdrawn**, replaced by an explicit prohibition on re-adding one there.
- The repair is reframed around the pattern `886de2b4f` demonstrates: prove the positive side from structured data, not formatted tracing text.
- The §5.1 table now classifies each of the six log-privacy tests by control *form*. Three express their control as log text and are themselves exposed to the mechanism; only `tools/router/tests.rs` uses the robust form; two have no control at all.
- Investigation order is now prescribed: scoped/per-test dispatcher isolation first, then avoiding globally-cached-interest assertions, then serializing the minimum set — with weakening or deleting a privacy assertion excluded.
- §8 records `tools/router/tests.rs` as a verified strength demonstrating the correct pattern.

---

## Summary of document changes

| Item | Disposition | Primary change |
| --- | --- | --- |
| 1 | Agree | `V1R-210` untracks both trees; drift verifier dissolved |
| 2 | Agree | `V1R-212` narrowed; new `V1R-213` command-contract verifier |
| 3 | Agree | `V1R-215` rebuilt; false onboarding claim withdrawn |
| 4 | Agree | New spec §6.4 preview-adapter policy |
| 5 | Agree in principle | `V1R-216` narrowed; new `V1R-217` failure reconciliation |
| 6 | Agree | `V1R-110` recorded satisfied; no onboarding toggles |
| 7 | Agree | `V1R-103` recorded N/A with reopen condition |
| 8 | Agree | `V1R-071` narrowed to event dedup and closed |
| 9 | Agree | `V1R-211` repair reframed; withdrawn instruction that would undo `886de2b4f` |

Item count grew from 8 to 9; severities are now S1 ×2, S2 ×4, S3 ×2, S4 ×1.

Two documented errors of process are worth carrying forward beyond these specific fixes:

1. **A defect annotation was carried into a new document without being re-checked against the repair its own row described** (item 3). The refutation was one table row away, inside the same file.
2. **Repository history was not searched for prior occurrences of a failure mode before it was classified as an unconfirmed hypothesis** (item 9). A `git log` over the affected test file would have found it.

Both are now reflected in the tracker rules.
