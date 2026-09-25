# Wake Word V1 Post-Closeout Remediation Spec

**Date:** 2026-09-25
**Status:** Draft remediation specification
**Review source:** post-closeout code review of `master` at `0ea5e03f9012884e2858d7b478fde916f0f163d7`
**Prior closeout baseline:** `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`
**Final implementation SHA recorded by prior closeout:** `99e216e13f78c1a0605684801d89dbd6b33ca7c9`

## 1. Purpose

Wake Word V1 was marked closed, but post-closeout review found product and acceptance gaps that are not resolved by the recorded CI evidence. This specification defines the remediation needed to make Wake Word V1 either actually satisfy the prior user-ready claims or explicitly narrow those claims in code, UI, documentation, and gates.

The remediation goal is not to redo already-working artifact, KWS, corpus, and runtime-manager work. The goal is to close the uncovered integration boundaries:

1. Settings changes must control the real native listener thread, not only the runtime enum.
2. Manual conversation start/stop must transfer shared microphone ownership deterministically.
3. Wake-triggered command handoff must not fail silently for unsupported ASR modes.
4. Downstream command-ASR acceptance must prove the first command word survives the wake handoff.
5. A clean installed app must have a supported path to verified Wake model/runtime artifacts.
6. Performance evidence must measure the production idle listener path, not only an initialized KWS session.
7. Documentation and UI must match the actual supported behavior.
8. Final audits must not close items that only have component evidence when the requirement is integrated behavior.

## 2. Non-goals

- Do not weaken fail-closed artifact verification, SHA checks, architecture checks, or native runtime identity checks.
- Do not introduce full-time cloud transcription for wake detection.
- Do not add arbitrary wake phrase editing or sensitivity controls for V1.
- Do not implement V1 barge-in while Moose is speaking.
- Do not persist raw Wake PCM, wake handoff PCM, generated fixture PCM, transcripts, or credentials in diagnostics or logs.
- Do not reopen completed artifact identity, native KWS corpus, or duplicate-stack cleanup work unless a change directly touches those paths.

## 3. Defects to fix

### 3.1 Settings toggle controls only runtime state

`apply_changed_runtime_preferences` currently calls `WakeWordApplicationRuntime::apply_enabled_setting`, which changes the runtime phase but does not start, stop, or restart the native listener thread. This can produce false UI/diagnostic state:

- toggling Wake on can leave the runtime in `Loading` without a listener;
- toggling Wake off can report `Disabled` while a previously started listener may still own or consume microphone capture;
- changing input devices while Wake is enabled does not restart the listener on the new device;
- enabling Wake during an active manual conversation can leave a pending `Loading` state with no subsequent listener start.

### 3.2 Manual conversation and Wake listener share capture without an intentional transfer boundary

Manual `start_conversation` suspends the runtime state but does not intentionally stop the native listener thread before normal command capture replaces the shared `AudioCapture`. The listener can observe capture shutdown as a failure, move Wake to `Error`, and prevent the normal post-command restart path from running.

### 3.3 Wake handoff policy is ambiguous and can fail after trigger

The current wake handoff requires a local Moonshine ASR pipeline. The product/spec/docs describe routing to the existing normal command ASR path. If the selected ASR mode is unsupported, Wake should not be enabled as if it will work and then fail only after a trigger.

This remediation must choose and enforce exactly one policy:

- **Policy A: provider-neutral Wake command handoff.** Wake handoff works with every supported normal command ASR mode, including Gemini Live audio and local Moonshine modes.
- **Policy B: explicit local-ASR-only Wake V1.** Wake handoff is intentionally limited to local Moonshine command ASR for V1. The app must prevent or clearly warn against enabling Wake with unsupported ASR modes, docs must state the limitation, and acceptance must verify unsupported modes fail before listening or before enablement, not after trigger.

Policy A is preferred when practical because it matches the earlier remediation spec. Policy B is acceptable only if the product deliberately narrows V1 and records that decision in code, Settings UI, docs, and acceptance evidence.

### 3.4 Downstream first-command-word acceptance is missing

Existing WWR-310 evidence proves buffer ordering and single-use handoff boundaries. It explicitly does not prove real or reproducible downstream ASR acceptance for an utterance such as `Hey Moose, tell me the time`, and it does not prove the first command word is transcribed by the downstream ASR path.

### 3.5 Clean-install artifact provisioning is incomplete

Production startup resolves Wake model/runtime paths under app data. CI prepares those paths explicitly, but the Tauri bundle configuration and startup code do not appear to install or copy Wake model/runtime artifacts for a clean user installation. A user-visible Settings toggle is not user-ready if enabling it only works after undocumented developer scripts have populated app data.

### 3.6 Performance evidence is narrower than production idle listening

The current KWS measurement observes an initialized KWS session and generated corpus execution. It does not fully measure the live listener thread, shared `AudioCapture`, canonicalization, router retention, KWS feed loop, listener event loop, and restart behavior as a production idle-listening path.

### 3.7 Documentation is stale or contradictory

Some Wake docs still describe final closeout as pending or state that Wake Word V1 is not production-qualified, while the canonical remediation TODO says the work is closed. Documentation must converge on the actual supported product state after this remediation.

## 4. Required design

### 4.1 Native listener control plane

Introduce one explicit control boundary for native listener lifecycle. It may be a new `WakeWordNativeListenerController` or a focused refactor of `wake_word_state.rs`, but it must provide one authoritative API used by startup, Settings, manual conversation, wake-triggered conversation, shutdown, and tests.

Required operations:

- `apply_settings_change(previous, next, context)` or equivalent:
  - starts the native listener when Wake is enabled, artifacts are available, and the app is idle;
  - stops the native listener when Wake is disabled;
  - restarts the native listener when input-device selection changes while Wake is enabled and idle;
  - records a pending enable/restart when Wake is enabled during an active conversation;
  - preserves rollback semantics if later settings persistence or unrelated runtime preference application fails.
- `suspend_for_command_ownership(reason)`:
  - intentionally stops the native listener before normal command ASR owns capture;
  - distinguishes intentional shutdown from capture failure;
  - must not record Wake `Error` for intentional command transfer.
- `resume_after_command_ownership(outcome, latest_settings)`:
  - restarts the listener if Wake remains enabled and the app is idle;
  - stays disabled if the user disabled Wake during the interaction;
  - handles success, cancellation, recoverable failure, and start failure.
- `shutdown()`:
  - stops the thread, releases capture, and moves runtime toward `ShuttingDown` without emitting misleading recoverable error state.

The runtime phase must not be the sole source of truth for physical listener ownership. If diagnostics expose `enabled` or `listening`, that state must reflect both runtime phase and listener ownership, or the diagnostics must explicitly distinguish `runtime_phase` from `native_listener_state`.

### 4.2 Settings behavior

Settings UI must be backed by the real listener control plane:

- Turning Wake on from Settings starts the listener or reports a bounded, sanitized actionable failure.
- Turning Wake off from Settings stops the listener and releases microphone capture before reporting disabled/listening stopped.
- Changing input device while Wake is enabled restarts the listener on the selected device, or records an actionable failure without leaving a stale listener on the old device.
- Enabling Wake during an active conversation must be deterministic: either defer startup until the conversation terminal boundary or present a bounded UI state such as `Pending until conversation ends`.
- Rollback must restore both runtime state and listener state, not just persisted settings fields.

### 4.3 Command ASR handoff policy

The implementation must enforce the selected policy from section 3.3.

For Policy A, the command path must accept `WakeCommandHandoffAudio` through the selected normal command ASR mode:

- local Moonshine modes receive the handoff before live microphone PCM;
- Gemini Live audio receives equivalent wake handoff audio before or with the first live audio sent to the provider;
- unsupported modes fail before the listener begins, not after trigger;
- one wake event creates at most one command interaction.

For Policy B, the product must explicitly constrain Wake V1:

- Settings must prevent enabling Wake when the current command ASR mode is unsupported, or present an explicit warning and require switching to a supported local ASR mode;
- switching from a supported ASR mode to an unsupported mode while Wake is enabled must disable Wake or suspend the listener with a clear status;
- docs must state the limitation;
- acceptance must prove unsupported modes cannot enter a misleading listening state.

In both policies, there must be no hidden cloud or full-ASR fallback for idle wake detection.

### 4.4 End-to-end downstream command acceptance

Add deterministic acceptance that proves wake handoff audio reaches downstream command ASR without first-word clipping.

Minimum accepted evidence:

- A reproducible fixture representing `Hey Moose, tell me the time` or an equivalent wake phrase plus immediate command.
- The KWS trigger occurs on the wake phrase.
- The handoff includes wake phrase tail plus command audio in chronological order.
- The selected command ASR test boundary receives one continuous utterance.
- The first command word after the wake phrase is present in the downstream command ASR acceptance result.
- The test fails if the first command word is clipped, reordered, duplicated, or omitted.

If real ASR transcription is too expensive or nondeterministic for every CI run, the suite may use a deterministic downstream ASR harness that validates audio boundaries plus a smaller scheduled or manually dispatched real-ASR acceptance. The final closeout must be honest about which layer each gate proves.

### 4.5 Clean-install artifact provisioning

Wake V1 must have one documented and tested artifact provisioning model:

- **Bundled/offline model:** Wake model/runtime resources are included in app resources, copied/verified into app data on first use, and never downloaded silently.
- **Explicit installer model:** Settings exposes an install/preparation flow for Wake artifacts, verifies hashes before use, and does not allow misleading `Listening` status until installed.
- **Developer-only model:** Wake remains hidden or clearly marked developer-prepared; docs and UI do not call it user-ready.

The selected model must be encoded in tests and docs. Clean-install acceptance must start from empty Wake app-data directories and prove the selected model works or fails closed with correct UI/diagnostics.

### 4.6 Production idle performance evidence

Performance acceptance must include the production listener path, not only an initialized KWS session. Evidence must cover:

- listener thread startup duration;
- listener idle CPU while capture is active and frames are routed;
- memory overhead after startup and after repeated cycles;
- KWS inference duration or route latency under representative frames;
- wake detection to command-ASR activation latency;
- pre-roll handoff startup latency;
- comparison against a continuous full-ASR idle path using the same measurement environment where feasible;
- repeated enable/disable and wake/command/resume cycles without listener/thread/capture multiplication.

### 4.7 Documentation and UI truthfulness

Docs and UI must match the actual final policy:

- Wake is not described as fully user-ready unless clean-install artifact provisioning, settings control, supported ASR policy, and downstream acceptance are complete.
- If Wake is local-ASR-only, that limitation is visible in Settings and docs.
- If Wake supports provider-neutral handoff, docs must explain what command ASR may receive and when cloud providers may process post-trigger command audio.
- Stale references to pending or completed WWR sections must be reconciled.
- Diagnostics and logs must not imply the microphone is inactive if a listener thread is still active.

### 4.8 Final audit and gate policy

A final closeout cannot rely solely on component tests for integrated requirements. The final audit must explicitly map every reopened issue to source, tests, workflow run IDs, and exact SHAs.

Required final gates:

- ordinary CI;
- settings/listener lifecycle tests;
- manual conversation capture-transfer tests;
- selected ASR policy acceptance;
- downstream first-command-word acceptance;
- clean-install artifact provisioning acceptance;
- production idle listener performance evidence;
- privacy/source audit;
- documentation audit;
- required-gates audit;
- exact PR-head and exact merged-master verification.

Skipped required gates do not count as passing.

## 5. Acceptance bar

This remediation is complete only when:

1. Wake Settings controls real listener ownership and capture ownership.
2. Manual and wake-triggered conversation paths use deterministic shared-capture transfer boundaries.
3. The selected ASR support policy is enforced before listening or enablement, not after trigger failure.
4. First command word preservation is proven at the downstream command-ASR boundary.
5. A clean install has a supported model/runtime provisioning path or Wake is truthfully marked not user-ready.
6. Production listener performance is measured and recorded.
7. Docs and UI are internally consistent.
8. Exact-head and exact-master gates pass and are recorded.
9. The post-closeout TODO is reconciled without reopening an evidence-only loop.
