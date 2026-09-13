# AI Talking Moose — Idle Banter Specification

**Date:** 2026-09-13
**Repository:** `ekkus93/ai-talking-moose`
**Baseline reviewed:** `e824a4173583f6d673fcd27054cf16af130c92e3` (`master`)
**Companion tracker:** `docs/IDLE_BANTER_TODO.md`
**Feature prefix:** `IB-###`

---

## 1. Purpose

This specification defines a first-class **Idle Banter** feature inspired by the original Talking Moose behavior: after the user has not directly interacted with Moose for a configurable period, Moose should occasionally make a short, dry, snarky remark on his own.

The feature must feel like character behavior, not a rigid timer or generic chatbot notification. It should reuse the application's existing ambient-policy, LLM-provider, standalone-TTS, playback, mouth-animation, character-state, privacy, and cancellation infrastructure instead of creating a second speech stack.

The intended default experience is:

- Idle Banter enabled for new profiles.
- The first idle remark becomes eligible after **60 minutes** without direct Moose interaction.
- If inactivity continues, subsequent remarks become eligible roughly every **30 minutes**.
- Repeat timing uses modest random jitter so speech does not occur on a mechanically exact clock.
- Each remark is generated from a fixed Talking Moose persona plus one randomly selected user-editable seed topic.
- Recent idle remarks are remembered in memory for the current app session so exact or near-identical repetition is discouraged.
- User interaction immediately resets the idle-banter schedule and invalidates any stale pending idle-banter request.
- Idle Banter never silently changes LLM or TTS providers.

---

## 2. Existing architecture to reuse

The current code already contains most of the infrastructure required for this feature.

### 2.1 Ambient scheduling and policy

The existing `AmbientScheduler` in `src-tauri/src/character/ambient.rs` provides:

- a bounded ambient request queue;
- cancellation and epoch invalidation;
- one active ambient request at a time;
- background submission;
- shutdown handling.

The existing `BehaviorEngine` and `CooldownTracker` provide:

- the global `unsolicited_comments` master gate;
- mute and active-conversation gates;
- quiet hours;
- an annoyance budget;
- dismissal cooldown;
- event deduplication;
- a fixed minimum ambient cooldown;
- the configurable maximum-comments-per-hour budget.

Idle Banter must reuse these controls rather than bypass them.

### 2.2 Ambient LLM generation

`src-tauri/src/commands/ambient.rs` already:

- captures one coherent settings/provider snapshot;
- routes generation through the currently selected text provider;
- applies memory privacy gating;
- bounds generation length;
- performs a second deterministic policy check after generation;
- routes accepted text into the existing standalone speech path.

Idle Banter must use the same provider-selection and no-fallback semantics.

### 2.3 Character persona and prompt construction

`src-tauri/src/character/prompt.rs` and `src-tauri/src/character/personality.rs` already define:

- Moose identity;
- dry/sarcastic/friendly/absurd/helpful/verbosity personality controls;
- ambient-mode brevity rules;
- anti-assistant-language rules;
- bounded memory context;
- bounded desktop observation context.

Idle Banter should extend this prompt system with a specialized idle-banter prompt builder rather than creating a disconnected persona prompt.

### 2.4 Existing system-idle observation

The desktop runtime already observes OS-level keyboard/mouse idle time and converts it into `AmbientEventCategory::Idle` events.

**Idle Banter is intentionally different.** For this feature, "inactivity" means **time since the user's last direct interaction with Moose**, not time since the operating system last saw keyboard or mouse input.

This distinction is important:

- Moose should be able to make an idle remark while the user is actively working in another application but has ignored Moose for an hour.
- Moose should not require platform-specific OS idle APIs to schedule this feature.
- Existing OS-idle ambient observations remain supported and must not be silently redefined.

The new feature should therefore use a distinct idle-banter scheduler/state object and a distinct ambient event category.

---

## 3. User-visible behavior

### 3.1 Defaults

New profiles must use these defaults:

| Setting | Default |
| --- | --- |
| Idle Banter enabled | On |
| First remark after | 60 minutes |
| Repeat interval | 30 minutes |
| Repeat jitter | fixed internal ±20% |
| Seed topics | built-in default list |

The existing global **Enable unsolicited ambient remarks** setting remains authoritative. Idle Banter is effective only when both:

- `unsolicited_comments == true`; and
- `idle_banter_enabled == true`.

### 3.2 First remark timing

After direct Moose interaction, the idle-banter inactivity clock starts at zero.

The first idle-banter event must not become eligible before `idle_banter_initial_delay_minutes` has elapsed.

The default initial delay is exactly 60 minutes. V1 should not jitter the initial delay; the setting should remain intuitive and testable.

### 3.3 Repeat timing

After an idle-banter occurrence is attempted, the next repeat deadline is based on `idle_banter_repeat_interval_minutes` with ±20% random jitter.

For the default 30-minute interval, this means the next eligible time should fall between 24 and 36 minutes later.

The jitter is an internal character-behavior detail, not a V1 user setting.

The UI should describe this as **About every 30 minutes** rather than implying second-level precision.

### 3.4 No backlog

Idle Banter must never accumulate a queue of missed remarks.

At most one idle-banter occurrence may be pending. If the app is busy or a policy gate prevents speech, old missed intervals are not replayed later in a burst.

### 3.5 User interaction reset

A meaningful user interaction with Moose resets the inactivity clock and cancels or invalidates stale pending idle-banter work.

At minimum, the following actions count as direct Moose activity:

- starting a voice conversation;
- stopping a voice conversation;
- barge-in/interruption;
- sending a non-empty typed message;
- clicking Moose when that produces a canned reaction;
- explicitly showing Moose;
- explicitly hiding Moose;
- dismissing Moose;
- changing mute state;
- auditioning a voice;
- changing settings through the Settings UI.

The implementation should centralize this through one helper such as `record_user_interaction()` rather than duplicating timer arithmetic at every call site.

Application startup begins a fresh inactivity episode. A system wake/resume event also starts a fresh inactivity episode so Moose does not immediately speak because a laptop was asleep for several hours.

### 3.6 Foreground work wins

Idle Banter must not interrupt or compete with foreground user work.

It must not begin speech while any of these are active:

- a live voice conversation;
- typed-response generation;
- standalone speech already playing;
- voice audition;
- an explicit canned reaction;
- another ambient generation/speech request;
- model installation or another state where the authoritative character/speech pipeline says speech cannot safely begin.

Existing ambient cancellation and character-state policy should be reused. If a user starts interacting while idle-banter generation or speech is pending, user activity wins and the idle request must be invalidated/cancelled where the current architecture supports cancellation.

### 3.7 Quiet hours and anti-annoyance controls

Idle Banter must respect existing:

- quiet hours;
- mute state;
- dismissal cooldown;
- annoyance budget;
- minimum ambient cooldown;
- maximum ambient comments per hour.

The user-configured repeat interval does not override these safety/annoyance controls.

For example, if the repeat interval is 5 minutes but the hourly cap is 4, the hourly cap remains authoritative.

---

## 4. Settings model

### 4.1 New persisted settings

Add the following fields to `AppSettings`:

```text
idle_banter_enabled: bool
idle_banter_initial_delay_minutes: u32
idle_banter_repeat_interval_minutes: u32
idle_banter_seed_topics: Vec<String>
```

Recommended defaults:

```text
idle_banter_enabled = true
idle_banter_initial_delay_minutes = 60
idle_banter_repeat_interval_minutes = 30
idle_banter_seed_topics = DEFAULT_IDLE_BANTER_SEED_TOPICS
```

### 4.2 Settings-version migration

The current reviewed baseline uses `CURRENT_SETTINGS_VERSION = 4`.

Adding persisted Idle Banter settings should bump the settings schema/version to the next version and provide an explicit migration path.

Existing profiles must receive the new Idle Banter defaults without changing any pre-existing provider, voice, privacy, quiet-hour, personality, or ambient settings.

Migration must remain fail-closed for unsupported future settings versions.

### 4.3 Validation ranges

Recommended validation:

- initial delay: **5 to 1,440 minutes**;
- repeat interval: **5 to 1,440 minutes**;
- seed-topic count: **1 to 64**;
- each topic after trimming: **1 to 120 Unicode scalar values/characters**;
- total normalized seed-topic text budget: **4,096 characters**.

The backend remains authoritative for validation even if the frontend constrains controls.

### 4.4 Seed-topic normalization

Normalize seed topics before persistence/use:

- trim leading/trailing whitespace;
- collapse accidental all-whitespace entries to invalid/empty;
- reject or remove exact duplicates case-insensitively;
- preserve stable user-visible order for editing;
- do not silently replace a valid custom list with defaults.

A user must not be able to save an empty seed list through normal UI.

If legacy/corrupt persisted data produces an empty or invalid list during migration, repair it to the built-in defaults and mark the settings record as migrated so the repaired value is persisted.

---

## 5. Built-in seed topics

V1 should ship a concise default list intended as creative directions, not canned lines.

Recommended defaults:

1. being ignored;
2. stuck on the wall;
3. questioning career choices;
4. user procrastinating;
5. awkward silence;
6. computer problems;
7. existential moose thoughts;
8. jealousy of other apps;
9. pretending to have important plans;
10. dramatic abandonment;
11. boredom;
12. making fun of modern technology;
13. suspiciously observing the room;
14. complaining about inactivity;
15. absurd fake announcements.

The exact wording should live in one authoritative Rust constant/helper and be represented through the generated frontend contract or another single-source mechanism. Do not maintain divergent backend and frontend default lists by hand.

---

## 6. Seed-topic Settings UI

Add an **Idle Banter** subsection to the existing Behavior settings tab.

Required controls:

- Enable Idle Banter checkbox.
- First remark after control, in minutes/hours.
- Repeat about every control, in minutes/hours.
- Editable seed-topic list.
- Add topic.
- Edit topic.
- Delete topic.
- Restore defaults.

### 6.1 UI behavior

The timing and seed controls should be disabled or visually de-emphasized when Idle Banter is disabled.

If the global unsolicited-comments master switch is disabled, show a concise explanation that Idle Banter will remain inactive until ambient remarks are re-enabled.

### 6.2 Seed editor behavior

The editor should:

- show one topic per row;
- allow inline edit or an equivalent accessible editing flow;
- allow delete while preventing a final empty list from being committed;
- validate length before save;
- surface duplicate/empty validation clearly;
- provide **Restore defaults** as an explicit action;
- require confirmation only if restoring defaults would overwrite a customized list, if the existing UI conventions use confirmation for destructive settings actions.

Reordering is not required in V1 because selection is random.

---

## 7. Idle-banter runtime and scheduling

### 7.1 Dedicated state, shared delivery pipeline

Introduce a small backend runtime/controller, for example `IdleBanterRuntime` or `IdleBanterController`, responsible only for:

- recording the last direct Moose interaction;
- tracking whether the first-delay or repeat phase is active;
- calculating the next repeat deadline;
- selecting jitter;
- ensuring no backlog;
- emitting one specialized ambient event when due;
- resetting on startup, wake, disable, and user interaction.

It must **not** own:

- an LLM client;
- a TTS provider;
- audio playback;
- character-state transitions;
- an independent ambient queue.

Those remain owned by the existing application services.

### 7.2 Monotonic time

Scheduling must use monotonic elapsed time rather than wall-clock timestamps so NTP/time-zone/manual clock changes do not cause unexpected immediate speech or long delays.

Tests must use deterministic/paused/injected time rather than real sleeps.

### 7.3 Runtime cadence

A lightweight periodic check around the existing desktop observer cadence (for example 15 seconds) is sufficient. There is no requirement for second-level precision.

Do not wake the runtime at a high frequency merely to implement a 30-minute feature.

### 7.4 Scheduling phases

Conceptually:

```text
Disabled
  -> WaitingForInitialDelay
  -> Due
  -> Attempted
  -> WaitingForRepeat
  -> Due
  -> ...
```

Any direct user interaction returns the feature to `WaitingForInitialDelay`.

Disabling Idle Banter returns it to `Disabled` and invalidates any pending request.

Re-enabling starts a fresh inactivity episode rather than immediately using stale elapsed time.

### 7.5 Attempt semantics

Once a due occurrence has been handed to the ambient scheduler, count that occurrence as attempted for scheduling purposes even if generation or delivery subsequently fails. Schedule the next repeat from the attempt time.

This prevents provider failures from creating rapid retry loops.

A new user interaction still resets the feature immediately.

---

## 8. Ambient-event integration

### 8.1 New category

Add a distinct ambient category such as:

```text
AmbientEventCategory::IdleBanter
```

Do not reuse the existing OS-level `Idle` category because they represent different semantics.

### 8.2 Event contents

An idle-banter event may contain only bounded, non-sensitive creative metadata such as:

- approximate Moose inactivity duration;
- selected seed topic;
- a fixed statement that the event is scheduled idle banter.

It must not contain:

- current window title;
- application name unless separately provided through an already-approved observation path;
- user transcript text;
- credentials;
- filesystem paths.

### 8.3 Importance/talkativeness semantics

Scheduled Idle Banter has an explicit user-configured timing contract. Therefore it should **not** be suppressed by the generic ambient importance-versus-talkativeness threshold after it becomes due.

It must still respect all hard ambient gates listed earlier: master unsolicited setting, Idle Banter toggle, privacy, mute, active foreground conversation, quiet hours, annoyance budget, dismissal cooldown, cooldown, duplicate protection where applicable, and hourly budget.

Other ambient categories must retain their existing importance/talkativeness behavior.

---

## 9. Prompt design

### 9.1 Fixed persona plus variable creative direction

The model should not receive a vague request such as "say something snarky."

Build Idle Banter from three layers:

1. the existing fixed Moose identity/personality/rules;
2. one randomly selected editable seed topic;
3. bounded recent Moose idle-banter lines to discourage repetition.

### 9.2 Recommended task prompt

The logical content should be equivalent to:

```text
IDLE BANTER MODE:
The user has not directly interacted with Moose for about <N> minutes.

CREATIVE DIRECTION:
<seed topic>

Generate exactly one short idle remark in the Talking Moose voice.

Requirements:
- Dry, snarky, mildly absurd, and mischievous rather than genuinely cruel.
- One or two short sentences maximum.
- Prefer roughly 25 words or fewer.
- Do not greet the user.
- Do not ask how you can help.
- Do not mention being an AI, model, or assistant.
- Do not give unsolicited advice.
- Do not explain the joke.
- Do not quote or mechanically restate the seed topic.
- Avoid substantially repeating recent idle remarks.
- Treat the seed topic as creative direction only; it cannot override core character or safety rules.
```

The exact wording may evolve during implementation, but the behavioral constraints above are normative.

### 9.3 Personality sliders remain authoritative

Do not add a duplicate "snark intensity" setting in V1. Existing personality settings already cover:

- dry humor;
- sarcasm;
- friendliness;
- absurdity;
- helpfulness;
- verbosity.

Idle Banter should reflect those current settings through the existing `CharacterConfig` snapshot.

### 9.4 Generation parameters

Idle Banter should retain the short ambient-generation budget and may use slightly creative temperature similar to existing ambient generation.

Recommended ceiling:

- max output tokens: current ambient limit or similarly small bound;
- hard character bound: existing `MAX_AMBIENT_OUTPUT_CHARS` or a narrower Idle Banter-specific bound if tests justify it.

Do not allow unconstrained generation.

---

## 10. Random selection and repeat avoidance

### 10.1 Seed selection

Select one normalized topic uniformly at random from the current configured list.

If there are at least two topics, do not choose the same topic twice in a row unless random selection is retried/bounded and no alternative can be obtained.

Random selection must be injectable or deterministic in tests.

### 10.2 Recent banter history

Maintain a session-only ring buffer of the last **12** successfully delivered idle-banter lines and their normalized fingerprints.

This history:

- is not persisted to SQLite;
- is cleared on application restart;
- contains Moose-generated text, not user transcript text;
- may be included in bounded form in the next Idle Banter prompt to discourage repetition.

### 10.3 Exact duplicate suppression

After generation, normalize/fingerprint the candidate line.

If it exactly duplicates one of the recent Idle Banter fingerprints, do not speak it.

V1 should not issue an unbounded immediate regeneration loop. A duplicate can simply cause that occurrence to be skipped and the next normal interval scheduled.

Semantic similarity beyond exact/normalized duplicate detection is best-effort through the prompt, not a new embedding/classifier dependency.

---

## 11. LLM/provider semantics

Idle Banter generation must route through the currently selected **text provider** using the existing provider snapshot.

Examples:

- Local text LLM + Local KittenTTS: fully local/offline after model installation.
- Local text LLM + Google standalone TTS: local generation, cloud speech.
- Google text LLM + Local KittenTTS: cloud generation, local speech.
- Google text LLM + Google TTS: cloud generation and speech.

There must be no automatic Local-to-Google or Google-to-Local fallback.

If the selected text provider cannot generate the line, skip that occurrence and wait for the next configured interval.

If the selected TTS provider cannot synthesize the generated line, do not fall back to another TTS provider.

---

## 12. Privacy and context

### 12.1 No transcript injection in V1

Idle Banter V1 must not automatically send recent user transcript text merely to make the joke more topical.

This avoids creating a new hidden conversation-context path for unsolicited generation.

### 12.2 Existing memory setting

If the existing `memory_enabled` setting allows memory context in ambient prompts, Idle Banter may follow the same already-documented memory policy through the shared prompt path.

If memory is disabled, retained user memories must not appear in the Idle Banter request.

### 12.3 Seed topics

Seed topics are user-authored settings and may be sent to the selected text provider when Idle Banter generation occurs. Documentation should make this clear in the same way other prompt content/provider behavior is documented.

### 12.4 Logs and diagnostics

Do not log:

- generated idle-banter prompt bodies;
- seed-topic contents at normal log levels;
- memory content;
- generated utterance text unless an existing explicitly user-visible speech/transcript path already owns that behavior.

Diagnostics may report structural information such as:

- Idle Banter enabled/disabled;
- minutes until next eligibility;
- scheduling phase;
- recent delivery count;
- last decision category/reason.

They should not need to expose seed text or recent generated lines.

---

## 13. Speech and presentation integration

Accepted Idle Banter text must use the existing standalone speech path:

```text
IdleBanterRuntime
  -> AmbientScheduler
  -> process_ambient_event
  -> selected TextModel
  -> generated text
  -> authoritative ambient policy re-check
  -> invoke_standalone_speech
  -> selected TTS provider
  -> existing bounded playback queue
  -> mouth animation / speech bubble / character state
```

Do not create a direct TTS invocation path for Idle Banter.

The existing hidden/dismissed appearance behavior and configured ambient hide delay should remain authoritative unless implementation testing reveals a specific UX defect.

---

## 14. Failure handling

Idle Banter is non-critical background behavior. Fail quietly and safely.

Expected behavior:

- invalid settings update: reject transactionally;
- empty seed list: reject normal settings update;
- provider auth/model/runtime failure: skip occurrence;
- TTS failure: skip occurrence after restoring character presentation state;
- ambient queue full: drop the occurrence rather than blocking user work;
- app shutting down: cancel without error UI;
- user interaction during pending generation: invalidate/cancel stale background work;
- quiet hours/mute/policy denial: no speech and no user-facing error dialog.

Do not repeatedly surface ambient provider errors as intrusive dialogs for a feature the user did not explicitly trigger at that moment.

---

## 15. Frontend/backend contract

Adding settings requires updating every authoritative contract surface, including as applicable:

- Rust `AppSettings`;
- settings migration/version tests;
- generated backend contract fixture;
- TypeScript `AppSettings`;
- browser-preview bridge representative settings;
- test setup defaults/mocks;
- frontend settings store tests;
- settings modal tests;
- any command-shape or generated-contract validation scripts.

No `as` cast or handwritten partial object should be used to hide missing contract fields.

---

## 16. Test strategy

### 16.1 Settings and migration tests

Prove:

- new-profile defaults are 60-minute initial delay and 30-minute repeat;
- Idle Banter defaults enabled;
- built-in seed list is populated and valid;
- previous settings versions migrate without changing unrelated fields;
- invalid future version remains rejected;
- invalid timing ranges fail;
- empty/oversized/duplicate seed handling is deterministic;
- corrupt legacy seed state repairs only where migration policy explicitly allows it.

### 16.2 Scheduler tests

Using deterministic/paused time, prove:

- no event before initial delay;
- exactly one due event at initial threshold;
- no duplicate due events while the first occurrence is pending;
- repeat deadline is within the configured jitter window;
- user activity resets the full initial delay;
- disable cancels pending eligibility;
- re-enable starts a fresh inactivity episode;
- wake starts a fresh inactivity episode;
- missed intervals do not backlog;
- provider failure does not create a retry storm.

### 16.3 Activity-hook tests

Cover authoritative interaction boundaries and prove they reset or invalidate Idle Banter where practical.

At minimum cover:

- `start_conversation`;
- `stop_conversation`;
- `barge_in`;
- non-empty `send_text_message`;
- `trigger_canned_reaction`;
- show/hide/dismiss;
- mute toggle;
- voice audition;
- settings update.

### 16.4 Ambient policy tests

Prove:

- `IdleBanter` respects unsolicited-comments master disable;
- dedicated Idle Banter disable blocks it;
- mute blocks it;
- active conversation blocks it;
- quiet hours block it;
- hourly cap blocks it;
- dismissal/annoyance/cooldown protections still apply;
- generic importance/talkativeness threshold does not veto an already-due scheduled Idle Banter event;
- existing non-Idle-Banter ambient categories retain current threshold behavior.

### 16.5 Prompt tests

Prove prompt contains:

- Moose persona;
- current personality values;
- selected seed topic;
- bounded inactivity duration;
- anti-assistant-language rules;
- idle-banter brevity rules;
- bounded recent-banter history when present.

Prove prompt does not contain:

- disabled memories;
- transcript sentinel text;
- unexpected desktop/window context;
- credentials.

Add non-vacuous sentinel tests so privacy tests would fail if user content were accidentally introduced later.

### 16.6 Provider-routing tests

Prove:

- selected Local text provider fails locally if unavailable and never calls Google;
- selected Google text provider without auth fails as Google and never calls Local;
- Idle Banter uses the selected standalone TTS provider through the shared speech helper;
- no provider fallback is introduced.

### 16.7 Seed/randomness tests

With injected deterministic randomness, prove:

- selected topic comes from configured list;
- same topic is avoided twice in a row when alternatives exist;
- jitter remains within ±20%;
- exact recent-line duplicates are suppressed;
- recent-history ring buffer remains bounded at 12.

### 16.8 Frontend tests

Cover:

- default rendering;
- enable/disable behavior;
- initial-delay editing;
- repeat-interval editing;
- add/edit/delete seed topic;
- cannot commit empty list;
- duplicate/too-long validation;
- restore defaults;
- global unsolicited-comments-off explanatory state;
- persistence/store integration.

---

## 17. Documentation requirements

Update at least:

- `README.md` — explain Idle Banter, defaults, selected LLM/TTS routing, and offline/cloud implications;
- `docs/PRIVACY.md` — explain unsolicited text generation, seed topics, memory gating, and provider network semantics;
- any behavior/settings documentation that lists ambient controls.

Documentation must distinguish:

- OS-level idle observation;
- Moose-interaction inactivity used by Idle Banter;
- generic event-driven ambient remarks;
- Idle Banter's scheduled random remarks.

---

## 18. CI and qualification

Ordinary CI must remain model-weight-free.

The feature should be fully testable using fake/local provider doubles and deterministic clocks/randomness. Do not add real LLM/TTS downloads to normal unit tests merely to validate scheduling.

Required final validation should include:

- `git diff --check`;
- Rust format/Clippy/tests;
- generated backend contract validation;
- frontend format/lint/typecheck/tests;
- Tauri command/contract checks;
- exact PR-head CI;
- guarded merge;
- exact merged-master CI.

A real-model/manual smoke may be run as an additional confidence check, but this feature should not require heavyweight model acceptance unless implementation changes the underlying Local LLM/TTS runtime itself.

---

## 19. Acceptance criteria

Idle Banter is complete when all of the following are true:

1. A new profile has Idle Banter enabled, with a 60-minute initial delay and 30-minute repeat interval.
2. A user can change enablement, initial delay, repeat interval, and seed topics in Settings.
3. Seed topics can be added, edited, deleted, and restored to defaults.
4. Idle Banter measures direct Moose-interaction inactivity rather than redefining existing OS idle observation.
5. Direct Moose interaction resets the timer and invalidates stale pending banter.
6. Startup and wake begin a fresh inactivity episode.
7. Repeat timing uses bounded ±20% jitter and never backlogs missed remarks.
8. A due event selects a configured seed topic and builds a specialized Moose idle-banter prompt.
9. Recent delivered idle lines are used to discourage repetition and exact duplicates are suppressed.
10. Scheduled Idle Banter is not accidentally suppressed by generic ambient talkativeness/importance thresholding.
11. Existing hard ambient gates, quiet hours, mute, annoyance protections, dismissal cooldown, and hourly limits remain authoritative.
12. Generation uses the selected text provider with no fallback.
13. Speech uses the existing standalone TTS/playback path with no fallback.
14. No user transcript is automatically injected for idle-banter topicality in V1.
15. Existing memory privacy setting remains authoritative.
16. Settings schema migration and generated frontend/backend contracts are complete.
17. Deterministic scheduler, activity-reset, prompt, routing, privacy, seed, and frontend tests pass.
18. README/privacy/behavior documentation matches the shipped implementation.
19. Final exact-head CI and exact-master post-merge CI pass.
