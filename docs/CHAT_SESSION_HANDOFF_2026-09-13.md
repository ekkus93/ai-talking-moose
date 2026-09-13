# AI Talking Moose — Chat Session Handoff (2026-09-13)

## Purpose

This document is the handoff point for the next ChatGPT session working on `ekkus93/ai-talking-moose`.

**Use Ralph Bridge first in the new session.** Do not assume the repository state from chat memory alone.

Recommended startup sequence:

1. Call `ralph_ping` through Ralph Bridge.
2. Use Ralph Bridge repository-state tooling (for example `get_repo_state`) for `ekkus93/ai-talking-moose` at `master`.
3. Confirm that `master` is at or descends from the handoff SHA below.
4. Continue the remaining closeout work described in this document.

If Ralph Bridge is not exposed in the new chat, stop and resolve that connector/tool availability before beginning repo mutations; the purpose of the new session is to continue through Ralph Bridge.

---

## Repository state at handoff

Repository: `https://github.com/ekkus93/ai-talking-moose`

Default branch: `master`

Clean handoff master SHA:

`9624dc1611a59103b366b5aefdf46696d3de3def`

This is the merge commit for PR #106:

- PR #106 — `test: close Idle Banter qualification gaps`
- qualified PR head: `fcdb80e96335dfa719f7cfb4d8241f3210b70221`
- PR-head CI `34785841957`: **PASS**
- merged master SHA: `9624dc1611a59103b366b5aefdf46696d3de3def`
- post-merge master CI `34786184975`: **PASS**

The post-merge master CI passed all active scoped gates:

- Rust formatting: PASS
- Clippy with `-D warnings`: PASS
- complete Rust test suite: PASS
- generated backend contract verification: PASS

PR #106 changed test code only:

- `src-tauri/src/character/idle_banter_closeout_tests.rs`
- `src-tauri/src/character/mod.rs` only to register that test module under `#[cfg(test)]`

There are no production behavior changes in PR #106.

---

## Idle Banter implementation status

The Idle Banter feature itself is implemented and merged.

Primary implementation PR:

- PR #105 — `feat: add configurable Idle Banter`
- final qualified PR head: `d103facdacf4cd45b73d28c0637d108f148fc631`
- PR-head CI `34778120985`: **PASS**
- PR-head KittenTTS production CPU acceptance `34778120998`: **PASS** on Linux x86_64 and macOS arm64
- merge/master commit for PR #105: `c531fa4098b32a5acb37e485dd4a8b091d70dadf`
- post-merge master CI `34778513562`: **PASS**
- post-merge KittenTTS production CPU acceptance `34778513583`: **PASS on retry attempt 2** on the exact same master SHA

The first Linux attempt of `34778513583` failed during the 1/2/4-thread KittenTTS sweep after hardened inference had passed. The failed Linux job was rerun without changing source; the retry passed completely. Treat that incident as a transient qualification/runtime flake, not an Idle Banter source regression.

### Implemented Idle Banter behavior

The feature extends the existing ambient subsystem rather than adding a separate speech stack.

Implemented behavior includes:

- Idle Banter enabled by default.
- First eligible banter after 60 minutes without direct Moose interaction.
- Repeat cadence approximately every 30 minutes while inactivity continues.
- Fixed bounded repeat jitter of approximately +/-20%.
- Direct Moose interaction resets the inactivity episode.
- Wake/resume resets the inactivity episode instead of creating catch-up speech.
- No accumulated backlog of missed banter events.
- Existing busy-state/ambient policy protections are reused.
- Foreground interaction preempts/cancels stale ambient work safely.
- Presentation ownership prevents stale ambient cleanup from clobbering a newer foreground `Thinking`, `Talking`, or `Hidden` state.
- Existing standalone speech/TTS pipeline remains authoritative.
- No silent provider fallback.
- Current configured conversation/text provider generates the remark.
- Current configured TTS provider speaks the remark.
- Local LLM + local KittenTTS can remain fully local/offline after model installation.
- Cloud-provider use remains explicit through the selected provider; no automatic transcript injection was added for Idle Banter.

### Editable seed topics

Settings now support an editable list of creative-direction seed topics.

The implementation provides:

- built-in default topics;
- Add/Edit/Delete/Restore-defaults UI behavior;
- trimming/validation/deduplication;
- a non-empty-list requirement when Idle Banter is enabled;
- random topic selection;
- session-wide immediate-repeat avoidance when multiple topics exist;
- unbiased selection among the remaining topics after repeat exclusion;
- correct behavior when exactly one topic is configured.

The previous repeat-selection audit issue was fixed before PR #105 merged: direct-interaction resets no longer discard the previous seed in a way that permits immediate reuse when alternatives exist.

### Prompt/history/privacy behavior

The Idle Banter prompt uses:

- fixed Talking Moose character/personality instructions;
- the randomly chosen seed topic as a creative direction;
- bounded recent Idle Banter lines for repetition avoidance;
- approximate idle duration.

Recent Idle Banter history is session-only and bounded. A remark is recorded only after successful completed playback, not merely when queued.

PR #106 added explicit closeout tests proving:

- user seed/recent-banter sentinel strings can appear in the constructed prompt but do not enter captured tracing logs;
- a one-topic seed list selects exactly that configured topic instead of inventing another topic;
- wake reset uses the currently configured initial delay and starts a fresh inactivity episode.

---

## Important architecture notes

Do not create a second ambient scheduler or a second speech path.

The implementation deliberately reuses existing subsystems:

- ambient generation/delivery remains integrated with the existing ambient pipeline;
- scheduling/runtime state is owned by the Idle Banter runtime integrated into app state;
- speech goes through the established standalone speech/TTS path;
- foreground activity/reset hooks are centralized rather than independently implemented per feature;
- existing quiet-hours/ambient eligibility policy remains authoritative where applicable.

The code includes ownership-aware ambient presentation cancellation. Be careful when changing this area: foreground user-triggered presentation must always have priority over stale background banter cleanup.

---

## Specification and tracker documents

Authoritative feature specification:

- `docs/IDLE_BANTER_SPEC.md`

Implementation tracker:

- `docs/IDLE_BANTER_TODO.md`

The implementation tracker was originally created before coding and still needs **final evidence reconciliation**. Do not interpret unchecked boxes in that file as proof that the implementation is missing. The code and CI already implement/qualify the feature described above.

The previous session was auditing the tracker item-by-item rather than blindly checking everything. PR #106 was created specifically because that audit found three checklist bullets whose evidence was weaker than the wording; those evidence gaps are now covered and merged.

---

## What still needs to be done

### 1. Reconcile `docs/IDLE_BANTER_TODO.md`

This is the immediate next technical/documentation task.

Audit every `IB-*` task/subtask against the merged implementation and tests on current `master`.

For each tracker item:

- mark it complete only when the merged code/test/docs/CI evidence actually proves it;
- leave genuinely unimplemented or manual-only items unchecked;
- add concise evidence references where useful (PR number, commit SHA, workflow run ID, or relevant test/module);
- change the tracker from a purely planned queue into an accurate closeout record.

Do **not** reimplement already merged Idle Banter behavior merely because its old checkbox is unchecked.

Suggested evidence baseline:

- PR #105 implementation and review hardening
- PR #105 head CI `34778120985` PASS
- PR #105 KittenTTS acceptance `34778120998` PASS
- merged implementation commit `c531fa4098b32a5acb37e485dd4a8b091d70dadf`
- master CI `34778513562` PASS
- master KittenTTS acceptance `34778513583` PASS on retry attempt 2
- PR #106 closeout tests
- PR #106 qualified head `fcdb80e96335dfa719f7cfb4d8241f3210b70221`
- PR #106 CI `34785841957` PASS
- final clean master SHA `9624dc1611a59103b366b5aefdf46696d3de3def`
- final master CI `34786184975` PASS

If the item-by-item audit uncovers a real code gap, fix it through the normal Ralph Loop: branch -> implementation/tests -> exact-head CI -> guarded merge -> exact-master verification -> tracker update.

### 2. Keep the human KittenTTS default-voice acceptance open

There is one intentionally human-owned KittenTTS closeout item from the earlier KittenTTS work:

- `KCR-330` / `KTT-805` — human audition and final default Local KittenTTS voice selection.

The current/provisional local default remains **Bella**.

Do not auto-close this item based on ASR scores, automated CI, or model-output metrics. It requires an owner/human listening decision.

### 3. Final documentation consistency check

After `IDLE_BANTER_TODO.md` is reconciled, verify the relevant documentation remains mutually consistent, especially:

- `README.md`
- `docs/PRIVACY.md`
- `docs/IDLE_BANTER_SPEC.md`
- `docs/IDLE_BANTER_TODO.md`
- `docs/VOICE_SELECTION.md` where the still-open human voice-selection item is referenced

If no substantive mismatch exists, avoid churn.

---

## Known low-severity follow-up opportunities unrelated to Idle Banter closeout

These were previously observed during KittenTTS review but were not blockers for the completed KittenTTS/Idle Banter work:

1. `src-tauri/src/ai/local_tts/runtime/engine.rs` has an ignored `real_kitten_cpu_acceptance` test that hardcodes `LocalTtsPlatform::LinuxX86_64`; future hardening could make it platform-aware or explicitly Linux-only.
2. The same engine extracts the first ORT output positionally (`outputs[0]`); future hardening could validate output count/name and return a sanitized inference error for malformed output.
3. Local TTS runtime errors ultimately pass through generic provider-neutral user messaging, which is privacy-safe but can read awkwardly for Local TTS; future UX hardening could provide Local-TTS-specific safe messages without exposing raw errors.

These are optional follow-ups, not reasons to reopen completed Idle Banter qualification.

---

## Ralph Loop operating instructions for the next session

The user wants autonomous Ralph Loop behavior.

Once Ralph Bridge is available:

1. Call `ralph_ping`.
2. Confirm repository state for `ekkus93/ai-talking-moose` at `master`.
3. Read this handoff plus `docs/IDLE_BANTER_TODO.md` and `docs/IDLE_BANTER_SPEC.md` from the repository.
4. Continue the TODO reconciliation task-by-task.
5. Use Ralph Bridge to inspect/mutate GitHub and monitor CI instead of asking the user to push or report routine CI status.
6. Keep looping through failures and fixes until the technical tracker is reconciled/completed or a genuine user-owned decision is required.
7. Use expected-head/guarded merges where supported.
8. Verify post-merge `master` CI before declaring a task closed.

Do not stop merely because a CI run fails; diagnose it, fix it, and continue. Stop only for an actual user decision/authorization/blocker or when all automatable work is complete.

---

## Clean starting point summary

At the time this handoff was written:

- Idle Banter production implementation: **merged**.
- Idle Banter correctness hardening: **merged**.
- Explicit closeout evidence tests: **merged**.
- `master`: `9624dc1611a59103b366b5aefdf46696d3de3def`.
- Post-merge master CI `34786184975`: **PASS**.
- No known unmerged production/test change is required before continuing documentation/tracker closeout.
- Immediate next task: **reconcile `docs/IDLE_BANTER_TODO.md` item-by-item against merged evidence**.
- Human-only open item: **KCR-330 / KTT-805 final Local KittenTTS voice audition/default selection**.
