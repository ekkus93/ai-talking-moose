# AI Talking Moose — Ralph Bridge Handoff (2026-09-13)

> **Historical handoff snapshot.** Do not use the voice-selection status in this dated handoff as current project state. On 2026-09-14, PR #114 added the automated ASR audition proxy and PR #115 selected `Luna` after explicit owner approval. `KCR-330` / `KTT-805` is closed for V1 by owner-approved ASR proxy evidence, without claiming subjective human listening. Use `docs/VOICE_SELECTION.md` and the current closeout/reconciliation docs for authoritative status.

## Purpose

This document is the preferred starting point for the next ChatGPT session working on `ekkus93/ai-talking-moose`.

**The next session should use Ralph Bridge first.** Do not rely on chat memory alone for repository state.

Recommended startup sequence:

1. Call `ralph_ping` through Ralph Bridge.
2. Use Ralph Bridge repository-state tooling (for example `get_repo_state`) for `ekkus93/ai-talking-moose` at `master`.
3. Confirm current `master` is at or descends from the baseline SHA recorded below.
4. Read this file, `docs/IDLE_BANTER_SPEC.md`, and `docs/IDLE_BANTER_TODO.md`.
5. Continue the remaining closeout work using Ralph Bridge for GitHub reads/writes and CI monitoring.

If Ralph Bridge is not exposed in the new chat, resolve that connector/tool availability before beginning repository mutations. This handoff is specifically intended for a Ralph Bridge session.

---

## Current repository state

Repository: `https://github.com/ekkus93/ai-talking-moose`

Default branch: `master`

Current verified master baseline before this handoff commit:

`ace6c0d8755e8f352ef3bffb8fcea7558a68727c`

That commit is documentation-only and has CI run:

- CI `34786573746`: **PASS**
- docs-only fast path: **PASS**

The last code-bearing master commit is:

`9624dc1611a59103b366b5aefdf46696d3de3def`

That is the merge commit for PR #106 and has post-merge CI:

- CI `34786184975`: **PASS**
- Rust formatting: PASS
- Clippy with `-D warnings`: PASS
- complete Rust test suite: PASS
- generated backend contract verification: PASS

The new Ralph Bridge handoff document is being committed after `ace6c0d8...`, so the next session should expect current `master` to be a descendant of that SHA rather than exactly equal to it.

---

## Idle Banter status

The Idle Banter feature is implemented, hardened, tested, merged, and qualified.

### Primary implementation

PR #105 — `feat: add configurable Idle Banter`

- final qualified PR head: `d103facdacf4cd45b73d28c0637d108f148fc631`
- PR-head CI `34778120985`: **PASS**
- PR-head KittenTTS production CPU acceptance `34778120998`: **PASS** on Linux x86_64 and macOS arm64
- merge/master commit: `c531fa4098b32a5acb37e485dd4a8b091d70dadf`
- post-merge master CI `34778513562`: **PASS**
- post-merge KittenTTS production CPU acceptance `34778513583`: **PASS on retry attempt 2** on the exact same source SHA

The first Linux attempt of `34778513583` failed during the 1/2/4-thread KittenTTS sweep after hardened inference passed. The failed Linux job was rerun without changing source and passed completely. Treat that as a transient qualification/runtime flake, not an Idle Banter regression.

### Closeout hardening

PR #106 — `test: close Idle Banter qualification gaps`

- qualified head: `fcdb80e96335dfa719f7cfb4d8241f3210b70221`
- PR-head CI `34785841957`: **PASS**
- merge commit: `9624dc1611a59103b366b5aefdf46696d3de3def`
- post-merge master CI `34786184975`: **PASS**

PR #106 changed test code only:

- `src-tauri/src/character/idle_banter_closeout_tests.rs`
- `src-tauri/src/character/mod.rs` only to register that test module under `#[cfg(test)]`

No production behavior changed in PR #106.

### Implemented behavior

Idle Banter now:

- is enabled by default for new profiles;
- waits 60 minutes after direct user interaction with Moose before the first eligible remark;
- repeats at about 30 minutes while inactivity continues;
- applies bounded internal repeat jitter of approximately +/-20%;
- uses **time since direct interaction with Moose**, not OS keyboard/mouse idle, as its inactivity definition;
- resets the inactivity episode on direct Moose interaction;
- resets fresh on wake/resume rather than creating catch-up speech;
- maintains no backlog of missed banter events;
- uses the existing ambient queue/policy path rather than a second ambient scheduler;
- uses the selected text provider with no silent provider fallback;
- uses the existing standalone speech/TTS pipeline;
- respects existing mute, conversation, quiet-hours, annoyance, dismissal, cooldown, and max-per-hour protections;
- gives foreground user intent priority over stale ambient work;
- uses ownership-aware presentation cleanup so stale ambient cancellation cannot overwrite newer foreground `Thinking`, `Talking`, or `Hidden` state.

### Seed topics and repetition control

Settings include editable Idle Banter seed topics with:

- built-in defaults;
- Add/Edit/Delete/Restore-defaults UI;
- trimming and validation;
- duplicate rejection/normalization;
- non-empty-list enforcement;
- random selection;
- immediate-repeat avoidance when multiple topics exist;
- unbiased selection among remaining topics after repeat exclusion;
- correct one-topic behavior.

Recent generated Idle Banter lines are bounded and session-only. A line is recorded only after successful completed playback, not merely when queued.

### Prompt/privacy evidence

The implementation does not add automatic transcript injection to Idle Banter.

PR #106 explicitly proves:

- private seed/recent-banter sentinel text can appear in the constructed prompt but does not enter captured tracing logs;
- a single configured seed selects exactly that topic;
- wake reset uses the currently configured initial delay and starts a fresh inactivity episode.

---

## What still needs to be done

### 1. Reconcile `docs/IDLE_BANTER_TODO.md`

This is the immediate next task.

The TODO was written before implementation and still contains many unchecked boxes. Do **not** interpret an unchecked box as proof that code is missing.

Audit each `IB-*` task/subtask against current merged code/tests/docs and qualification evidence.

For each item:

- mark it complete only when merged evidence proves it;
- leave genuinely unimplemented or manual-only items unchecked;
- add concise evidence references where useful (PR, SHA, run ID, module/test);
- convert the file from a planned queue into an accurate closeout record.

Useful evidence baseline:

- PR #105
- PR #105 head CI `34778120985` PASS
- PR #105 KittenTTS acceptance `34778120998` PASS
- implementation merge `c531fa4098b32a5acb37e485dd4a8b091d70dadf`
- implementation master CI `34778513562` PASS
- implementation master KittenTTS acceptance `34778513583` PASS on retry attempt 2
- PR #106
- PR #106 head `fcdb80e96335dfa719f7cfb4d8241f3210b70221`
- PR #106 CI `34785841957` PASS
- code-bearing master `9624dc1611a59103b366b5aefdf46696d3de3def`
- post-merge code-bearing master CI `34786184975` PASS
- docs-only master `ace6c0d8755e8f352ef3bffb8fcea7558a68727c`
- docs-only CI `34786573746` PASS

If the item-by-item audit uncovers a real code gap, resume the Ralph Loop normally: branch -> implement/tests -> exact-head CI -> guarded merge -> exact-master verification -> tracker update.

### 2. Human-owned KittenTTS voice-selection closeout remains open

Do not auto-close:

- `KCR-330` / `KTT-805` — human audition and final default Local KittenTTS voice selection.

The provisional Local KittenTTS default remains **Bella**.

Automated ASR, CI, or model-output metrics are not sufficient to close this item; it requires a human listening decision.

### 3. Final documentation consistency check

After reconciling `docs/IDLE_BANTER_TODO.md`, verify consistency across:

- `README.md`
- `docs/PRIVACY.md`
- `docs/IDLE_BANTER_SPEC.md`
- `docs/IDLE_BANTER_TODO.md`
- `docs/VOICE_SELECTION.md`

Avoid documentation churn if the documents are already consistent.

### 4. Inspect stale open PR #96 before deciding whether to close it

GitHub still reports open PR #96:

- `KTT-500/501/504: add provider-aware voice settings`
- head: `03e64784619e532860dbb0dc9d342291e358ebd0`
- branch: `ralph/ktt-500-voice-settings`

That PR is old and diverged substantially from current `master`:

- current master was 149 commits ahead of the PR head at this handoff;
- the old branch still had 6 branch-only commits;
- merge base: `4de5170c6c9f20a497b706b619d492adb46ac1af`.

Do **not** merge PR #96 blindly. First determine whether all intended functionality was superseded by later merged KittenTTS/voice-settings work. If fully superseded, close the PR with an explanatory comment; if some unique requirement is genuinely missing, extract only that requirement into fresh work based on current `master`.

### 5. Optional low-severity KittenTTS hardening opportunities

These are known follow-ups, not blockers for Idle Banter closeout:

1. `src-tauri/src/ai/local_tts/runtime/engine.rs` has an ignored `real_kitten_cpu_acceptance` test that hardcodes `LocalTtsPlatform::LinuxX86_64`; future hardening could make it platform-aware or explicitly Linux-only.
2. The same engine extracts the first ORT output positionally (`outputs[0]`); future hardening could validate output count/name and return a sanitized inference error for malformed output.
3. Local TTS runtime errors use generic provider-neutral user messaging; future UX hardening could provide Local-TTS-specific safe messages without exposing raw errors.

Do not reopen completed Idle Banter qualification solely for these items.

---

## Architecture constraints to preserve

Do not create a second Idle Banter speech stack or second ambient scheduler.

Preserve these invariants:

- existing `AmbientScheduler` remains the ambient queue/cancellation owner;
- Idle Banter scheduling only decides when a specialized ambient occurrence becomes due;
- `process_ambient_event` remains the authoritative ambient generation/delivery path;
- selected text provider remains authoritative; no automatic local/cloud fallback;
- `invoke_standalone_speech` remains the authoritative standalone speech path;
- selected TTS provider remains authoritative;
- transcript text is not automatically injected into Idle Banter prompts;
- foreground user intent always preempts stale ambient generation/presentation;
- presentation ownership must prevent stale ambient cleanup from clobbering a newer foreground state.

---

## Ralph Bridge operating instructions for the next session

The user wants autonomous Ralph Loop behavior.

Once Ralph Bridge is available:

1. Call `ralph_ping`.
2. Confirm repository state for `ekkus93/ai-talking-moose` at `master`.
3. Read this handoff plus `docs/IDLE_BANTER_SPEC.md` and `docs/IDLE_BANTER_TODO.md` directly from the repository.
4. Continue the TODO reconciliation task-by-task.
5. Use Ralph Bridge to inspect/mutate GitHub and monitor CI instead of asking the user to push or report routine status.
6. If CI fails, diagnose it, fix it, push, and continue.
7. Use exact-head/guarded merges where supported.
8. Verify exact post-merge `master` CI before declaring a task closed.
9. Stop only for a genuine user-owned decision/authorization/blocker or when all automatable work is complete.

Do not stop merely because a CI job fails.

---

## Clean-start summary

At this handoff:

- Idle Banter production implementation: **merged**.
- Idle Banter correctness hardening: **merged**.
- explicit Idle Banter closeout evidence tests: **merged**.
- last code-bearing master: `9624dc1611a59103b366b5aefdf46696d3de3def` — CI `34786184975` PASS.
- current pre-handoff docs master: `ace6c0d8755e8f352ef3bffb8fcea7558a68727c` — CI `34786573746` PASS.
- no active Idle Banter implementation/test PR needs merging.
- immediate next technical/documentation task: reconcile `docs/IDLE_BANTER_TODO.md` item-by-item against merged evidence.
- human-only open item: `KCR-330` / `KTT-805` final Local KittenTTS voice audition/default selection.
- stale PR #96 should be audited and likely closed if superseded; do not merge it as-is.
