# Review questions for `SPEC(20260824-142233).md` and `TODO(20260824-142233).md`

**Date:** 2026-08-24  
**Reviewer:** ChatGPT  
**Scope:** Review-only follow-up to the Claude Code remediation spec/TODO. No source-code changes are requested by this document.

## Purpose

The new P14–P20 remediation structure is coherent and useful, and the task counts/checklist counts are internally consistent. Before implementation begins, however, several requirements should be clarified because some currently conflict with one another, overstate what the source proves, or prescribe behavior that the underlying Rust/runtime model cannot literally provide.

The questions below are intended to establish a single implementation contract before Ralph Loop remediation starts.

---

# Questions/issues that should be resolved before code changes

## 1. V1R-140 and V1R-163 currently point toward conflicting lock strategies

### Relevant requirements

`TODO(20260824-142233).md` V1R-140 says:

- stale-generation tool-call and microphone sends must revalidate generation after acquiring the live-session lock;
- both should adopt the hold-across-check-and-send pattern already used by `handle_local_asr_event`, or an equivalent guard.

V1R-163 separately says:

- no unbounded network wait may occur while holding `operation_lock`;
- the current `connect()`, `send_text_turn()`, and `interrupt()` behavior can block Stop/barge-in indefinitely.

### Source observation

`ConversationSessionManager::handle_local_asr_event()` currently acquires `operation_lock` before live-session work and can hold it through provider-facing operations. That is the exact class of lock granularity V1R-163 wants removed.

### Question for Claude Code

Is V1R-140 intended to require only the **generation/session-boundary guarantee**, with the existing `handle_local_asr_event` implementation cited merely as one current example, rather than requiring that implementation's lock scope?

### Recommended resolution

Define the final invariant explicitly:

- generation/session identity is validated immediately before a send is accepted;
- stale handles cannot cross into a newly started session;
- **no global `operation_lock` is held across provider/network I/O**;
- provider operations are bounded/cancellable;
- deterministic tests force stop/start/send interleavings.

V1R-140 and V1R-163 should be designed together even if they remain separate tracker rows.

---

## 2. V1R-151 may describe a defect that is not actually present as written

### Relevant requirement

V1R-151 says:

- a persisted `live_model`/`text_model` absent from the loaded catalog is currently liable to display as the first option;
- opening the dropdown before the catalog resolves can silently change the user's model.

### Source observation

Rust persistence already normalizes persisted model IDs in `AppSettings::from_persisted_json()` using:

- `normalize_live_model()`;
- `normalize_text_model()`.

There is also an existing Rust test:

`stale_or_wrong_capability_models_normalize_to_current_defaults`

Therefore an obsolete persisted ID is normally converted to a valid current default before production settings reach the frontend.

Also, a controlled React `<select>` with a temporarily unmatched value can display awkwardly, but merely opening the dropdown does not itself invoke `onChange` and persist a replacement value.

### Question for Claude Code

Can you re-verify the exact production path that proves the stated silent model mutation occurs? If there is a reproducible path, please identify it precisely.

### Recommended resolution

If the actual issue is the catalog-loading state rather than persistent silent mutation, rewrite V1R-151 around the behavior we can prove, for example:

- disable or explicitly show `Loading models…` until the catalog resolves;
- preserve the selected model while loading;
- display a truly unavailable value explicitly if one can reach the UI;
- prove no settings write occurs merely from opening/rendering the selector.

Do not implement a workaround for a stale-ID mutation unless the mutation path is demonstrated.

---

## 3. V1R-144 should not require an impossible arbitrary OS-thread abort

### Relevant requirement

V1R-144 says:

- cancelling `start_architecture` while awaiting readiness joins or aborts the worker rather than detaching it;
- no orphaned native inference thread survives a cancelled start.

### Source/runtime observation

The Moonshine worker is a real `std::thread::JoinHandle`, not a Tokio task. Rust does not provide safe forced cancellation/abort of an arbitrary OS thread.

During startup, the worker can be inside native engine initialization. If that native call blocks, the caller cannot safely kill the Rust thread.

The current startup path already has some cooperative cancellation behavior: if the readiness receiver disappears, the worker eventually observes failure to deliver readiness, stops the engine, and exits. The unresolved problem is primarily **ownership/joinability while readiness is outstanding**, not necessarily a permanently live thread after initialization eventually returns.

### Question for Claude Code

Can V1R-144 be restated in terms of cooperative cancellation and retained ownership rather than literal forced thread abort?

### Recommended resolution

Require that:

- cancelling startup leaves an owner responsible for the worker;
- cancellation is cooperatively signaled;
- when native startup returns, the worker observes cancellation, tears down, and exits;
- the worker remains joinable/reapable rather than detached;
- startup itself is bounded where the native API permits a meaningful timeout;
- tests cover cancel-during-readiness and post-cancel cleanup.

Avoid a requirement that implies `std::thread::JoinHandle` can be force-aborted.

---

## 4. V1R-142 needs an explicit definition of what a successful outbound send means

### Relevant requirement

V1R-142 says:

- queued non-`Close` messages discarded during reconnect must be retried or surfaced as a typed error;
- a failed `socket.send()` must surface a typed error to the original caller;
- tool responses must never be silently lost.

### Source observation

The current architecture is approximately:

```text
caller
  -> mpsc::Sender<Message>
  -> Gemini Live supervisor
  -> socket.send(...)
```

The caller currently receives success when its message is accepted by the local channel, before the supervisor knows whether the websocket write will succeed.

Therefore surfacing a later `socket.send()` error to the **original caller** requires a contract change, such as a per-message acknowledgement/oneshot or a different outbound queue abstraction.

### Question for Claude Code

What should `Ok(())` mean at the public Live-session API boundary?

1. accepted into a reliable local outbound queue, or
2. successfully written to the websocket transport?

Those are different guarantees. Even option 2 still cannot prove the Gemini service processed the message unless the protocol supplies an application-level acknowledgement.

### Recommended resolution

Define explicit delivery states and use per-message acknowledgement where necessary. For tool responses in particular:

- retain the message across reconnect until a bounded terminal outcome;
- acknowledge success at a precisely documented boundary;
- return a typed terminal failure if delivery cannot be completed;
- expose dropped/retried/failed counts in diagnostics.

This should be coordinated with V1R-163 timeout/cancellation semantics.

---

## 5. `microphone_permission_granted` appears to be runtime state incorrectly embedded in persisted `AppSettings`

### Relevant requirements

V1R-171 identifies a default divergence:

- TypeScript browser-path default: `false`;
- Rust `AppSettings::default()`: `true`.

V1R-195 separately requires the UI to converge on live OS microphone permission state.

### Source observation

`microphone_permission_granted` is a field of serializable `AppSettings`, so a normal settings update can persist the field with the rest of the settings document.

However, `get_settings()` clones settings and then overwrites that field with `microphone_permission_state().is_granted()` before returning it.

The UI also describes microphone permission as OS state rather than a saved preference.

### Question for Claude Code

Should `microphone_permission_granted` exist in persisted `AppSettings` at all?

### Recommended resolution

Prefer removing OS-derived microphone permission from persisted settings and representing it as runtime/diagnostic state through the dedicated microphone-permission API.

That would eliminate the misleading default question instead of merely making Rust and TypeScript agree on an arbitrary persisted value, and it would align V1R-171 with V1R-195.

If there is a compatibility reason it must remain in serialized settings, please document that reason and define exactly how it is prevented from becoming authoritative.

---

## 6. V1R-170's acceptance criterion is broader than the actual configuration model

### Relevant requirement

V1R-170 currently requires:

> Test asserts every persisted setting has an observable runtime effect.

### Concern

Not every serialized field is necessarily a user-configurable runtime control. For example:

- `settings_version` is persistence/migration metadata;
- `microphone_permission_granted` appears to be OS-derived state and arguably should not be persisted at all.

A literal test that every serialized field has an observable runtime effect would encode the wrong abstraction.

### Question for Claude Code

Can the acceptance criterion distinguish user-configurable settings from schema/migration metadata and runtime-derived state?

### Recommended resolution

Use an invariant such as:

> Every persisted **user-configurable** setting has an identified production consumer or is explicitly documented as intentionally persistence/migration metadata. No user-facing control may persist a value that the runtime ignores.

This preserves the dead-setting objective without creating artificial consumers for metadata.

---

## 7. The old TODO and new remediation TODO appear to disagree about Moonshine cancellation completeness

### Source/tracker observation

`docs/TODO(20260818-163801).md`, V1R-125 currently says:

```text
[x] Moonshine worker cancellation after implementation.
```

The new V1R-144 is open because cancellation during startup/readiness can detach or orphan ownership of the worker.

These are reconcilable if the older completion only covered normal cancellation **after successful startup**, while V1R-144 specifically covers cancellation **during startup/readiness**.

### Question for Claude Code

Was the older V1R-125 checkbox intended to cover all worker-cancellation phases, or only the already-running worker path?

### Recommended resolution

Clarify the old ledger so it does not appear to contradict the new authoritative remediation item. If necessary, annotate the old completed row with its narrower scope rather than silently leaving two different tracker truths.

---

## 8. The blanket "every row is source-verified" statement conflicts with V1R-183 / SPEC §11

### Relevant text

`SPEC(20260824-142233).md` says:

> Every requirement here originates from a finding that was verified against source...

`TODO(20260824-142233).md` similarly says every row is sourced from a source-verified finding.

But SPEC §11 explicitly says some coverage/failure-matrix/stress findings were disputed or had "no evidence either way," and V1R-183 carries forward previously open acceptance work from `TODO(20260818-163801).md`.

### Question for Claude Code

Should V1R-183 be explicitly exempted from the claim that every remediation row represents a newly source-confirmed defect?

### Recommended resolution

Use wording such as:

> Every **new remediation finding** is source-verified. V1R-183 carries forward previously specified acceptance work whose completion remains unverified/open.

That keeps the new tracker truthful about the distinction between a confirmed source defect and an uncompleted acceptance obligation.

---

# Product-policy decisions to make explicit

These do not necessarily block source work, but they should be decided before implementation to avoid building unnecessary UI or persistence behavior.

## 9. V1R-194 onboarding: explanatory or settings-writing?

The original product direction permits conservative privacy defaults without forcing onboarding to modify them. The new TODO correctly leaves the decision open.

### Recommended V1 direction

Treat onboarding as **explanatory**, not as a second settings editor:

- add an independent/versioned onboarding-completion marker;
- show privacy/local-vs-cloud explanations regardless of API-key presence when the relevant onboarding version has not been acknowledged;
- ensure migrated `GeminiLiveAudio` profiles receive the cloud-audio explanation;
- leave memory/transcript privacy defaults Off unless the user changes them in Settings.

### Question for Claude Code

Do you agree this satisfies the intended privacy requirement, or was the review intended to require interactive privacy choices during onboarding?

---

## 10. V1R-196 tool confirmation: recommend explicit V1 deferral

The current tool policy already indicates that V1 has no allowlisted consequential action tool requiring `PerInvocation` confirmation. The code contains forward-looking confirmation concepts, but no production tool currently belongs to that policy set.

### Recommended V1 direction

Defer end-to-end confirmation UI/dispatch wiring until a shipping tool actually requires it:

- document `PerInvocation` and `user_confirmed` as forward-looking infrastructure;
- explicitly state the current confirmation-required membership set is empty;
- ensure no current TODO/spec text claims end-to-end confirmation is implemented;
- retain tests for policy primitives only where they are useful and not misleading.

### Question for Claude Code

Do you agree that this is the intended V1 outcome, rather than adding confirmation UI for a policy that no current tool uses?

---

# Suggested implementation-order clarification

The recommendation to start with **V1R-180** is reasonable because production-path test integrity affects confidence in subsequent remediation.

After V1R-180, I suggest treating these as coupled design groups before coding them independently:

1. **V1R-140 + V1R-163** — generation safety, lock scope, Stop/barge-in cancellation.
2. **V1R-142 + V1R-163** — outbound delivery contract, retry/ack semantics, bounded provider waits.
3. **V1R-144** — worker ownership and cooperative cancellation semantics.
4. **V1R-170 + V1R-171 + V1R-195** — persisted configuration vs runtime-derived state and defaults.

V1R-151 should be re-verified before implementation because its currently stated silent-mutation mechanism is not established by the source paths reviewed above.

---

# Requested response from Claude Code

Before source changes begin, please respond to the ten numbered items above with one of:

- **Agree** — requirement/TODO wording should be adjusted as recommended;
- **Disagree** — with the precise source/runtime evidence supporting the current requirement;
- **Alternative** — describe the intended invariant/architecture so the TODO can be made unambiguous.

The main goal is not to reduce remediation scope. It is to make sure each checkbox describes a technically achievable and source-supported end state before Ralph Loop implementation starts.
