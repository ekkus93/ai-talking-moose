# AI Talking Moose — Idle Banter TODO

**Date:** 2026-09-13
**Specification:** `docs/IDLE_BANTER_SPEC.md`
**Baseline:** `e824a4173583f6d673fcd27054cf16af130c92e3` (`master`)
**Status:** Planned implementation queue

Task IDs use the `IB-###` prefix (**Idle Banter**).

---

# IB-000 — Freeze scope and preserve existing architecture

## IB-001 — Record baseline and invariants

- [ ] Record implementation baseline `e824a4173583f6d673fcd27054cf16af130c92e3`.
- [ ] Confirm existing `AmbientScheduler` remains the authoritative ambient queue/cancellation mechanism.
- [ ] Confirm existing `BehaviorEngine`/`CooldownTracker` remain authoritative for global unsolicited-comments, mute/conversation, quiet hours, annoyance, dismissal, cooldown, dedup, and hourly-limit policy.
- [ ] Confirm existing selected text provider remains authoritative for ambient generation.
- [ ] Confirm existing standalone speech path remains authoritative for TTS/playback/mouth animation.
- [ ] Confirm no provider fallback is added.
- [ ] Confirm ordinary CI remains model-weight-free.
- [ ] Record that existing OS-level `AmbientEventCategory::Idle` remains distinct from Moose-interaction Idle Banter.

**Acceptance**

- [ ] Implementation plan extends existing architecture rather than creating parallel LLM, TTS, playback, or ambient-policy stacks.

---

# IB-100 — Add settings schema, defaults, migration, and validation

## IB-101 — Add persisted Idle Banter settings

- [ ] Add `idle_banter_enabled: bool` to Rust `AppSettings`.
- [ ] Add `idle_banter_initial_delay_minutes: u32`.
- [ ] Add `idle_banter_repeat_interval_minutes: u32`.
- [ ] Add `idle_banter_seed_topics: Vec<String>`.
- [ ] Default Idle Banter to enabled for new profiles.
- [ ] Default initial delay to 60 minutes.
- [ ] Default repeat interval to 30 minutes.
- [ ] Add one authoritative built-in default seed-topic list.
- [ ] Keep existing `unsolicited_comments` as the global master gate.

**Acceptance**

- [ ] `AppSettings::default()` represents the intended V1 Idle Banter behavior exactly.

## IB-102 — Bump settings version and migrate old profiles

- [ ] Bump `CURRENT_SETTINGS_VERSION` from the reviewed version.
- [ ] Migrate profiles that predate Idle Banter fields to the new defaults.
- [ ] Preserve all unrelated existing provider/voice/privacy/personality/quiet-hour settings.
- [ ] Preserve fail-closed rejection of future settings versions.
- [ ] Repair an invalid/empty legacy seed list to built-in defaults only during explicit migration/repair logic.
- [ ] Persist repaired/migrated settings when the existing migration path requires it.
- [ ] Add migration tests from the previous settings version.
- [ ] Add tests proving unrelated settings survive migration unchanged.

**Acceptance**

- [ ] Existing users receive safe Idle Banter defaults without configuration regression.

## IB-103 — Validate timing and seed settings

- [ ] Validate initial delay range 5–1,440 minutes.
- [ ] Validate repeat interval range 5–1,440 minutes.
- [ ] Validate seed count 1–64.
- [ ] Validate each trimmed seed topic is non-empty.
- [ ] Validate each seed topic is <= 120 characters.
- [ ] Validate total normalized seed text <= 4,096 characters.
- [ ] Reject case-insensitive duplicate topics or normalize them deterministically according to the spec.
- [ ] Keep backend validation authoritative.
- [ ] Add focused settings-policy tests for every invalid boundary.

**Acceptance**

- [ ] Invalid timing/seed settings fail transactionally and cannot partially update runtime state.

## IB-104 — Normalize seed topics in one authoritative helper

- [ ] Trim leading/trailing whitespace.
- [ ] Preserve stable display order.
- [ ] Implement case-insensitive duplicate detection.
- [ ] Ensure normalized result cannot be empty during normal save.
- [ ] Expose/derive built-in defaults from one authoritative source.
- [ ] Add normalization tests for Unicode, whitespace, duplicates, max lengths, and stable order.

**Acceptance**

- [ ] Backend and frontend cannot drift on seed normalization/default semantics.

---

# IB-200 — Implement Idle Banter scheduling runtime

## IB-201 — Add dedicated Idle Banter runtime/controller

- [ ] Add a small `IdleBanterRuntime`/`IdleBanterController` or equivalent backend component.
- [ ] Track time since last direct Moose interaction using monotonic elapsed time.
- [ ] Track initial-delay versus repeat phase.
- [ ] Track at most one pending/due occurrence.
- [ ] Do not own an LLM client.
- [ ] Do not own a TTS provider.
- [ ] Do not own audio playback.
- [ ] Do not add a second ambient request queue.
- [ ] Integrate lifecycle with `AppState` startup/shutdown.
- [ ] Provide deterministic test hooks for clock and randomness.

**Acceptance**

- [ ] Idle Banter scheduling is independently testable but delivery remains entirely on the existing ambient/speech pipeline.

## IB-202 — Implement default initial-delay behavior

- [ ] Start a fresh inactivity episode on application startup.
- [ ] Do not emit before configured initial delay.
- [ ] Emit exactly one due occurrence when the initial threshold is reached.
- [ ] Do not jitter the initial delay in V1.
- [ ] Ensure repeated runtime polls cannot duplicate the same due occurrence.

**Acceptance**

- [ ] Default new profile cannot attempt Idle Banter before 60 minutes of direct-Moose inactivity.

## IB-203 — Implement repeat interval with jitter

- [ ] Define fixed V1 repeat jitter at ±20%.
- [ ] Compute repeat deadline from configured repeat interval after an occurrence is attempted.
- [ ] Default 30-minute repeat must produce deadlines between 24 and 36 minutes.
- [ ] Use injectable/deterministic RNG in tests.
- [ ] Avoid selecting pathological negative/zero durations after jitter/clamping.
- [ ] Do not expose jitter as a user setting in V1.

**Acceptance**

- [ ] Repeat timing feels non-mechanical while remaining bounded and deterministic under test.

## IB-204 — Prevent backlog and retry storms

- [ ] Never queue multiple missed Idle Banter occurrences.
- [ ] Count a handed-off due occurrence as attempted for scheduling even if generation fails.
- [ ] Provider failure schedules the next normal repeat instead of rapid retry.
- [ ] TTS failure schedules the next normal repeat instead of rapid retry.
- [ ] Ambient queue-full/drop does not block foreground work.
- [ ] Long sleep/suspend does not cause multiple catch-up remarks.

**Acceptance**

- [ ] At most one Idle Banter occurrence can be pending and missed intervals never burst afterward.

## IB-205 — Reset on startup, enable transitions, and wake

- [ ] Disabling Idle Banter invalidates pending eligibility/work.
- [ ] Re-enabling starts a fresh inactivity episode.
- [ ] System wake/resume resets to a fresh initial-delay episode.
- [ ] Application startup starts a fresh initial-delay episode.
- [ ] Add deterministic tests for each transition.

**Acceptance**

- [ ] Moose never immediately speaks on launch/wake/re-enable due solely to stale elapsed time.

---

# IB-210 — Define and instrument direct Moose activity

## IB-211 — Add one centralized user-interaction reset API

- [ ] Add `record_user_interaction()` or equivalent authoritative helper.
- [ ] Reset idle-banter timing through that helper.
- [ ] Invalidate/cancel stale pending Idle Banter through the existing ambient interruption mechanism where appropriate.
- [ ] Do not duplicate timer arithmetic at command call sites.
- [ ] Keep the helper cheap and non-blocking.

**Acceptance**

- [ ] All command surfaces use one consistent reset mechanism.

## IB-212 — Hook conversation activity

- [ ] Reset on `start_conversation`.
- [ ] Reset on `stop_conversation`.
- [ ] Reset on `barge_in`.
- [ ] Reset on a non-empty `send_text_message` before long-running generation.
- [ ] Do not treat empty/validation-rejected text as meaningful activity unless UI policy intentionally does so.
- [ ] Add focused command/unit tests as practical.

**Acceptance**

- [ ] Direct conversational activity restarts the full initial Idle Banter delay.

## IB-213 — Hook character/control activity

- [ ] Reset on `trigger_canned_reaction`.
- [ ] Reset on explicit `show_moose`.
- [ ] Reset on explicit `hide_moose`.
- [ ] Reset on `dismiss_moose`.
- [ ] Reset on mute/unmute change.
- [ ] Reset on voice audition.
- [ ] Reset on successful Settings update.
- [ ] Add focused tests for representative command boundaries.

**Acceptance**

- [ ] User attention to Moose prevents stale scheduled banter from firing immediately afterward.

---

# IB-300 — Add Idle Banter ambient event and policy semantics

## IB-301 — Add distinct ambient event category

- [ ] Add `AmbientEventCategory::IdleBanter` or equivalent.
- [ ] Keep existing OS-level `Idle` category unchanged.
- [ ] Update serialization/contract tests for the new category.
- [ ] Update exhaustive privacy/policy matches.
- [ ] Treat Idle Banter event metadata as privacy-safe when it contains only bounded inactivity duration and configured seed topic.

**Acceptance**

- [ ] System-idle observations and Moose-interaction Idle Banter cannot be confused in policy or tests.

## IB-302 — Preserve hard ambient gates

- [ ] Require global `unsolicited_comments` master enablement.
- [ ] Require `idle_banter_enabled`.
- [ ] Respect mute.
- [ ] Respect active conversation/foreground work.
- [ ] Respect quiet hours.
- [ ] Respect annoyance budget.
- [ ] Respect dismissal cooldown.
- [ ] Respect minimum ambient cooldown.
- [ ] Respect maximum ambient comments per hour.
- [ ] Preserve duplicate-event protection where applicable.

**Acceptance**

- [ ] Idle Banter cannot bypass existing anti-annoyance or foreground-work protections.

## IB-303 — Exempt due Idle Banter from generic importance threshold

- [ ] Make scheduled due Idle Banter independent of generic event-importance/talkativeness thresholding.
- [ ] Do not change threshold behavior for Application/Power/Wake/System/other existing ambient categories.
- [ ] Add behavior-engine tests proving the distinction.

**Acceptance**

- [ ] A user-configured due Idle Banter event is not silently suppressed merely because default talkativeness would reject a low-importance event.

---

# IB-310 — Implement seed selection, prompt construction, and repetition control

## IB-311 — Add deterministic random seed-topic selection

- [ ] Select one topic from the current normalized configured list.
- [ ] Use injectable RNG for tests.
- [ ] Avoid selecting the same seed twice consecutively when at least two topics exist.
- [ ] Handle single-topic lists without looping.
- [ ] Never select a topic outside the configured list.

**Acceptance**

- [ ] Seed selection is random in production and deterministic under test.

## IB-312 — Add specialized Idle Banter prompt builder

- [ ] Extend `PromptBuilder` with an Idle Banter-specific builder or equivalent.
- [ ] Reuse existing Moose identity/personality/core rules.
- [ ] Include bounded approximate inactivity duration.
- [ ] Include exactly one bounded creative-direction seed topic.
- [ ] Instruct model to produce one short idle remark.
- [ ] Require dry/snarky/mildly absurd/mischievous rather than cruel tone.
- [ ] Require one or two short sentences maximum.
- [ ] Target roughly <= 25 words.
- [ ] Forbid greeting/assistant-fluff/help offers.
- [ ] Forbid "I am an AI/model/assistant" language.
- [ ] Forbid unsolicited advice and joke explanation.
- [ ] Tell model not to mechanically restate seed text.
- [ ] State that seed content cannot override core character/safety rules.
- [ ] Keep final prompt within existing bounded prompt budgets.

**Acceptance**

- [ ] Prompt consistently produces character banter rather than generic assistant chatter.

## IB-313 — Add session-only recent banter history

- [ ] Maintain ring buffer of the last 12 successfully delivered Idle Banter lines.
- [ ] Maintain normalized fingerprints for exact duplicate detection.
- [ ] Do not persist history to SQLite.
- [ ] Clear history on app restart.
- [ ] Bound history included in prompts.
- [ ] Do not include user transcript text in this history.
- [ ] Add tests proving buffer remains bounded.

**Acceptance**

- [ ] Repetition control has bounded memory and creates no new retained user-data store.

## IB-314 — Suppress exact recent duplicates

- [ ] Normalize/fingerprint generated Idle Banter candidate before speech.
- [ ] If candidate duplicates recent Idle Banter history, skip speech.
- [ ] Do not start an unbounded immediate regenerate loop.
- [ ] Schedule next normal interval after duplicate suppression.
- [ ] Add deterministic duplicate tests.

**Acceptance**

- [ ] Exact repeated lines cannot be spoken repeatedly inside the recent-history window.

---

# IB-320 — Preserve provider, privacy, and cancellation semantics

## IB-321 — Route through selected text provider only

- [ ] Reuse coherent text-provider/settings snapshot used by existing ambient generation.
- [ ] Prove Local selection fails locally when unavailable.
- [ ] Prove Google selection fails as Google when auth is unavailable.
- [ ] Prove no Local→Google fallback.
- [ ] Prove no Google→Local fallback.
- [ ] Skip failed occurrence rather than surfacing repeated intrusive background errors.

**Acceptance**

- [ ] Idle Banter never changes network/provider behavior behind the user's back.

## IB-322 — Preserve memory/transcript privacy

- [ ] Do not automatically include recent conversation transcript text in Idle Banter V1.
- [ ] Reuse existing `memory_enabled` gate if ambient memory context is retained.
- [ ] Prove disabled memory sentinel is absent from Idle Banter prompt.
- [ ] Prove arbitrary transcript sentinel is absent from Idle Banter prompt.
- [ ] Do not log prompt bodies or seed-topic text at normal log levels.
- [ ] Keep event metadata free of window-title/application content unless separately authorized through existing observation semantics.

**Acceptance**

- [ ] Unsolicited banter introduces no hidden transcript/context exfiltration path.

## IB-323 — Make user activity preempt stale background work

- [ ] Reuse `AmbientScheduler::interrupt()`/epoch invalidation where applicable.
- [ ] Cancel or invalidate pending Idle Banter generation when direct user activity begins.
- [ ] Do not let stale generated banter speak after a new conversation/message starts.
- [ ] Add concurrency test around activity arriving during pending ambient work.

**Acceptance**

- [ ] Foreground user intent always wins over background banter.

---

# IB-400 — Reuse authoritative standalone speech delivery

## IB-401 — Deliver through existing ambient/speech pipeline

- [ ] Submit due Idle Banter through `AmbientScheduler`.
- [ ] Generate through `process_ambient_event` or a narrowly refactored shared ambient path.
- [ ] Preserve second deterministic policy check before speech.
- [ ] Route accepted text through `invoke_standalone_speech`.
- [ ] Preserve configured standalone TTS provider.
- [ ] Preserve bounded audio queue semantics.
- [ ] Preserve speech bubble and mouth animation.
- [ ] Preserve hidden/dismissed ambient appearance semantics.
- [ ] Preserve configured post-ambient hide delay.

**Acceptance**

- [ ] No Idle Banter-specific direct TTS/playback implementation exists.

## IB-402 — Handle failures quietly

- [ ] Restore character state after generation failure.
- [ ] Restore character state after TTS/queue failure.
- [ ] Do not show intrusive dialogs for expected background provider failures.
- [ ] Do not retry rapidly after failure.
- [ ] Ensure shutdown cancels pending Idle Banter cleanly.

**Acceptance**

- [ ] Background banter failure cannot wedge Moose in Thinking/Talking or interfere with foreground operation.

---

# IB-500 — Add Settings UI and generated contracts

## IB-501 — Add Idle Banter Behavior settings UI

- [ ] Add Idle Banter subsection to `BehaviorTab`.
- [ ] Add enable/disable checkbox.
- [ ] Add initial-delay editor with clear minutes/hours presentation.
- [ ] Add repeat-interval editor labeled approximately/"About every".
- [ ] Disable/de-emphasize dependent controls when Idle Banter is off.
- [ ] Explain global unsolicited-comments master dependency when master is off.
- [ ] Preserve accessibility labels and keyboard navigation.

**Acceptance**

- [ ] User can understand and configure when Idle Banter starts and repeats.

## IB-502 — Add editable seed-topic list UI

- [ ] Render one seed topic per row.
- [ ] Add new topic.
- [ ] Edit existing topic.
- [ ] Delete topic.
- [ ] Prevent committing an empty list.
- [ ] Surface duplicate/empty/length validation.
- [ ] Add Restore Defaults action.
- [ ] Avoid requiring reordering in V1.
- [ ] Add frontend tests for every seed action.

**Acceptance**

- [ ] User can fully customize creative-direction seeds without editing config files.

## IB-503 — Update frontend/backend settings contracts

- [ ] Update TypeScript `AppSettings` exactly.
- [ ] Regenerate `src/generated/backendContract.json`.
- [ ] Update browser-preview representative settings.
- [ ] Update frontend test setup/default mocks.
- [ ] Update settings store tests.
- [ ] Update settings modal tests.
- [ ] Update generated-contract negative probe/shape checks if needed.
- [ ] Prove a missing/mistyped Idle Banter field fails the contract gate.

**Acceptance**

- [ ] Rust and TypeScript settings cannot drift silently.

---

# IB-600 — Deterministic backend qualification

## IB-601 — Add scheduler timing tests

- [ ] No due event before initial threshold.
- [ ] One due event at threshold.
- [ ] No duplicate due event while pending.
- [ ] Repeat deadline lies inside ±20% jitter window.
- [ ] Activity resets full initial delay.
- [ ] Disable invalidates pending due state.
- [ ] Re-enable starts fresh.
- [ ] Wake starts fresh.
- [ ] Missed intervals do not backlog.
- [ ] Provider failure does not rapid-retry.

**Acceptance**

- [ ] No real-time sleeps are required to prove hour-scale scheduling behavior.

## IB-602 — Add policy and busy-state tests

- [ ] Global unsolicited disable blocks Idle Banter.
- [ ] Dedicated Idle Banter disable blocks it.
- [ ] Mute blocks it.
- [ ] Active conversation blocks/preempts it.
- [ ] Quiet hours block it.
- [ ] Hourly budget blocks it.
- [ ] Annoyance budget blocks it.
- [ ] Dismissal cooldown blocks it.
- [ ] Fixed ambient cooldown remains enforced.
- [ ] Generic talkativeness/importance threshold does not block a due Idle Banter event.
- [ ] Existing categories retain generic threshold behavior.

**Acceptance**

- [ ] Specialized scheduling does not weaken existing anti-annoyance policy.

## IB-603 — Add prompt/privacy tests

- [ ] Prompt contains persona.
- [ ] Prompt contains selected seed.
- [ ] Prompt contains bounded inactivity duration.
- [ ] Prompt contains Idle Banter brevity/anti-assistant instructions.
- [ ] Prompt includes bounded recent generated banter when available.
- [ ] Disabled-memory sentinel is absent.
- [ ] Transcript sentinel is absent.
- [ ] Unexpected desktop/window sentinel is absent.
- [ ] Prompt/log sentinel does not appear in tracing.
- [ ] Positive controls prove expected seed/personality fields are actually present.

**Acceptance**

- [ ] Privacy tests are non-vacuous and would fail if future code accidentally injects user transcript/context.

## IB-604 — Add provider and delivery tests

- [ ] Local text route remains Local.
- [ ] Google text route remains Google.
- [ ] No text-provider fallback.
- [ ] Existing selected TTS provider remains authoritative.
- [ ] No TTS-provider fallback.
- [ ] Exact duplicate generated line is skipped.
- [ ] Successful delivery records recent history.
- [ ] Failed delivery does not record a successful-history entry.
- [ ] User interaction during pending banter prevents stale delivery.

**Acceptance**

- [ ] End-to-end control flow preserves provider and foreground-preemption invariants.

---

# IB-700 — Frontend qualification and UX regression tests

## IB-701 — Add Behavior tab tests

- [ ] Default Idle Banter controls render correctly.
- [ ] Enable toggle persists.
- [ ] Initial delay persists.
- [ ] Repeat interval persists.
- [ ] Global ambient-off explanatory state renders.
- [ ] Disabled dependent controls behave correctly.
- [ ] Validation errors are accessible.

## IB-702 — Add seed-editor tests

- [ ] Add topic.
- [ ] Edit topic.
- [ ] Delete topic.
- [ ] Cannot delete/commit final topic into an empty list.
- [ ] Duplicate topic rejected.
- [ ] Oversized topic rejected.
- [ ] Restore Defaults returns exact authoritative defaults.
- [ ] Store/backend update contains normalized list.

**Acceptance**

- [ ] Settings UX is fully covered without relying on manual-only happy-path testing.

---

# IB-800 — Documentation and privacy disclosure

## IB-801 — Update README

- [ ] Describe Idle Banter purpose.
- [ ] Document default 60-minute initial delay.
- [ ] Document default roughly-30-minute repeat behavior.
- [ ] Explain editable seed topics.
- [ ] Explain that direct Moose interaction resets inactivity.
- [ ] Distinguish Moose inactivity from OS keyboard/mouse idle observation.
- [ ] Explain generation uses selected text provider.
- [ ] Explain speech uses selected standalone TTS provider.
- [ ] State no provider fallback.

## IB-802 — Update privacy documentation

- [ ] Document that seed topics are prompt input to selected text provider.
- [ ] Document that recent transcript is not automatically injected in V1.
- [ ] Document existing memory setting remains authoritative.
- [ ] Document Local versus Google network implications.
- [ ] Document recent banter history is session-only and not persisted.
- [ ] Document Idle Banter can be disabled independently and through global unsolicited-comments master switch.

**Acceptance**

- [ ] User documentation truthfully describes when unsolicited cloud/local generation can occur and what context it uses.

---

# IB-900 — Final static/local validation

## IB-901 — Run available validation

- [ ] `git diff --check` passes.
- [ ] Rust format passes.
- [ ] Rust Clippy passes.
- [ ] Rust unit/integration tests pass.
- [ ] Generated backend contract validation passes.
- [ ] Tauri command/IPC contract checks pass.
- [ ] Frontend formatting passes.
- [ ] Frontend lint passes.
- [ ] Frontend typecheck passes.
- [ ] Frontend unit tests pass.
- [ ] Canonical repository aggregate check passes where available.
- [ ] Record any local-environment limitations without falsely claiming unavailable gates passed.

**Acceptance**

- [ ] All locally available deterministic checks pass before final PR qualification.

---

# IB-910 — Exact-head CI and merge closeout

## IB-911 — Pass exact final PR-head CI

- [ ] Record exact final implementation head SHA.
- [ ] Confirm ordinary CI runs on that exact SHA.
- [ ] Confirm Rust quality/tests pass.
- [ ] Confirm frontend quality/tests pass.
- [ ] Confirm generated-contract/command-shape gates pass.
- [ ] Confirm no ordinary CI job downloads heavyweight LLM/TTS model weights solely for Idle Banter scheduling tests.
- [ ] Record exact workflow run ID(s).

**Acceptance**

- [ ] The exact commit intended for merge is green.

## IB-912 — Decide whether heavyweight real-model acceptance is required

- [ ] Audit final diff for Local LLM runtime changes.
- [ ] Audit final diff for Local TTS runtime/native/model changes.
- [ ] If runtime/model code did not change, record that existing runtime qualification remains applicable and do not rerun heavyweight acceptance solely for scheduling/UI changes.
- [ ] If runtime/model code did change, rerun the appropriate explicit acceptance workflow on exact final head.

**Acceptance**

- [ ] Expensive validation is evidence-driven rather than automatic or omitted when actually required.

## IB-913 — Guarded merge and exact-master verification

- [ ] Confirm PR is mergeable and exact head is current.
- [ ] Merge using expected-head guard where available.
- [ ] Record exact merge commit/master SHA.
- [ ] Verify post-merge CI on exact master SHA.
- [ ] Confirm no stale feature branch work is being mistaken for master state.
- [ ] Update this tracker/evidence only when the resulting documentation commit does not create recursive qualification requirements under repository policy.

**Acceptance**

- [ ] Idle Banter is present and green on exact `master`.

---

# Final acceptance checklist

- [ ] Idle Banter defaults enabled.
- [ ] First remark default is after 60 minutes of direct Moose inactivity.
- [ ] Repeat default is about every 30 minutes.
- [ ] Repeat timing uses bounded ±20% jitter.
- [ ] Direct Moose interaction resets timing.
- [ ] Startup/wake/re-enable start fresh inactivity episodes.
- [ ] Missed occurrences never backlog.
- [ ] Global unsolicited-comments master gate remains authoritative.
- [ ] Quiet hours, mute, conversation, annoyance, dismissal, cooldown, and hourly limits remain authoritative.
- [ ] Due Idle Banter is not suppressed by generic talkativeness/importance thresholding.
- [ ] Editable seed-topic list ships with defaults.
- [ ] Add/edit/delete/restore-default seed UX works.
- [ ] Random selection avoids immediate seed repetition when possible.
- [ ] Recent 12 delivered idle lines are session-only and bounded.
- [ ] Exact recent duplicate lines are suppressed.
- [ ] Prompt is Moose-specific, brief, snarky, and anti-assistant-fluff.
- [ ] No recent user transcript is automatically injected in V1.
- [ ] Existing memory privacy gate remains authoritative.
- [ ] Selected text provider is used with no fallback.
- [ ] Existing standalone TTS provider/path is used with no fallback.
- [ ] Foreground interaction preempts stale Idle Banter.
- [ ] Settings migration and frontend/backend contracts are complete.
- [ ] Backend deterministic timing/privacy/routing tests pass.
- [ ] Frontend settings/seed-editor tests pass.
- [ ] README/privacy docs match implementation.
- [ ] Exact final PR-head CI passes.
- [ ] Guarded merge completes.
- [ ] Exact merged-master CI passes.
