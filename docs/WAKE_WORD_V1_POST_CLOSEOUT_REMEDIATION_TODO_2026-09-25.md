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

### Tasks

- [ ] Reload current `master` and record exact baseline SHA.
- [ ] Record the code-review findings that reopened this remediation.
- [ ] Preserve links to the prior closeout TODO, final implementation SHA, and exact-master final-gate runs.
- [ ] Identify all code paths touched by the reopened issues:
  - [ ] Settings persistence/runtime preference path.
  - [ ] Wake native listener state path.
  - [ ] manual `start_conversation` path.
  - [ ] wake-triggered conversation path.
  - [ ] local Moonshine ASR path.
  - [ ] Gemini Live audio path, if provider-neutral support is selected.
  - [ ] artifact provisioning path.
  - [ ] performance evidence path.
  - [ ] documentation and UI surfaces.
- [ ] Add a short evidence note under `docs/evidence/` describing why this post-closeout remediation exists.

### Acceptance

- [ ] Baseline evidence file is merged.
- [ ] The new spec and TODO are referenced by the evidence note.
- [ ] No production behavior changes are included in WPCR-000 unless required by repository formatting or doc policy.

## WPCR-100 — Build one native listener control plane

### Tasks

- [ ] Introduce one authoritative listener control boundary in `src-tauri/src/app/wake_word_state.rs` or a focused new module.
- [ ] Ensure startup, Settings changes, manual conversation, wake-triggered conversation, shutdown, and tests use the same listener lifecycle API.
- [ ] Add explicit listener states or diagnostics sufficient to distinguish:
  - [ ] runtime disabled;
  - [ ] runtime loading;
  - [ ] listener starting;
  - [ ] listener active/listening;
  - [ ] listener intentionally suspended for command ownership;
  - [ ] listener pending until conversation ends;
  - [ ] listener failed closed;
  - [ ] listener stopped.
- [ ] Ensure runtime `Disabled` is never reported as proof that microphone capture is stopped unless the listener thread has actually stopped.
- [ ] Make intentional listener shutdown distinct from capture failure.
- [ ] Prevent intentional command-transfer shutdown from recording Wake `Error`.
- [ ] Keep the native KWS session local to the listener thread.
- [ ] Preserve fail-closed behavior for artifact, runtime, architecture, and capture failures.
- [ ] Add unit tests for listener state transitions without real audio hardware.

### Acceptance

- [ ] There is one public/internal control API for listener lifecycle.
- [ ] Existing direct lifecycle call sites are migrated or explicitly justified.
- [ ] Tests prove intentional shutdown does not become a Wake error.
- [ ] Diagnostics cannot say Wake is disabled/listening incorrectly relative to listener ownership.

## WPCR-110 — Wire Settings enable/disable to real listener ownership

### Tasks

- [ ] Update `apply_changed_runtime_preferences` so Wake setting changes call the listener control plane, not only `apply_enabled_setting`.
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

- [ ] Detect input-device changes while Wake is enabled.
- [ ] Restart the listener on the new input device when idle.
- [ ] If a conversation is active, record a pending restart and apply it at the terminal boundary.
- [ ] If restart fails, fail Wake closed with sanitized status and keep manual interaction available.
- [ ] Detect ASR-mode changes while Wake is enabled.
- [ ] Enforce the selected WPCR-300 ASR policy when ASR mode changes.
- [ ] Add tests for input-device restart.
- [ ] Add tests for input-device restart failure.
- [ ] Add tests for ASR mode change while Wake is enabled.

### Acceptance

- [ ] Wake does not keep listening on a stale input device after a successful device change.
- [ ] Wake does not enter a misleading listening state for an unsupported ASR mode.
- [ ] Diagnostics explain pending/restart/failure state without leaking paths or secrets.

## WPCR-200 — Fix manual conversation shared-capture transfer

### Tasks

- [ ] Before manual `start_conversation`, intentionally stop or suspend the native Wake listener through the listener control plane.
- [ ] Ensure the listener thread has released or is guaranteed not to use `AudioCapture` before normal command ASR starts capture.
- [ ] Preserve manual start behavior when Wake is disabled, loading, failed, or unavailable.
- [ ] On conversation start failure, resume or restart Wake according to latest settings.
- [ ] On conversation terminal success, cancellation, recoverable failure, and stop, restart Wake according to latest settings.
- [ ] Ensure `stop_conversation` uses the same resume/restart boundary as natural lifecycle completion.
- [ ] Ensure barge-in/cancel paths do not leave Wake permanently suspended.
- [ ] Add tests for manual start while Wake listener is active.
- [ ] Add tests for manual start failure while Wake was active.
- [ ] Add tests for stop/cancel/recoverable failure restart.
- [ ] Add tests that no capture failure is recorded for intentional manual transfer.

### Acceptance

- [ ] Manual interaction remains available and reliable regardless of Wake state.
- [ ] Manual interaction does not strand Wake in `Error` after normal command completion.
- [ ] No duplicate microphone streams are opened.
- [ ] No path leaves Wake permanently suspended unintentionally.

## WPCR-300 — Resolve Wake command-ASR policy mismatch

### Decision task

- [ ] Choose one supported Wake command-ASR policy and record it in code, docs, and evidence:
  - [ ] Policy A: provider-neutral handoff to every supported normal command ASR mode.
  - [ ] Policy B: explicit local-Moonshine-only Wake V1.

### Policy A implementation tasks

- [ ] Add a provider-neutral command-ASR handoff boundary for `WakeCommandHandoffAudio`.
- [ ] Local Moonshine modes receive handoff before subsequent live microphone PCM.
- [ ] Gemini Live audio receives equivalent handoff audio before or with first live provider audio.
- [ ] Unsupported ASR modes fail before listener startup or enablement.
- [ ] Add tests for Moonshine handoff.
- [ ] Add tests for Gemini Live audio handoff.
- [ ] Add tests for unsupported modes.

### Policy B implementation tasks

- [ ] Explicitly define supported ASR modes for Wake V1.
- [ ] Settings prevents enabling Wake with unsupported ASR modes, or requires switching to a supported local ASR mode.
- [ ] ASR-mode changes to unsupported modes disable or suspend Wake with visible status.
- [ ] Wake listener cannot start when unsupported ASR mode is selected.
- [ ] Docs and UI disclose local-ASR-only behavior.
- [ ] Add tests for enable blocked by unsupported ASR mode.
- [ ] Add tests for ASR-mode switch while Wake is enabled.

### Acceptance

- [ ] Wake cannot fail only after trigger because the selected ASR mode was unsupported.
- [ ] Product docs and Settings UI match the selected policy.
- [ ] No hidden cloud or full-ASR fallback exists for idle wake detection.

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

### Decision task

- [ ] Choose one artifact provisioning model and record it in docs and code:
  - [ ] bundled/offline resources copied into app data;
  - [ ] explicit user/developer installer flow;
  - [ ] developer-only Wake feature hidden or clearly marked not user-ready.

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

- [ ] Hide or clearly mark Wake as developer-prepared/not user-ready.
- [ ] Docs must state exactly how artifacts are prepared.
- [ ] UI must not imply the feature works on a clean install.

### Acceptance

- [ ] Empty Wake app-data directories do not produce misleading listener status.
- [ ] Clean-install behavior is tested.
- [ ] Artifact verification remains fail-closed.
- [ ] No silent network download is introduced unless explicitly approved and disclosed.

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

### Tasks

- [ ] Add listener ownership/status fields to diagnostics or otherwise prevent misleading `enabled`/`disabled` state.
- [ ] Display pending, unsupported-ASR, missing-artifacts, startup-failed, and listener-active states clearly in Settings.
- [ ] Keep sanitized help text free of raw paths, credentials, and audio content.
- [ ] Ensure UI disclosure changes according to selected ASR policy.
- [ ] Ensure active microphone disclosure appears whenever the native listener can be active.
- [ ] Add frontend tests for all new status states.
- [ ] Add backend serialization tests proving diagnostics remain privacy-safe.

### Acceptance

- [ ] The UI cannot report Wake as fully disabled while the listener is still active.
- [ ] The UI cannot report Wake as listening when artifacts/ASR mode/listener state make listening impossible.
- [ ] Privacy disclosures remain visible and accurate.

## WPCR-700 — Reconcile Wake documentation

### Tasks

- [ ] Update `docs/WAKE_WORD_V1.md` to match the post-remediation support state.
- [ ] Update `docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md` to remove stale pending-closeout language or accurately describe remaining limits.
- [ ] Update `docs/WAKE_WORD_V1_ARCHITECTURE.md` for listener control plane, Settings, manual transfer, selected ASR policy, and artifact provisioning.
- [ ] Update `docs/WAKE_WORD_V1_CI_GATES.md` for new gates.
- [ ] Update README only if the feature is genuinely user-ready after this remediation.
- [ ] Ensure docs do not contradict the canonical TODO.
- [ ] Add documentation audit rules for stale pending/final-closeout contradictions.

### Acceptance

- [ ] Documentation audit fails on stale claims found in the code review.
- [ ] Docs accurately distinguish production behavior, accepted evidence, and remaining limits.
- [ ] User-facing docs do not advertise unsupported ASR modes, missing clean-install artifacts, or unmeasured performance.

## WPCR-800 — Add required CI gates for reopened issues

### Tasks

- [ ] Add or extend a Settings/listener lifecycle workflow or ordinary CI test coverage.
- [ ] Add or extend a shared-capture manual conversation transfer workflow/test suite.
- [ ] Add selected ASR policy acceptance to CI.
- [ ] Add downstream first-command-word acceptance to CI.
- [ ] Add clean-install artifact provisioning acceptance to CI.
- [ ] Add production idle listener performance evidence gate.
- [ ] Update `docs/wake-word-required-gates.json` so final closeout requires the new gates.
- [ ] Ensure skipped required gates fail the required-gates audit.
- [ ] Ensure path filters include the new source, docs, scripts, and workflow files.

### Acceptance

- [ ] Required-gates audit proves the new gates are mandatory for final closeout.
- [ ] CI names and reports clearly distinguish component, deterministic integrated, real native, and product-level acceptance.

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
