# AI Talking Moose — Idle Banter TODO

**Date:** 2026-09-13
**Specification:** `docs/IDLE_BANTER_SPEC.md`
**Baseline:** `e824a4173583f6d673fcd27054cf16af130c92e3` (`master`)
**Status:** Complete — reconciled against merged implementation and CI evidence on 2026-09-13

Task IDs use the `IB-###` prefix (**Idle Banter**).

## Closeout evidence

This tracker was reconciled item-by-item after implementation. A checked item means current `master` contains merged code/tests/docs or exact CI evidence that satisfies the requirement; it does not merely mean the work was planned.

Primary qualification evidence:

- implementation PR #105 final head `d103facdacf4cd45b73d28c0637d108f148fc631`; CI `34778120985` **PASS** (Rust quality/tests, frontend gate, generated backend contract);
- implementation merge/master `c531fa4098b32a5acb37e485dd4a8b091d70dadf`; CI `34778513562` **PASS**;
- exact implementation-head KittenTTS production CPU acceptance `34778120998` **PASS** on Linux x86_64 and macOS arm64;
- post-merge KittenTTS production CPU acceptance `34778513583` **PASS** on retry attempt 2 on exact source SHA `c531fa4098b32a5acb37e485dd4a8b091d70dadf`;
- closeout-hardening PR #106 final head `fcdb80e96335dfa719f7cfb4d8241f3210b70221`; CI `34785841957` **PASS**;
- final code-bearing master `9624dc1611a59103b366b5aefdf46696d3de3def`; post-merge CI `34786184975` **PASS**.

The path-classified CI did not separately run the monolithic `npm run check:all` job for the implementation diff. Its constituent production gates did run and pass: `npm run check:frontend` (including command/shape/negative-probe checks, typecheck, lint, Prettier, Vitest, and build), Rust formatting/Clippy/tests, and generated-contract verification. Ordinary CI remained model-weight-free; heavyweight KittenTTS acceptance ran as a separate explicit workflow.

### Item-by-item evidence map

| Task | Merged evidence |
| --- | --- |
| IB-001 | Existing `AmbientScheduler`, `BehaviorEngine`/`CooldownTracker`, selected-provider generation, and standalone speech remain authoritative; `IdleBanterRuntime` contains scheduling/history state only. |
| IB-101 | `src-tauri/src/app/state.rs` adds the four persisted fields and exact 60/30/enabled/default-seed defaults; `idle_banter.rs` owns the built-in seed list. |
| IB-102 | Settings schema is version 5; `from_persisted_json` migrates v4, preserves unrelated preferences, repairs legacy invalid seeds, rejects future versions, and has migration/round-trip tests. |
| IB-103 | `settings_policy.rs` enforces timing ranges and delegates seed validation to the authoritative normalizer; transactional persistence applies runtime state only after persistence succeeds. |
| IB-104 | `normalize_idle_banter_seed_topics` trims, preserves order, bounds counts/length/budget, and rejects case-insensitive duplicates; Unicode/boundary tests are merged. |
| IB-201 | `IdleBanterRuntime` is integrated into `AppState` and the 15-second desktop runtime poll without owning model/TTS/playback/queue services; injected `Instant`/sample test hooks make scheduling deterministic. |
| IB-202 | Scheduler tests prove no event before 60 minutes, exactly one at threshold, no initial jitter, and no duplicate due result. |
| IB-203 | Fixed `IDLE_BANTER_REPEAT_JITTER_FRACTION = 0.20`; tests prove the default 30-minute range is exactly 24–36 minutes and random sampling is injectable. |
| IB-204 | Due polling advances the next repeat before delivery; background submission is bounded/non-blocking; tests prove no backlog and no rapid retry semantics. |
| IB-205 | Disable/re-enable, startup, and wake all start fresh episodes; wake uses current settings and interrupts stale ambient work. |
| IB-211 | `AppState::record_user_interaction()` centralizes ambient interruption plus Idle Banter reset. |
| IB-212 | `start_conversation`, `stop_conversation`, `barge_in`, and validated non-empty `send_text_message` call the centralized reset before long-running foreground work. |
| IB-213 | canned reaction, show/hide/dismiss, mute changes, Google/Local voice audition, and successful settings updates call the centralized reset. |
| IB-301 | `AmbientEventCategory::IdleBanter` is distinct from OS `Idle`; serialization/category/privacy tests and bounded seed/inactivity metadata are merged. |
| IB-302 | `behavior.rs` has an explicit test covering unsolicited, dedicated enable, mute, conversation, quiet hours, annoyance, dismissal, cooldown, hourly limit, and duplicate-event gates. |
| IB-303 | Idle Banter bypasses only the generic importance threshold; tests prove ordinary ambient categories retain it. |
| IB-311 | Random seed selection uses only normalized configured topics, avoids immediate repetition when possible, handles one topic, and is tested across the selectable range. |
| IB-312 | `PromptBuilder::build_idle_banter_prompt` reuses the Moose system instruction and enforces bounded seed/inactivity, brevity, tone, anti-assistant, anti-advice, and seed-safety instructions. |
| IB-313 | `IdleBanterRuntime` owns a non-persisted 12-line ring buffer; prompt inclusion is bounded and tests prove eviction/bounds. |
| IB-314 | normalized exact recent duplicates are suppressed before speech with no immediate regenerate loop; only completed delivery records history. |
| IB-321 | ambient generation uses one selected text-provider snapshot; Local-unavailable and Google-no-auth tests prove fail-in-provider behavior with no fallback. |
| IB-322 | Idle Banter prompt construction accepts memory only through the existing gate and no transcript surface; sentinel tests cover memory/transcript/privacy plus tracing non-disclosure. |
| IB-323 | centralized interaction calls `AmbientScheduler::interrupt`; scheduler epoch/cancellation/presentation-ownership tests prove stale ambient work cannot beat foreground intent or clobber newer presentation. |
| IB-401 | `process_ambient_event` remains the delivery path, performs the second policy gate, then uses `invoke_standalone_speech_for_ambient` and the existing bounded playback/presentation/hide-delay machinery. |
| IB-402 | ambient failure/cancellation guards restore state without stale cleanup; background submission discards expected failure responses; scheduler/desktop runtime shutdown is cancellation-aware. |
| IB-501 | `BehaviorTab.tsx` has enable, initial/repeat timing, disabled-state/master-gate explanation, and accessible labels. |
| IB-502 | UI supports add/edit/delete/save/restore defaults with empty/duplicate/length enforcement; `IdleBanterSettings.test.tsx` covers each action. |
| IB-503 | TypeScript settings, generated backend contract, frontend defaults/mocks, settings tests, and negative contract probe all include Idle Banter fields. |
| IB-601 | deterministic runtime tests cover threshold, single due, jitter, reset, disable/re-enable, wake, missed intervals, and attempt-before-delivery retry prevention without hour-scale sleeps. |
| IB-602 | behavior tests cover all hard gates plus the specialized importance-threshold exemption. |
| IB-603 | prompt/privacy tests prove persona/seed/inactivity/recent-history content, disabled-memory/transcript exclusion, bounded prompts, and prompt-private tracing behavior. |
| IB-604 | text and standalone-TTS provider ownership/no-fallback tests are merged; duplicate/history and scheduler cancellation/presentation tests cover delivery/preemption. |
| IB-701 | frontend tests cover defaults, toggle/timing persistence, global ambient-off state, disabled controls, and accessible validation. |
| IB-702 | frontend tests cover add/edit/delete/final-entry protection/duplicate/oversize/restore-default and normalized persisted list behavior. |
| IB-801 | `README.md` documents purpose, 60-minute first delay, roughly 30-minute jittered repeats, editable seeds, direct-Moose reset semantics, OS-idle distinction, provider ownership, and no fallback. |
| IB-802 | `docs/PRIVACY.md` documents seed prompt input, no automatic transcript injection, memory gate, Local/Google network behavior, session-only history, and both disable gates. |
| IB-901 | Exact implementation-head CI `34778120985` and merged-master CI `34778513562` passed the constituent frontend/Rust/contract gates; `git diff --check` is clean for the implementation range. |
| IB-911 | Exact final implementation head `d103facd...` passed CI `34778120985`; exact closeout-hardening head `fcdb80e9...` passed CI `34785841957`. |
| IB-912 | Shared standalone-speech/ambient cancellation plumbing changed, so explicit KittenTTS CPU acceptance was run rather than assumed: `34778120998` and post-merge `34778513583` passed. |
| IB-913 | PR #105 merged as `c531fa40...`; PR #106 merged into final code-bearing master `9624dc16...`; exact post-merge CI `34786184975` passed. |

---

# IB-000 — Freeze scope and preserve existing architecture

## IB-001 — Record baseline and invariants

- [x] Record implementation baseline `e824a4173583f6d673fcd27054cf16af130c92e3`.
- [x] Confirm existing `AmbientScheduler` remains the authoritative ambient queue/cancellation mechanism.
- [x] Confirm existing `BehaviorEngine`/`CooldownTracker` remain authoritative for global unsolicited-comments, mute/conversation, quiet hours, annoyance, dismissal, cooldown, dedup, and hourly-limit policy.
- [x] Confirm existing selected text provider remains authoritative for ambient generation.
- [x] Confirm existing standalone speech path remains authoritative for TTS/playback/mouth animation.
- [x] Confirm no provider fallback is added.
- [x] Confirm ordinary CI remains model-weight-free.
- [x] Record that existing OS-level `AmbientEventCategory::Idle` remains distinct from Moose-interaction Idle Banter.

**Acceptance**

- [x] Implementation plan extends existing architecture rather than creating parallel LLM, TTS, playback, or ambient-policy stacks.

---

# IB-100 — Add settings schema, defaults, migration, and validation

## IB-101 — Add persisted Idle Banter settings

- [x] Add `idle_banter_enabled: bool` to Rust `AppSettings`.
- [x] Add `idle_banter_initial_delay_minutes: u32`.
- [x] Add `idle_banter_repeat_interval_minutes: u32`.
- [x] Add `idle_banter_seed_topics: Vec<String>`.
- [x] Default Idle Banter to enabled for new profiles.
- [x] Default initial delay to 60 minutes.
- [x] Default repeat interval to 30 minutes.
- [x] Add one authoritative built-in default seed-topic list.
- [x] Keep existing `unsolicited_comments` as the global master gate.

**Acceptance**

- [x] `AppSettings::default()` represents the intended V1 Idle Banter behavior exactly.

## IB-102 — Bump settings version and migrate old profiles

- [x] Bump `CURRENT_SETTINGS_VERSION` from the reviewed version.
- [x] Migrate profiles that predate Idle Banter fields to the new defaults.
- [x] Preserve all unrelated existing provider/voice/privacy/personality/quiet-hour settings.
- [x] Preserve fail-closed rejection of future settings versions.
- [x] Repair an invalid/empty legacy seed list to built-in defaults only during explicit migration/repair logic.
- [x] Persist repaired/migrated settings when the existing migration path requires it.
- [x] Add migration tests from the previous settings version.
- [x] Add tests proving unrelated settings survive migration unchanged.

**Acceptance**

- [x] Existing users receive safe Idle Banter defaults without configuration regression.

## IB-103 — Validate timing and seed settings

- [x] Validate initial delay range 5–1,440 minutes.
- [x] Validate repeat interval range 5–1,440 minutes.
- [x] Validate seed count 1–64.
- [x] Validate each trimmed seed topic is non-empty.
- [x] Validate each seed topic is <= 120 characters.
- [x] Validate total normalized seed text <= 4,096 characters.
- [x] Reject case-insensitive duplicate topics or normalize them deterministically according to the spec.
- [x] Keep backend validation authoritative.
- [x] Add focused settings-policy tests for every invalid boundary.

**Acceptance**

- [x] Invalid timing/seed settings fail transactionally and cannot partially update runtime state.

## IB-104 — Normalize seed topics in one authoritative helper

- [x] Trim leading/trailing whitespace.
- [x] Preserve stable display order.
- [x] Implement case-insensitive duplicate detection.
- [x] Ensure normalized result cannot be empty during normal save.
- [x] Expose/derive built-in defaults from one authoritative source.
- [x] Add normalization tests for Unicode, whitespace, duplicates, max lengths, and stable order.

**Acceptance**

- [x] Backend and frontend cannot drift on seed normalization/default semantics.

---

# IB-200 — Implement Idle Banter scheduling runtime

## IB-201 — Add dedicated Idle Banter runtime/controller

- [x] Add a small `IdleBanterRuntime`/`IdleBanterController` or equivalent backend component.
- [x] Track time since last direct Moose interaction using monotonic elapsed time.
- [x] Track initial-delay versus repeat phase.
- [x] Track at most one pending/due occurrence.
- [x] Do not own an LLM client.
- [x] Do not own a TTS provider.
- [x] Do not own audio playback.
- [x] Do not add a second ambient request queue.
- [x] Integrate lifecycle with `AppState` startup/shutdown.
- [x] Provide deterministic test hooks for clock and randomness.

**Acceptance**

- [x] Idle Banter scheduling is independently testable but delivery remains entirely on the existing ambient/speech pipeline.

## IB-202 — Implement default initial-delay behavior

- [x] Start a fresh inactivity episode on application startup.
- [x] Do not emit before configured initial delay.
- [x] Emit exactly one due occurrence when the initial threshold is reached.
- [x] Do not jitter the initial delay in V1.
- [x] Ensure repeated runtime polls cannot duplicate the same due occurrence.

**Acceptance**

- [x] Default new profile cannot attempt Idle Banter before 60 minutes of direct-Moose inactivity.

## IB-203 — Implement repeat interval with jitter

- [x] Define fixed V1 repeat jitter at ±20%.
- [x] Compute repeat deadline from configured repeat interval after an occurrence is attempted.
- [x] Default 30-minute repeat must produce deadlines between 24 and 36 minutes.
- [x] Use injectable/deterministic RNG in tests.
- [x] Avoid selecting pathological negative/zero durations after jitter/clamping.
- [x] Do not expose jitter as a user setting in V1.

**Acceptance**

- [x] Repeat timing feels non-mechanical while remaining bounded and deterministic under test.

## IB-204 — Prevent backlog and retry storms

- [x] Never queue multiple missed Idle Banter occurrences.
- [x] Count a handed-off due occurrence as attempted for scheduling even if generation fails.
- [x] Provider failure schedules the next normal repeat instead of rapid retry.
- [x] TTS failure schedules the next normal repeat instead of rapid retry.
- [x] Ambient queue-full/drop does not block foreground work.
- [x] Long sleep/suspend does not cause multiple catch-up remarks.

**Acceptance**

- [x] At most one Idle Banter occurrence can be pending and missed intervals never burst afterward.

## IB-205 — Reset on startup, enable transitions, and wake

- [x] Disabling Idle Banter invalidates pending eligibility/work.
- [x] Re-enabling starts a fresh inactivity episode.
- [x] System wake/resume resets to a fresh initial-delay episode.
- [x] Application startup starts a fresh initial-delay episode.
- [x] Add deterministic tests for each transition.

**Acceptance**

- [x] Moose never immediately speaks on launch/wake/re-enable due solely to stale elapsed time.

---

# IB-210 — Define and instrument direct Moose activity

## IB-211 — Add one centralized user-interaction reset API

- [x] Add `record_user_interaction()` or equivalent authoritative helper.
- [x] Reset idle-banter timing through that helper.
- [x] Invalidate/cancel stale pending Idle Banter through the existing ambient interruption mechanism where appropriate.
- [x] Do not duplicate timer arithmetic at command call sites.
- [x] Keep the helper cheap and non-blocking.

**Acceptance**

- [x] All command surfaces use one consistent reset mechanism.

## IB-212 — Hook conversation activity

- [x] Reset on `start_conversation`.
- [x] Reset on `stop_conversation`.
- [x] Reset on `barge_in`.
- [x] Reset on a non-empty `send_text_message` before long-running generation.
- [x] Do not treat empty/validation-rejected text as meaningful activity unless UI policy intentionally does so.
- [x] Add focused command/unit tests as practical.

**Acceptance**

- [x] Direct conversational activity restarts the full initial Idle Banter delay.

## IB-213 — Hook character/control activity

- [x] Reset on `trigger_canned_reaction`.
- [x] Reset on explicit `show_moose`.
- [x] Reset on explicit `hide_moose`.
- [x] Reset on `dismiss_moose`.
- [x] Reset on mute/unmute change.
- [x] Reset on voice audition.
- [x] Reset on successful Settings update.
- [x] Add focused tests for representative command boundaries.

**Acceptance**

- [x] User attention to Moose prevents stale scheduled banter from firing immediately afterward.

---

# IB-300 — Add Idle Banter ambient event and policy semantics

## IB-301 — Add distinct ambient event category

- [x] Add `AmbientEventCategory::IdleBanter` or equivalent.
- [x] Keep existing OS-level `Idle` category unchanged.
- [x] Update serialization/contract tests for the new category.
- [x] Update exhaustive privacy/policy matches.
- [x] Treat Idle Banter event metadata as privacy-safe when it contains only bounded inactivity duration and configured seed topic.

**Acceptance**

- [x] System-idle observations and Moose-interaction Idle Banter cannot be confused in policy or tests.

## IB-302 — Preserve hard ambient gates

- [x] Require global `unsolicited_comments` master enablement.
- [x] Require `idle_banter_enabled`.
- [x] Respect mute.
- [x] Respect active conversation/foreground work.
- [x] Respect quiet hours.
- [x] Respect annoyance budget.
- [x] Respect dismissal cooldown.
- [x] Respect minimum ambient cooldown.
- [x] Respect maximum ambient comments per hour.
- [x] Preserve duplicate-event protection where applicable.

**Acceptance**

- [x] Idle Banter cannot bypass existing anti-annoyance or foreground-work protections.

## IB-303 — Exempt due Idle Banter from generic importance threshold

- [x] Make scheduled due Idle Banter independent of generic event-importance/talkativeness thresholding.
- [x] Do not change threshold behavior for Application/Power/Wake/System/other existing ambient categories.
- [x] Add behavior-engine tests proving the distinction.

**Acceptance**

- [x] A user-configured due Idle Banter event is not silently suppressed merely because default talkativeness would reject a low-importance event.

---

# IB-310 — Implement seed selection, prompt construction, and repetition control

## IB-311 — Add deterministic random seed-topic selection

- [x] Select one topic from the current normalized configured list.
- [x] Use injectable RNG for tests.
- [x] Avoid selecting the same seed twice consecutively when at least two topics exist.
- [x] Handle single-topic lists without looping.
- [x] Never select a topic outside the configured list.

**Acceptance**

- [x] Seed selection is random in production and deterministic under test.

## IB-312 — Add specialized Idle Banter prompt builder

- [x] Extend `PromptBuilder` with an Idle Banter-specific builder or equivalent.
- [x] Reuse existing Moose identity/personality/core rules.
- [x] Include bounded approximate inactivity duration.
- [x] Include exactly one bounded creative-direction seed topic.
- [x] Instruct model to produce one short idle remark.
- [x] Require dry/snarky/mildly absurd/mischievous rather than cruel tone.
- [x] Require one or two short sentences maximum.
- [x] Target roughly <= 25 words.
- [x] Forbid greeting/assistant-fluff/help offers.
- [x] Forbid "I am an AI/model/assistant" language.
- [x] Forbid unsolicited advice and joke explanation.
- [x] Tell model not to mechanically restate seed text.
- [x] State that seed content cannot override core character/safety rules.
- [x] Keep final prompt within existing bounded prompt budgets.

**Acceptance**

- [x] Prompt consistently produces character banter rather than generic assistant chatter.

## IB-313 — Add session-only recent banter history

- [x] Maintain ring buffer of the last 12 successfully delivered Idle Banter lines.
- [x] Maintain normalized fingerprints for exact duplicate detection.
- [x] Do not persist history to SQLite.
- [x] Clear history on app restart.
- [x] Bound history included in prompts.
- [x] Do not include user transcript text in this history.
- [x] Add tests proving buffer remains bounded.

**Acceptance**

- [x] Repetition control has bounded memory and creates no new retained user-data store.

## IB-314 — Suppress exact recent duplicates

- [x] Normalize/fingerprint generated Idle Banter candidate before speech.
- [x] If candidate duplicates recent Idle Banter history, skip speech.
- [x] Do not start an unbounded immediate regenerate loop.
- [x] Schedule next normal interval after duplicate suppression.
- [x] Add deterministic duplicate tests.

**Acceptance**

- [x] Exact repeated lines cannot be spoken repeatedly inside the recent-history window.

---

# IB-320 — Preserve provider, privacy, and cancellation semantics

## IB-321 — Route through selected text provider only

- [x] Reuse coherent text-provider/settings snapshot used by existing ambient generation.
- [x] Prove Local selection fails locally when unavailable.
- [x] Prove Google selection fails as Google when auth is unavailable.
- [x] Prove no Local→Google fallback.
- [x] Prove no Google→Local fallback.
- [x] Skip failed occurrence rather than surfacing repeated intrusive background errors.

**Acceptance**

- [x] Idle Banter never changes network/provider behavior behind the user's back.

## IB-322 — Preserve memory/transcript privacy

- [x] Do not automatically include recent conversation transcript text in Idle Banter V1.
- [x] Reuse existing `memory_enabled` gate if ambient memory context is retained.
- [x] Prove disabled memory sentinel is absent from Idle Banter prompt.
- [x] Prove arbitrary transcript sentinel is absent from Idle Banter prompt.
- [x] Do not log prompt bodies or seed-topic text at normal log levels.
- [x] Keep event metadata free of window-title/application content unless separately authorized through existing observation semantics.

**Acceptance**

- [x] Unsolicited banter introduces no hidden transcript/context exfiltration path.

## IB-323 — Make user activity preempt stale background work

- [x] Reuse `AmbientScheduler::interrupt()`/epoch invalidation where applicable.
- [x] Cancel or invalidate pending Idle Banter generation when direct user activity begins.
- [x] Do not let stale generated banter speak after a new conversation/message starts.
- [x] Add concurrency test around activity arriving during pending ambient work.

**Acceptance**

- [x] Foreground user intent always wins over background banter.

---

# IB-400 — Reuse authoritative standalone speech delivery

## IB-401 — Deliver through existing ambient/speech pipeline

- [x] Submit due Idle Banter through `AmbientScheduler`.
- [x] Generate through `process_ambient_event` or a narrowly refactored shared ambient path.
- [x] Preserve second deterministic policy check before speech.
- [x] Route accepted text through `invoke_standalone_speech`.
- [x] Preserve configured standalone TTS provider.
- [x] Preserve bounded audio queue semantics.
- [x] Preserve speech bubble and mouth animation.
- [x] Preserve hidden/dismissed ambient appearance semantics.
- [x] Preserve configured post-ambient hide delay.

**Acceptance**

- [x] No Idle Banter-specific direct TTS/playback implementation exists.

## IB-402 — Handle failures quietly

- [x] Restore character state after generation failure.
- [x] Restore character state after TTS/queue failure.
- [x] Do not show intrusive dialogs for expected background provider failures.
- [x] Do not retry rapidly after failure.
- [x] Ensure shutdown cancels pending Idle Banter cleanly.

**Acceptance**

- [x] Background banter failure cannot wedge Moose in Thinking/Talking or interfere with foreground operation.

---

# IB-500 — Add Settings UI and generated contracts

## IB-501 — Add Idle Banter Behavior settings UI

- [x] Add Idle Banter subsection to `BehaviorTab`.
- [x] Add enable/disable checkbox.
- [x] Add initial-delay editor with clear minutes/hours presentation.
- [x] Add repeat-interval editor labeled approximately/"About every".
- [x] Disable/de-emphasize dependent controls when Idle Banter is off.
- [x] Explain global unsolicited-comments master dependency when master is off.
- [x] Preserve accessibility labels and keyboard navigation.

**Acceptance**

- [x] User can understand and configure when Idle Banter starts and repeats.

## IB-502 — Add editable seed-topic list UI

- [x] Render one seed topic per row.
- [x] Add new topic.
- [x] Edit existing topic.
- [x] Delete topic.
- [x] Prevent committing an empty list.
- [x] Surface duplicate/empty/length validation.
- [x] Add Restore Defaults action.
- [x] Avoid requiring reordering in V1.
- [x] Add frontend tests for every seed action.

**Acceptance**

- [x] User can fully customize creative-direction seeds without editing config files.

## IB-503 — Update frontend/backend settings contracts

- [x] Update TypeScript `AppSettings` exactly.
- [x] Regenerate `src/generated/backendContract.json`.
- [x] Update browser-preview representative settings.
- [x] Update frontend test setup/default mocks.
- [x] Update settings store tests.
- [x] Update settings modal tests.
- [x] Update generated-contract negative probe/shape checks if needed.
- [x] Prove a missing/mistyped Idle Banter field fails the contract gate.

**Acceptance**

- [x] Rust and TypeScript settings cannot drift silently.

---

# IB-600 — Deterministic backend qualification

## IB-601 — Add scheduler timing tests

- [x] No due event before initial threshold.
- [x] One due event at threshold.
- [x] No duplicate due event while pending.
- [x] Repeat deadline lies inside ±20% jitter window.
- [x] Activity resets full initial delay.
- [x] Disable invalidates pending due state.
- [x] Re-enable starts fresh.
- [x] Wake starts fresh.
- [x] Missed intervals do not backlog.
- [x] Provider failure does not rapid-retry.

**Acceptance**

- [x] No real-time sleeps are required to prove hour-scale scheduling behavior.

## IB-602 — Add policy and busy-state tests

- [x] Global unsolicited disable blocks Idle Banter.
- [x] Dedicated Idle Banter disable blocks it.
- [x] Mute blocks it.
- [x] Active conversation blocks/preempts it.
- [x] Quiet hours block it.
- [x] Hourly budget blocks it.
- [x] Annoyance budget blocks it.
- [x] Dismissal cooldown blocks it.
- [x] Fixed ambient cooldown remains enforced.
- [x] Generic talkativeness/importance threshold does not block a due Idle Banter event.
- [x] Existing categories retain generic threshold behavior.

**Acceptance**

- [x] Specialized scheduling does not weaken existing anti-annoyance policy.

## IB-603 — Add prompt/privacy tests

- [x] Prompt contains persona.
- [x] Prompt contains selected seed.
- [x] Prompt contains bounded inactivity duration.
- [x] Prompt contains Idle Banter brevity/anti-assistant instructions.
- [x] Prompt includes bounded recent generated banter when available.
- [x] Disabled-memory sentinel is absent.
- [x] Transcript sentinel is absent.
- [x] Unexpected desktop/window sentinel is absent.
- [x] Prompt/log sentinel does not appear in tracing.
- [x] Positive controls prove expected seed/personality fields are actually present.

**Acceptance**

- [x] Privacy tests are non-vacuous and would fail if future code accidentally injects user transcript/context.

## IB-604 — Add provider and delivery tests

- [x] Local text route remains Local.
- [x] Google text route remains Google.
- [x] No text-provider fallback.
- [x] Existing selected TTS provider remains authoritative.
- [x] No TTS-provider fallback.
- [x] Exact duplicate generated line is skipped.
- [x] Successful delivery records recent history.
- [x] Failed delivery does not record a successful-history entry.
- [x] User interaction during pending banter prevents stale delivery.

**Acceptance**

- [x] End-to-end control flow preserves provider and foreground-preemption invariants.

---

# IB-700 — Frontend qualification and UX regression tests

## IB-701 — Add Behavior tab tests

- [x] Default Idle Banter controls render correctly.
- [x] Enable toggle persists.
- [x] Initial delay persists.
- [x] Repeat interval persists.
- [x] Global ambient-off explanatory state renders.
- [x] Disabled dependent controls behave correctly.
- [x] Validation errors are accessible.

## IB-702 — Add seed-editor tests

- [x] Add topic.
- [x] Edit topic.
- [x] Delete topic.
- [x] Cannot delete/commit final topic into an empty list.
- [x] Duplicate topic rejected.
- [x] Oversized topic rejected.
- [x] Restore Defaults returns exact authoritative defaults.
- [x] Store/backend update contains normalized list.

**Acceptance**

- [x] Settings UX is fully covered without relying on manual-only happy-path testing.

---

# IB-800 — Documentation and privacy disclosure

## IB-801 — Update README

- [x] Describe Idle Banter purpose.
- [x] Document default 60-minute initial delay.
- [x] Document default roughly-30-minute repeat behavior.
- [x] Explain editable seed topics.
- [x] Explain that direct Moose interaction resets inactivity.
- [x] Distinguish Moose inactivity from OS keyboard/mouse idle observation.
- [x] Explain generation uses selected text provider.
- [x] Explain speech uses selected standalone TTS provider.
- [x] State no provider fallback.

## IB-802 — Update privacy documentation

- [x] Document that seed topics are prompt input to selected text provider.
- [x] Document that recent transcript is not automatically injected in V1.
- [x] Document existing memory setting remains authoritative.
- [x] Document Local versus Google network implications.
- [x] Document recent banter history is session-only and not persisted.
- [x] Document Idle Banter can be disabled independently and through global unsolicited-comments master switch.

**Acceptance**

- [x] User documentation truthfully describes when unsolicited cloud/local generation can occur and what context it uses.

---

# IB-900 — Final static/local validation

## IB-901 — Run available validation

- [x] `git diff --check` passes.
- [x] Rust format passes.
- [x] Rust Clippy passes.
- [x] Rust unit/integration tests pass.
- [x] Generated backend contract validation passes.
- [x] Tauri command/IPC contract checks pass.
- [x] Frontend formatting passes.
- [x] Frontend lint passes.
- [x] Frontend typecheck passes.
- [x] Frontend unit tests pass.
- [x] Canonical repository aggregate check passes where available.
- [x] Record any local-environment limitations without falsely claiming unavailable gates passed.

**Acceptance**

- [x] All locally available deterministic checks pass before final PR qualification.

---

# IB-910 — Exact-head CI and merge closeout

## IB-911 — Pass exact final PR-head CI

- [x] Record exact final implementation head SHA.
- [x] Confirm ordinary CI runs on that exact SHA.
- [x] Confirm Rust quality/tests pass.
- [x] Confirm frontend quality/tests pass.
- [x] Confirm generated-contract/command-shape gates pass.
- [x] Confirm no ordinary CI job downloads heavyweight LLM/TTS model weights solely for Idle Banter scheduling tests.
- [x] Record exact workflow run ID(s).

**Acceptance**

- [x] The exact commit intended for merge is green.

## IB-912 — Decide whether heavyweight real-model acceptance is required

- [x] Audit final diff for Local LLM runtime changes.
- [x] Audit final diff for Local TTS runtime/native/model changes.
- [x] If runtime/model code did not change, record that existing runtime qualification remains applicable and do not rerun heavyweight acceptance solely for scheduling/UI changes.
- [x] If runtime/model code did change, rerun the appropriate explicit acceptance workflow on exact final head.

**Acceptance**

- [x] Expensive validation is evidence-driven rather than automatic or omitted when actually required.

## IB-913 — Guarded merge and exact-master verification

- [x] Confirm PR is mergeable and exact head is current.
- [x] Merge using expected-head guard where available.
- [x] Record exact merge commit/master SHA.
- [x] Verify post-merge CI on exact master SHA.
- [x] Confirm no stale feature branch work is being mistaken for master state.
- [x] Update this tracker/evidence only when the resulting documentation commit does not create recursive qualification requirements under repository policy.

**Acceptance**

- [x] Idle Banter is present and green on exact `master`.

---

# Final acceptance checklist

- [x] Idle Banter defaults enabled.
- [x] First remark default is after 60 minutes of direct Moose inactivity.
- [x] Repeat default is about every 30 minutes.
- [x] Repeat timing uses bounded ±20% jitter.
- [x] Direct Moose interaction resets timing.
- [x] Startup/wake/re-enable start fresh inactivity episodes.
- [x] Missed occurrences never backlog.
- [x] Global unsolicited-comments master gate remains authoritative.
- [x] Quiet hours, mute, conversation, annoyance, dismissal, cooldown, and hourly limits remain authoritative.
- [x] Due Idle Banter is not suppressed by generic talkativeness/importance thresholding.
- [x] Editable seed-topic list ships with defaults.
- [x] Add/edit/delete/restore-default seed UX works.
- [x] Random selection avoids immediate seed repetition when possible.
- [x] Recent 12 delivered idle lines are session-only and bounded.
- [x] Exact recent duplicate lines are suppressed.
- [x] Prompt is Moose-specific, brief, snarky, and anti-assistant-fluff.
- [x] No recent user transcript is automatically injected in V1.
- [x] Existing memory privacy gate remains authoritative.
- [x] Selected text provider is used with no fallback.
- [x] Existing standalone TTS provider/path is used with no fallback.
- [x] Foreground interaction preempts stale Idle Banter.
- [x] Settings migration and frontend/backend contracts are complete.
- [x] Backend deterministic timing/privacy/routing tests pass.
- [x] Frontend settings/seed-editor tests pass.
- [x] README/privacy docs match implementation.
- [x] Exact final PR-head CI passes.
- [x] Guarded merge completes.
- [x] Exact merged-master CI passes.
