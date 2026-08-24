# Responses to `REVIEW_QUESTIONS(20260824-143800).md`

**Date:** 2026-08-24
**Responder:** Claude Code
**Reviewed documents:** `SPEC(20260824-142233).md`, `TODO(20260824-142233).md`
**Outcome:** All ten items accepted. Amendments applied to both documents plus one scope annotation in `TODO(20260818-163801).md`.

---

## Method

Each factual claim in the review was **re-verified against source** before responding, rather than accepted or disputed on reasoning alone. Six items rested on checkable claims; all six checked out in the reviewer's favour, including three that identify genuine errors in the original specification:

| # | Claim to verify | Result |
| --- | --- | --- |
| 1 | `handle_local_asr_event` holds `operation_lock` across provider I/O | **Confirmed** — `session.rs:346` → `:389` |
| 2 | Persisted model ids are normalized before reaching the frontend | **Confirmed** — `app/state.rs:146-151`; test at `ai/google/config.rs:318` |
| 3 | The Moonshine worker is an OS thread, not a Tokio task | **Confirmed** — `pipeline.rs:16`, `:94`, joined via `spawn_blocking` |
| 4 | Caller `Ok(())` precedes the socket write | **Confirmed** — `live.rs:630` sender; supervisor `recv()` at `:451/:487/:543` |
| 5 | `get_settings()` overwrites `microphone_permission_granted` | **Confirmed** — `commands/settings.rs:95` |
| 7 | Old V1R-125 test covers only an already-started worker | **Confirmed** — `engine_tests.rs:445` opens then cancels |

Three of the original requirements were not merely imprecise but **unachievable or self-contradictory as written** (items 1, 3, 4). That is recorded plainly here rather than softened.

---

## Item-by-item

### 1. V1R-140 / V1R-163 conflicting lock strategies — **Agree**

Confirmed contradiction. `handle_local_asr_event` acquires `operation_lock` (`session.rs:346`) and holds it through `send_text_turn().await` (`:389`) — exactly the granularity V1R-163 requires removed. Citing it as the pattern to adopt was an error.

**Applied.** V1R-140 now specifies the **generation/session-identity invariant only** and explicitly prescribes no lock scope, with a note that `handle_local_asr_event` is an instance of the defect rather than a model to copy. A row was added requiring that no global `operation_lock` is held across provider/network I/O. V1R-163 gained a reciprocal note that it owns the lock/cancellation model and must be designed jointly with V1R-140 and V1R-142. SPEC §4.1 and §6.4 amended to match.

### 2. V1R-151 mutation path unproven — **Alternative**

The reviewer is right about the mechanism. `AppSettings::from_persisted_json` normalizes both model ids (`app/state.rs:146-151`), covered by `stale_or_wrong_capability_models_normalize_to_current_defaults` (`ai/google/config.rs:318`), so a stale persisted id does not reach the UI unmatched. Opening a `<select>` also does not fire `onChange`. The original wording overstated both.

A narrower defect does survive and is worth keeping: `googleModels` starts `[]`, so during the fetch window the control renders with a `value` matching no option. It misrepresents the current selection, and a user who *interacts* in that window — not merely opens it — can write a model they did not intend.

**Applied.** V1R-151 retitled "Model selector loading state" and rescoped to the loading window, with an explicit note that the stale-id mechanism is unreachable and no workaround for it is to be implemented. A row was added requiring proof that no settings write occurs from rendering or opening the selector alone.

### 3. V1R-144 implies impossible thread abort — **Agree**

Confirmed: `pipeline.rs:16` imports `std::thread::JoinHandle`, `:94` holds `Option<JoinHandle<…>>`, reaped via `spawn_blocking(|| worker.join())`. Rust offers no safe forced abort of an arbitrary OS thread, and the worker may be inside a blocking native init call. "Joins or aborts" was not implementable.

**Applied.** Restated as cooperative cancellation with retained ownership: an identified owner remains responsible, cancellation is signalled cooperatively, the worker observes it when the native call returns and then tears down and exits, it stays joinable rather than detached, and startup is bounded where the native API permits. SPEC §4.5 records why the original wording was wrong.

### 4. V1R-142 undefined send contract — **Agree**

Confirmed: `live.rs:630` is `sender: mpsc::Sender<Message>`; the supervisor pulls via `recv()` and writes separately, so caller `Ok(())` means *accepted into the local channel*. Requiring a later socket error to reach "its caller" demanded a contract change the requirement never specified.

**Applied.** V1R-142 now requires the meaning of `Ok(())` at the public boundary to be documented explicitly as either (a) accepted into a reliable local queue or (b) written to the transport; requires explicit delivery states (queued / written / retried / terminally failed); requires tool responses to be retained across reconnect to a bounded terminal outcome; and requires diagnostics to expose dropped/retried/failed counts. The reviewer's caveat that transport success still does not prove service-side processing absent an application-level ack is recorded in SPEC §4.3.

### 5. `microphone_permission_granted` is runtime state — **Agree**

Confirmed: `commands/settings.rs:95` overwrites the field with live OS state on every read, so the persisted value is never authoritative. The default divergence flagged in V1R-171 is a symptom of the field being persisted at all, and making the two defaults agree would fix the wrong thing.

**Applied.** V1R-171 now prefers removing the field from persisted `AppSettings` in favour of the dedicated microphone-permission API, with a documented-exception path if compatibility demands otherwise. New SPEC §7.2a states the reasoning and explicitly rules out "make the defaults agree" as the fix. This aligns V1R-171 with V1R-195 as the reviewer intended. V1R-171's broader point stands unchanged: other fields in the mirrored defaults blob are still hand-maintained.

### 6. V1R-170 acceptance criterion too broad — **Agree**

"Every persisted setting has an observable runtime effect" would force artificial consumers for `settings_version` (migration metadata) and `microphone_permission_granted` (runtime-derived, and per item 5 should not be persisted).

**Applied.** The invariant is now: every persisted **user-configurable** setting has an identified production consumer, or is explicitly classified as persistence/migration metadata or runtime-derived state. Acceptance restated as "no user-facing control persists a value the runtime ignores." The dead-setting objective for `volume`, `hide_delay_seconds`, and `tts_model` is unchanged.

### 7. Old V1R-125 vs new V1R-144 — **Agree**

Confirmed reconcilable exactly as proposed. `cancellation_stops_once_and_prevents_reuse` (`engine_tests.rs:445`) calls `open_with_components(...).unwrap()` and *then* cancels — it only exercises an already-started engine. The old row's wording was broader than its coverage.

**Applied.** The old row in `TODO(20260818-163801).md` now carries an inline scope note stating it covers already-started cancellation only, and pointing at V1R-144 for the startup/readiness case. Two tracker truths are no longer in silent conflict.

### 8. "Every row source-verified" vs §11 / V1R-183 — **Agree**

A genuine internal inconsistency in the original documents.

**Applied.** Both documents now say every **new remediation finding** is source-verified, with `V1R-183` named as the sole exception — previously specified acceptance work whose completion remains unverified or open. V1R-183 carries an inline exemption note.

### 9. Onboarding explanatory or settings-writing — **Agree**

Adopted as recommended: onboarding is **explanatory**, not a second settings editor. This satisfies the requirement the review was actually protecting — that users placed on cloud microphone audio see the cloud-audio explanation — without duplicating Settings.

**Applied.** V1R-194 records the decision and now requires a versioned completion marker independent of key presence, explanations shown whenever the relevant onboarding version is unacknowledged, cloud-audio explanation for migrated `GeminiLiveAudio` profiles, and privacy defaults remaining Off unless changed in Settings. SPEC §8.4 matches.

### 10. Tool confirmation deferral — **Agree**

Adopted as recommended. Building confirmation UI and dispatch wiring for an empty membership set is speculative work.

**Applied.** V1R-196 now closes by *documenting* the deferral rather than implementing confirmation: `PerInvocation` and `user_confirmed` documented as forward-looking infrastructure in `docs/TOOLS_V1_POLICY.md`, the V1 membership set stated as empty, no document claiming end-to-end confirmation exists, and policy-primitive tests retained only where not misleading. The row reopens with full end-to-end criteria if a shipping tool ever requires confirmation. SPEC §8.6 matches.

---

## Implementation-order clarification

The suggested coupling is accepted and now recorded in the affected rows:

1. **V1R-140 + V1R-163** — generation invariant, lock scope, Stop/barge-in cancellation. Design together.
2. **V1R-142 + V1R-163** — outbound delivery contract, retry/ack semantics, bounded provider waits.
3. **V1R-144** — worker ownership and cooperative cancellation.
4. **V1R-170 + V1R-171 + V1R-195** — persisted configuration vs runtime-derived state.

`V1R-180` remains first for the reason the review endorses: while privacy gates are proven only by `#[cfg(test)]` helpers that re-implement the production branch rather than calling it, no subsequent closure can be trusted.

`V1R-151` is re-verified and rescoped ahead of implementation, per item 2.

---

## Scope

No source code changed in response to this review. The changes are confined to:

- `SPEC(20260824-142233).md` — §1, §4.1, §4.3, §4.5, §5.1, §7.1, new §7.2a, §8.4, §8.6
- `TODO(20260824-142233).md` — tracker rules, V1R-140, V1R-142, V1R-144, V1R-151, V1R-163, V1R-170, V1R-171, V1R-183, V1R-194, V1R-196
- `TODO(20260818-163801).md` — V1R-125 scope annotation only

Remediation scope was not reduced. Three requirements were made *achievable* rather than smaller; one (V1R-151) was narrowed to what the source actually proves; two (V1R-194, V1R-196) were resolved from open decisions into recorded ones.
