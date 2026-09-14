# AI Talking Moose — Chat Session Handoff (2026-09-13)

> **Historical handoff snapshot.** Do not use the voice-selection status in this dated handoff as current project state. On 2026-09-14, PR #114 added the automated ASR audition proxy and PR #115 selected `Luna` after explicit owner approval. `KCR-330` / `KTT-805` is closed for V1 by owner-approved ASR proxy evidence, without claiming subjective human listening. Use `docs/VOICE_SELECTION.md` and the current closeout/reconciliation docs for authoritative status.

## Purpose

This document is the handoff point for the next ChatGPT session working on `ekkus93/ai-talking-moose`.

**Use Ralph Bridge first in the new session.** Do not rely on chat memory alone for repository state.

Recommended startup sequence:

1. Call `ralph_ping` through Ralph Bridge.
2. Use Ralph Bridge repository-state tooling (for example `get_repo_state`) for `ekkus93/ai-talking-moose` at `master`.
3. Confirm that current `master` is at or descends from the last code-bearing handoff SHA below.
4. Read this handoff, `docs/IDLE_BANTER_SPEC.md`, and `docs/IDLE_BANTER_TODO.md`.
5. Continue the remaining closeout work described here.

If Ralph Bridge is not exposed in the new chat, resolve that tool/connector availability before beginning repository mutations; the new session is intended to continue through Ralph Bridge.

---

## Clean repository baseline

Repository: `https://github.com/ekkus93/ai-talking-moose`

Default branch: `master`

### Last code-bearing master SHA

`9624dc1611a59103b366b5aefdf46696d3de3def`

This is the merge commit for PR #106 and is the last commit in this handoff that changes Rust/test code. The handoff document itself was committed afterward as documentation-only work, so the current `master` seen by the next session should be a **descendant** of this SHA rather than necessarily equal to it.

PR #106 qualification:

- PR #106 — `test: close Idle Banter qualification gaps`
- qualified PR head: `fcdb80e96335dfa719f7cfb4d8241f3210b70221`
- PR-head CI `34785841957`: **PASS**
- merged code-bearing master SHA: `9624dc1611a59103b366b5aefdf46696d3de3def`
- post-merge master CI `34786184975`: **PASS**

Post-merge master CI passed all active scoped gates:

- Rust formatting: PASS
- Clippy with `-D warnings`: PASS
- complete Rust test suite: PASS
- generated backend contract verification: PASS

PR #106 changed test code only:

- `src-tauri/src/character/idle_banter_closeout_tests.rs`
- `src-tauri/src/character/mod.rs` only to register that test module under `#[cfg(test)]`

There were no production behavior changes in PR #106.

---

## Idle Banter implementation status

The Idle Banter feature is implemented, hardened, tested, and merged.

Primary implementation PR:

- PR #105 — `feat: add configurable Idle Banter`
- final qualified PR head: `d103facdacf4cd45b73d28c0637d108f148fc631`
- PR-head CI `34778120985`: **PASS**
- PR-head KittenTTS production CPU acceptance `34778120998`: **PASS** on Linux x86_64 and macOS arm64
- merge/master commit for PR #105: `c531fa4098b32a5acb37e485dd4a8b091d70dadf`
- post-merge master CI `34778513562`: **PASS**
- post-merge KittenTTS production CPU acceptance `34778513583`: **PASS on retry attempt 2** on the exact same master SHA

The first Linux attempt of `34778513583` failed during the 1/2/4-thread KittenTTS sweep after hardened inference had passed. That failed Linux job was rerun without changing source, and the retry passed completely. Treat that incident as a transient runner/runtime qualification flake, not an Idle Banter source regression.

### Implemented behavior

Idle Banter extends the existing ambient subsystem rather than creating a second speech architecture.

Implemented behavior includes:

- enabled by default;
- first eligible banter after 60 minutes without direct Moose interaction;
- repeat cadence approximately every 30 minutes while inactivity continues;
- fixed bounded repeat jitter of approximately +/-20%;
- direct Moose interaction resets the inactivity episode;
- wake/resume resets the episode instead of producing catch-up speech;
- no accumulated backlog of missed events;
- existing busy-state/ambient policy protections reused;
- foreground interaction safely preempts stale ambient work;
- presentation ownership prevents stale ambient cleanup from clobbering a newer foreground `Thinking`, `Talking`, or `Hidden` state;
- existing standalone speech/TTS path remains authoritative;
- no silent provider fallback;
- configured conversation/text provider generates the remark;
- configured TTS provider speaks it;
- local LLM + local KittenTTS remains capable of fully local/offline operation after model installation;
- no automatic transcript injection was added for Idle Banter.

### Editable seed topics

Settings support an editable creative-direction topic list with:

- built-in defaults;
- Add/Edit/Delete/Restore-defaults UI behavior;
- trimming, validation, and deduplication;
- non-empty-list requirement while Idle Banter is enabled;
- random topic selection;
- session-wide immediate-repeat avoidance when multiple topics exist;
- unbiased selection among the remaining topics after repeat exclusion;
- correct behavior when exactly one topic is configured.

A previous audit issue in repeat selection was fixed before PR #105 merged: direct-interaction resets no longer discard the prior seed in a way that permits immediate reuse when alternatives exist.

### Prompt/history/privacy behavior

The prompt uses:

- fixed Talking Moose character/personality instructions;
- the selected seed as a creative direction;
- bounded recent Idle Banter lines for repetition avoidance;
- approximate idle duration.

Recent Idle Banter history is bounded and session-only. A remark is recorded only after successful completed playback, not merely when queued.

PR #106 added explicit closeout tests proving:

- user seed/recent-banter sentinel strings can appear in the constructed prompt but do not enter captured tracing logs;
- a one-topic seed list selects exactly that configured topic instead of inventing another topic;
- wake reset uses the currently configured initial delay and starts a fresh inactivity episode.

---

## Architecture constraints for future work

Do not introduce a second ambient scheduler or separate speech path.

The merged implementation deliberately reuses existing subsystems:

- ambient generation/delivery remains integrated with the existing ambient pipeline;
- scheduling/runtime state is owned by the Idle Banter runtime integrated into app state;
- speech goes through the established standalone speech/TTS path;
- foreground activity/reset hooks are centralized;
- existing quiet-hours/ambient eligibility policy remains authoritative where applicable.

The code includes ownership-aware ambient presentation cancellation. Foreground user-triggered presentation must always have priority over stale background banter cleanup.

---

## Authoritative feature documents

Specification:

- `docs/IDLE_BANTER_SPEC.md`

Implementation tracker:

- `docs/IDLE_BANTER_TODO.md`

The tracker was created before implementation and still needs **final evidence reconciliation**. Do not interpret an unchecked box as proof that code is missing.

The previous session was auditing that tracker item-by-item instead of mass-checking it. PR #106 was created because the audit found three checklist bullets whose evidence was weaker than the tracker wording; those evidence gaps are now explicitly covered and merged.

---

## What still needs to be done

### 1. Reconcile `docs/IDLE_BANTER_TODO.md`

This is the immediate next automatable task.

Audit every `IB-*` task/subtask against current merged code, tests, docs, and CI evidence.

For each item:

- mark complete only when merged evidence proves it;
- leave genuinely unimplemented or manual-only items unchecked;
- add concise evidence references where useful;
- turn the file from a planned queue into an accurate closeout record;
- do **not** reimplement already merged behavior simply because an old checkbox is unchecked.

Evidence baseline:

- PR #105 implementation/hardening
- PR #105 head CI `34778120985` PASS
- PR #105 KittenTTS acceptance `34778120998` PASS
- merged implementation commit `c531fa4098b32a5acb37e485dd4a8b091d70dadf`
- master CI `34778513562` PASS
- master KittenTTS acceptance `34778513583` PASS on retry attempt 2
- PR #106 explicit closeout tests
- PR #106 qualified head `fcdb80e96335dfa719f7cfb4d8241f3210b70221`
- PR #106 CI `34785841957` PASS
- last code-bearing master SHA `9624dc1611a59103b366b5aefdf46696d3de3def`
- post-merge master CI `34786184975` PASS

If the item-by-item audit reveals a real code gap, use the normal Ralph Loop: branch -> implementation/tests -> exact-head CI -> guarded merge -> exact-master verification -> tracker update.

### 2. Keep the human KittenTTS default-voice acceptance open

One intentionally human-owned KittenTTS closeout item remains:

- `KCR-330` / `KTT-805` — human audition and final default Local KittenTTS voice selection.

The current/provisional Local KittenTTS default is **Bella**.

Do not auto-close this based on ASR scores, automated CI, or model-output metrics. It requires a human listening/owner decision.

### 3. Final documentation consistency check

After tracker reconciliation, verify the relevant documentation remains consistent:

- `README.md`
- `docs/PRIVACY.md`
- `docs/IDLE_BANTER_SPEC.md`
- `docs/IDLE_BANTER_TODO.md`
- `docs/VOICE_SELECTION.md`

If no substantive mismatch exists, avoid unnecessary doc churn.

---

## Known low-severity follow-up opportunities unrelated to Idle Banter closeout

These were previously observed during KittenTTS review and are not blockers for completed Idle Banter/KittenTTS qualification:

1. `src-tauri/src/ai/local_tts/runtime/engine.rs` has an ignored `real_kitten_cpu_acceptance` test that hardcodes `LocalTtsPlatform::LinuxX86_64`; future hardening could make it platform-aware or explicitly Linux-only.
2. The same engine extracts the first ORT output positionally (`outputs[0]`); future hardening could validate output count/name and return a sanitized inference error for malformed output.
3. Local TTS runtime errors ultimately pass through generic provider-neutral user messaging. It is privacy-safe but can read awkwardly for Local TTS; future UX hardening could provide Local-TTS-specific safe messages without exposing raw errors.

These are optional follow-ups, not reasons to reopen completed Idle Banter qualification.

---

## Ralph Loop instructions for the next session

The user wants autonomous Ralph Loop behavior.

Once Ralph Bridge is available:

1. Call `ralph_ping`.
2. Confirm `ekkus93/ai-talking-moose` repository state at `master`.
3. Confirm `master` descends from `9624dc1611a59103b366b5aefdf46696d3de3def`.
4. Read this handoff plus `docs/IDLE_BANTER_TODO.md` and `docs/IDLE_BANTER_SPEC.md`.
5. Continue TODO reconciliation task-by-task.
6. Use Ralph Bridge to inspect/mutate GitHub and monitor CI instead of asking the user to push or report routine CI status.
7. Keep looping through failures/fixes until all automatable tracker work is complete or a genuine user-owned decision is required.
8. Use exact-head/guarded merges where supported.
9. Verify post-merge `master` CI before declaring a task closed.

Do not stop merely because CI fails. Diagnose, fix, and continue. Stop only for a genuine user decision/authorization/blocker or when all automatable work is complete.

---

## Clean starting-point summary

At handoff:

- Idle Banter production implementation: **merged**.
- Idle Banter correctness hardening: **merged**.
- Explicit closeout evidence tests: **merged**.
- Last code-bearing `master` SHA: `9624dc1611a59103b366b5aefdf46696d3de3def`.
- Post-merge code-bearing master CI `34786184975`: **PASS**.
- The actual current `master` in the next session may be a later **docs-only descendant** because this handoff document was committed after the code baseline.
- No known unmerged production/test change is required before continuing documentation/tracker closeout.
- Immediate next task: **reconcile `docs/IDLE_BANTER_TODO.md` item-by-item against merged evidence**.
- Human-only open item: **KCR-330 / KTT-805 final Local KittenTTS voice audition/default selection**.
