# Wake Word V1 Post-Closeout Remediation TODO

**Date:** 2026-09-25
**Status:** Open
**Spec:** `docs/WAKE_WORD_V1_POST_CLOSEOUT_REMEDIATION_SPEC_2026-09-25.md`
**Source review:** post-closeout code review of `master` at `0ea5e03f9012884e2858d7b478fde916f0f163d7`
**Scope:** Fix the integration, acceptance, packaging, performance, and documentation gaps found after the Wake Word V1 closeout.

## Execution rules

- Treat this file as the authoritative checklist for post-closeout remediation.
- Do not weaken artifact verification, privacy boundaries, or exact-head discipline for speed.
- Prefer coherent vertical-slice PRs when several tasks share listener lifecycle, Settings, capture ownership, or acceptance-gate code.
- Do not mark a task complete because a component test exists if the task asks for integrated production behavior.
- Do not count skipped required gates as passing.
- Bind evidence to exact commit SHAs and run IDs.
- After every merge, reload this TODO from `master` and re-evaluate remaining work.

## WPCR-000 — Reopen and freeze post-closeout remediation baseline

**Evidence:** baseline evidence merged in `docs/evidence/WPCR-000_POST_CLOSEOUT_BASELINE_2026-09-25.md` on `master` at `362ec673d0195fb360a3ffc32fd22ad616cdb169`; ordinary CI run `36230401074` passed for that exact master.

### Tasks

- [x] Reload current `master` and record exact baseline SHA.
- [x] Record the code-review findings that reopened this remediation.
- [x] Preserve links to the prior closeout TODO, final implementation SHA, and exact-master final-gate runs.
- [x] Identify all code paths touched by the reopened issues:
  - [x] Settings persistence/runtime preference path.
  - [x] Wake native listener state path.
  - [x] manual `start_conversation` path.
  - [x] wake-triggered conversation path.
  - [x] local Moonshine ASR path.
  - [x] Gemini Live audio path, if provider-neutral support is selected.
  - [x] artifact provisioning path.
  - [x] performance evidence path.
  - [x] documentation and UI surfaces.
- [x] Add a short evidence note under `docs/evidence/` describing why this post-closeout remediation exists.

### Acceptance

- [x] Baseline evidence file is merged.
- [x] The new spec and TODO are referenced by the evidence note.
- [x] No production behavior changes are included in WPCR-000 unless required by repository formatting or doc policy.

## WPCR-100 — Build one native listener control plane

**Incremental evidence:** intentional listener shutdown/capture-close race fixed by PR #473 and merged as `dcc855cd16a89e37a6894ef217ba0532fc4f068b`; exact-master CI run `36235465363` passed. Shutdown evidence is recorded in `docs/evidence/WPCR-100_LISTENER_SHUTDOWN_2026-09-26.md`, merged by PR #474 as `9d0f6530eac3169500d573f1039c57aa25b4fb72`; exact-master CI run `36236017079` passed. Current source also keeps `NativeKwsSession` construction/use inside the dedicated listener thread and retains fail-closed startup/capture handling. The authoritative `control_native_wake_listener` boundary, listener-status classifier, centralized terminal completion boundary, and deterministic mock-capture listener tests are present on `master` at `e4606ee2f5a649d01468ed002d41beb09989da05`; exact-master ordinary CI `36341840706`, lifecycle stability `36341840594`, and source-security audit `36341840687` passed.

### Tasks

- [x] Introduce one authoritative listener control boundary in `src-tauri/src/app/wake_word_state.rs` or a focused new module.
- [x] Ensure startup, Settings changes, manual conversation, wake-triggered conversation, shutdown, and tests use the same listener lifecycle API.
- [x] Add explicit listener states or diagnostics sufficient to distinguish:
  - [x] runtime disabled;
  - [x] runtime loading;
  - [x] listener starting;
  - [x] listener active/listening;
  - [x] listener intentionally suspended for command ownership;
  - [x] listener pending until conversation ends;
  - [x] listener failed closed;
  - [x] listener stopped.
- [x] Ensure runtime `Disabled` is never reported as proof that microphone capture is stopped unless the listener thread has actually stopped.
- [x] Make intentional listener shutdown distinct from capture failure.
- [x] Prevent intentional command-transfer shutdown from recording Wake `Error`.
- [x] Keep the native KWS session local to the listener thread.
- [x] Preserve fail-closed behavior for artifact, runtime, architecture, and capture failures.
- [x] Add unit tests for listener state transitions without real audio hardware.

### Acceptance

- [x] There is one public/internal control API for listener lifecycle.
- [x] Existing direct lifecycle call sites are migrated or explicitly justified.
- [x] Tests prove intentional shutdown does not become a Wake error.
- [x] Diagnostics cannot say Wake is disabled/listening incorrectly relative to listener ownership.

## WPCR-110 — Wire Settings enable/disable to real listener ownership

### Tasks

- [x] Update `apply_changed_runtime_preferences` so Wake setting changes call the listener control plane, not only `apply_enabled_setting`.
- [ ] Turning Wake on while idle starts the native listener when artifacts and selected policy are valid.
- [ ] Turning Wake off stops the listener thread, releases capture, clears retained audio, and reports disabled only after the stop boundary is complete or safely in progress.
- [ ] Settings rollback restores listener state as well as runtime phase and persisted values.
- [ ] Settings failure surfaces sanitized actionable errors without raw paths, secrets, or audio content.
- [ ] UI refreshes diagnostics/status after enable/disable completes.
- [ ] Add tests for enable from disabled.
- [ ] Add tests for disable from active listening.
- [ ] Add tests for settings persistence failure rollback.
- [ ] Add tests that diagnostics do not report disabled while a listener handle remains active.

### Acceptance

- [ ] A user can enable Wake from Settings without app restart when prerequisites are satisfied.
- [ ] A user can disable Wake from Settings and microphone listener ownership stops boundedly.
- [ ] Manual conversation behavior is preserved immediately after disable.
- [ ] Existing Settings UI tests are updated to assert real backend state, not just patched frontend settings.

## WPCR-120 — Handle input-device and ASR-mode settings changes while Wake is enabled

### Tasks

- [x] Detect input-device changes while Wake is enabled.
- [ ] Restart the listener on the new input device when idle.
- [ ] If a conversation is active, record a pending restart and apply it at the terminal boundary.
- [ ] If restart fails, fail Wake closed with sanitized status and keep manual interaction available.
- [x] Detect ASR-mode changes while Wake is enabled.
- [x] Enforce the selected WPCR-300 ASR policy when ASR mode changes.
- [ ] Add tests for input-device restart.
- [ ] Add tests for input-device restart failure.
- [ ] Add tests for ASR mode change while Wake is enabled.

### Acceptance

- [ ] Wake does not keep listening on a stale input device after a successful device change.
- [ ] Wake does not enter a misleading listening state for an unsupported ASR mode.
- [ ] Diagnostics explain pending/restart/failure state without leaking paths or secrets.

## WPCR-200 — Fix manual conversation shared-capture transfer

**Incremental evidence:** current `master` routes manual start through `NativeWakeListenerControl::TransferToCommand` before normal ASR capture, uses `complete_native_wake_command_interaction` for start failure and terminal resolution, and routes explicit stop through the same boundary. Deterministic lifecycle tests cover disabled/failed Wake availability, start-failure recovery, latest-setting terminal behavior, and intentional transfer without Wake error. Exact-master ordinary CI `36346852425` passed at `cc593f8d4ae1ac8d399073c6cfb8784c8451bdab`. Active-listener ownership tests and final acceptance remain open.

### Tasks

- [x] Before manual `start_conversation`, intentionally stop or suspend the native Wake listener through the listener control plane.
- [x] Ensure the listener thread has released or is guaranteed not to use `AudioCapture` before normal command ASR starts capture.
- [x] Preserve manual start behavior when Wake is disabled, loading, failed, or unavailable.
- [x] On conversation start failure, resume or restart Wake according to latest settings.
- [x] On conversation terminal success, cancellation, recoverable failure, and stop, restart Wake according to latest settings.
- [x] Ensure `stop_conversation` uses the same resume/restart boundary as natural lifecycle completion.
- [ ] Ensure barge-in/cancel paths do not leave Wake permanently suspended.
- [ ] Add tests for manual start while Wake listener is active.
- [ ] Add tests for manual start failure while Wake was active.
- [x] Add tests for stop/cancel/recoverable failure restart.
- [x] Add tests that no capture failure is recorded for intentional manual transfer.

### Acceptance

- [ ] Manual interaction remains available and reliable regardless of Wake state.
- [ ] Manual interaction does not strand Wake in `Error` after normal command completion.
- [ ] No duplicate microphone streams are opened.
- [ ] No path leaves Wake permanently suspended unintentionally.

## WPCR-300 — Resolve Wake command-ASR policy mismatch

**Evidence:** Policy B is selected and enforced in current `master`: `wake_word_state.rs` accepts only local Moonshine streaming ASR for Wake-triggered command activation, Settings/docs disclose local-Moonshine-only behavior, and unsupported modes are rejected before listener startup/enablement. Exact-master ordinary CI `36230401074` passed at `362ec673d0195fb360a3ffc32fd22ad616cdb169`.

### Decision task

- [x] Choose one supported Wake command-ASR policy and record it in code, docs, and evidence:
  - [ ] Policy A: provider-neutral handoff to every supported normal command ASR mode.
  - [x] Policy B: explicit local-Moonshine-only Wake V1.

### Policy A implementation tasks

- [ ] Add a provider-neutral command-ASR handoff boundary for `WakeCommandHandoffAudio`.
- [ ] Local Moonshine modes receive handoff before subsequent live microphone PCM.
- [ ] Gemini Live audio receives equivalent handoff audio before or with first live provider audio.
- [ ] Unsupported ASR modes fail before listener startup or enablement.
- [ ] Add tests for Moonshine handoff.
- [ ] Add tests for Gemini Live audio handoff.
- [ ] Add tests for unsupported modes.

### Policy B implementation tasks

- [x] Explicitly define supported ASR modes for Wake V1.
- [x] Settings prevents enabling Wake with unsupported ASR modes, or requires switching to a supported local ASR mode.
- [x] ASR-mode changes to unsupported modes disable or suspend Wake with visible status.
- [x] Wake listener cannot start when unsupported ASR mode is selected.
- [x] Docs and UI disclose local-ASR-only behavior.
- [x] Add tests for enable blocked by unsupported ASR mode.
- [x] Add tests for ASR-mode switch while Wake is enabled.

### Acceptance

- [x] Wake cannot fail only after trigger because the selected ASR mode was unsupported.
- [x] Product docs and Settings UI match the selected policy.
- [x] No hidden cloud or full-ASR fallback exists for idle wake detection.

## WPCR-310 — Add downstream first-command-word acceptance

### Tasks

- [ ] Add a deterministic fixture or generated fixture equivalent to `Hey Moose, tell me the time`.
- [ ] Ensure KWS detects the wake phrase in that fixture.
- [ ] Route pre-roll plus live command audio through the production handoff boundary.
- [ ] Verify downstream command-ASR test boundary receives one continuous utterance.
- [ ] Verify the first command word after the wake phrase is present at the downstream boundary.
- [ ] Make the test fail if the first command word is clipped, duplicated, reordered, or omitted.
- [ ] If real ASR transcription is too nondeterministic for ordinary CI, add a deterministic downstream ASR harness plus a clearly scoped real-ASR/manual/scheduled gate.
- [ ] Store reports without raw PCM or transcript leakage beyond approved test strings.

### Acceptance

- [ ] WWR-310 no longer relies only on router unit tests.
- [ ] Evidence proves the downstream command path receives the first command word.
- [ ] The acceptance report states exactly whether it proves deterministic boundary receipt, real ASR transcription, or both.

## WPCR-400 — Add clean-install Wake artifact provisioning

**Evidence:** WPCR-400 selected the developer-prepared model. Current `master` records this in `wake-word-artifacts.json`, `docs/evidence/WPCR-400_ARTIFACT_PROVISIONING_MODEL_2026-09-25.md`, Wake docs, Settings UI disclosure, manifest validation, and clean-app-data fail-closed tests. Exact-master ordinary CI `36230401074` passed at `362ec673d0195fb360a3ffc32fd22ad616cdb169`; prior exact-head artifact/documentation/privacy/native packaging gates for the provisioning slice passed at `a7ff8b74ed2b4556b821e9b4047ef149b2d8b306`.

### Decision task

- [x] Choose one artifact provisioning model and record it in docs and code:
  - [ ] bundled/offline resources copied into app data;
  - [ ] explicit user/developer installer flow;
  - [x] developer-only Wake feature hidden or clearly marked not user-ready.

### Bundled/offline model tasks

- [ ] Add Wake model and runtime resources to Tauri bundle or platform-appropriate resource packaging.
- [ ] Copy resources into app data on first use or verify them in place without weakening hash checks.
- [ ] Verify every copied file against `wake-word-artifacts.json` before listener startup.
- [ ] Support Linux x86_64 and macOS arm64 according to accepted platforms.
- [ ] Add clean app-data install tests.

### Explicit installer model tasks

- [ ] Add a backend command or existing model-installer integration for Wake artifacts.
- [ ] Expose install/preparation status in Settings.
- [ ] Do not allow `Listening` until artifacts are verified.
- [ ] Support offline deterministic preparation from approved archives where applicable.
- [ ] Add installer success/failure/corrupt-cache tests.

### Developer-only model tasks

- [x] Hide or clearly mark Wake as developer-prepared/not user-ready.
- [x] Docs must state exactly how artifacts are prepared.
- [x] UI must not imply the feature works on a clean install.

### Acceptance

- [x] Empty Wake app-data directories do not produce misleading listener status.
- [x] Clean-install behavior is tested.
- [x] Artifact verification remains fail-closed.
- [x] No silent network download is introduced unless explicitly approved and disclosed.

## WPCR-500 — Measure production idle listener performance

### Tasks

- [ ] Add a benchmark or acceptance path that starts the production native listener, not just a standalone KWS session.
- [ ] Measure listener startup duration.
- [ ] Measure idle CPU while capture is active and frames are routed.
- [ ] Measure memory overhead after startup.
- [ ] Measure route or inference latency under representative frames.
- [ ] Measure wake detection to command-ASR activation latency.
- [ ] Measure pre-roll handoff startup latency.
- [ ] Compare against continuous full-ASR idle behavior in the same environment where feasible.
- [ ] Measure repeated enable/disable and wake/command/resume cycles.
- [ ] Verify no thread, listener handle, native session, ring buffer, or capture multiplication across cycles.
- [ ] Write privacy-safe performance reports.

### Acceptance

- [ ] Performance evidence covers the production listener path.
- [ ] Reports distinguish KWS-only measurement from full listener measurement.
- [ ] Final docs do not overstate performance claims beyond measured evidence.

## WPCR-600 — Fix diagnostics and Settings UI truthfulness

**Evidence:** PR #469 merged listener ownership diagnostics at `9e35b47c85278646e8933f5bb601fe312a8d1b4b` with exact-head ordinary CI, Wake source-security audit, Wake documentation audit, and Wake privacy audit passing at `47b368b23aeac9c4dfd7238d033304984d97e54d`. Current exact-master ordinary CI `36230401074` passed at `362ec673d0195fb360a3ffc32fd22ad616cdb169`.

### Tasks

- [x] Add listener ownership/status fields to diagnostics or otherwise prevent misleading `enabled`/`disabled` state.
- [x] Display pending, unsupported-ASR, missing-artifacts, startup-failed, and listener-active states clearly in Settings.
- [x] Keep sanitized help text free of raw paths, credentials, and audio content.
- [x] Ensure UI disclosure changes according to selected ASR policy.
- [x] Ensure active microphone disclosure appears whenever the native listener can be active.
- [x] Add frontend tests for all new status states.
- [x] Add backend serialization tests proving diagnostics remain privacy-safe.

### Acceptance

- [x] The UI cannot report Wake as fully disabled while the listener is still active.
- [x] The UI cannot report Wake as listening when artifacts/ASR mode/listener state make listening impossible.
- [x] Privacy disclosures remain visible and accurate.

## WPCR-700 — Reconcile Wake documentation

**Evidence:** Post-closeout documentation truthfulness updates merged through PR #476 at `d85e1a601219ff857e8e08b150bba6f249b3db7f`; exact-master ordinary CI `36249865635`, Wake documentation audit `36249865640`, Wake privacy audit `36249865636`, and Wake required-gates audit `36249865639` passed. WPCR-700 evidence note `docs/evidence/WPCR-700_POST_CLOSEOUT_DOCUMENTATION_2026-09-26.md` merged through PR #477 at `ebeb2a118c63df4f8d71b3f41476163df80dfc3a`; exact-master ordinary CI `36250002298` passed. Reopened-gate documentation was completed by PR #479 at `8b0022742288c2183be15e297393d72194a16b32`; exact-master ordinary CI `36303647425`, documentation audit `36303647388`, and required-gates audit `36303647268` passed.

### Tasks

- [x] Update `docs/WAKE_WORD_V1.md` to match the post-remediation support state.
- [x] Update `docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md` to remove stale pending-closeout language or accurately describe remaining limits.
- [x] Update `docs/WAKE_WORD_V1_ARCHITECTURE.md` for listener control plane, Settings, manual transfer, selected ASR policy, and artifact provisioning.
- [x] Update `docs/WAKE_WORD_V1_CI_GATES.md` for new gates.
- [x] Update README only if the feature is genuinely user-ready after this remediation. README was not promoted; documentation audit continues to enforce truthful non-user-ready/pending-qualification boundaries if Wake is mentioned.
- [x] Ensure docs do not contradict the canonical TODO.
- [x] Add documentation audit rules for stale pending/final-closeout contradictions.

### Acceptance

- [x] Documentation audit fails on stale claims found in the code review.
- [x] Docs accurately distinguish production behavior, accepted evidence, and remaining limits.
- [x] User-facing docs do not advertise unsupported ASR modes, missing clean-install artifacts, or unmeasured performance.

## WPCR-800 — Add required CI gates for reopened issues

**Incremental evidence:** Reopened WPCR gate inventory updates merged through PR #479 at `8b0022742288c2183be15e297393d72194a16b32`. Exact PR head `4441973809779a4d92941faa701b08a29ecfe2fd` passed ordinary CI `36266232016`, Wake required-gates audit `36266231835`, and Wake documentation audit `36266231845`. Exact merged master passed ordinary CI `36303647425`, Wake documentation audit `36303647388`, and Wake required-gates audit `36303647268`. Evidence note: `docs/evidence/WPCR-800_REQUIRED_GATES_2026-09-26.md`. The gate manifest now requires the reopened WPCR gates for final closeout, but WPCR-110 and WPCR-200 still own the remaining implementation/test proof for live Settings/listener lifecycle and manual shared-capture transfer behavior.

### Tasks

- [ ] Add or extend a Settings/listener lifecycle workflow or ordinary CI test coverage. Gate inventory is present, but full acceptance remains coupled to WPCR-110 implementation/tests.
- [ ] Add or extend a shared-capture manual conversation transfer workflow/test suite. Gate inventory is present, but full acceptance remains coupled to WPCR-200 implementation/tests.
- [x] Add selected ASR policy acceptance to CI.
- [x] Add downstream first-command-word acceptance to CI.
- [x] Add clean-install artifact provisioning acceptance to CI.
- [x] Add production idle listener performance evidence gate.
- [x] Update `docs/wake-word-required-gates.json` so final closeout requires the new gates.
- [x] Ensure skipped required gates fail the required-gates audit.
- [x] Ensure path filters include the new source, docs, scripts, and workflow files.

### Acceptance

- [x] Required-gates audit proves the new gates are mandatory for final closeout.
- [x] CI names and reports clearly distinguish component, deterministic integrated, real native, and product-level acceptance.

## WPCR-900 — Final source/privacy/security audit

### Tasks

- [ ] Audit listener ownership and Settings transitions.
- [ ] Audit manual conversation transfer and restart paths.
- [ ] Audit wake-triggered conversation handoff and selected ASR policy.
- [ ] Audit clean-install artifact provisioning.
- [ ] Audit diagnostics and logs for misleading microphone state.
- [ ] Audit logs/errors/metrics for raw audio, transcripts, credentials, and unnecessary paths.
- [ ] Audit performance reports for privacy and claim scope.
- [ ] Audit docs for stale or contradictory closeout/user-ready claims.
- [ ] Confirm no mandatory review finding remains open.

### Acceptance

- [ ] Audit evidence lists each reopened code-review finding and its closure evidence.
- [ ] Audit distinguishes implementation closure from evidence-only documentation.
- [ ] No product-level requirement is closed by component-only evidence unless explicitly scoped as component-only.

## WPCR-950 — Exact-head final qualification

### Tasks

- [ ] Reload latest `master` before final qualification.
- [ ] Review final branch diff against current `master`.
- [ ] Confirm no unrelated ASR/TTS/settings regression is introduced.
- [ ] Record exact final PR head SHA.
- [ ] Run ordinary CI at exact head.
- [ ] Run Settings/listener lifecycle gate at exact head.
- [ ] Run manual conversation shared-capture transfer gate at exact head.
- [ ] Run selected ASR policy acceptance at exact head.
- [ ] Run downstream first-command-word acceptance at exact head.
- [ ] Run clean-install artifact provisioning acceptance at exact head.
- [ ] Run production idle listener performance gate at exact head.
- [ ] Run privacy/source-security audit at exact head.
- [ ] Run documentation audit at exact head.
- [ ] Run required-gates audit at exact head.
- [ ] Record run IDs and report artifact names.

### Acceptance

- [ ] Every required exact-head gate passes.
- [ ] No skipped required gate is counted as passing.
- [ ] Evidence is bound to the exact final PR head.

## WPCR-960 — Guarded merge and exact-master verification

### Tasks

- [ ] Recheck PR mergeability immediately before merge.
- [ ] Recheck exact head SHA immediately before merge.
- [ ] Merge only the exact tested head using an allowed guarded merge method.
- [ ] Record exact merged master SHA.
- [ ] Verify ordinary CI on exact merged master.
- [ ] Verify all required Wake-specific exact-master gates.
- [ ] Verify clean-install, listener lifecycle, ASR policy, downstream command, performance, privacy, source-security, docs, and required-gates evidence on exact merged master.
- [ ] Reconcile this TODO with exact evidence.
- [ ] Do not close final status until all reopened code-review findings have implementation and acceptance evidence.

### Acceptance

- [ ] `master` contains the complete post-closeout remediation.
- [ ] Required exact-master validation evidence passes.
- [ ] This TODO is reconciled with exact SHAs and run IDs.
- [ ] Final docs truthfully describe Wake Word V1 support state.
