# WPCR-000 — post-closeout remediation baseline

Date: 2026-09-25
Baseline master reloaded for this evidence: `9e35b47c85278646e8933f5bb601fe312a8d1b4b`
Post-closeout review source: `0ea5e03f9012884e2858d7b478fde916f0f163d7`

## Why this remediation exists

Wake Word V1 was marked closed by `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`, with final implementation evidence recorded for `99e216e13f78c1a0605684801d89dbd6b33ca7c9` and final documentation closeout merged at `0ea5e03f9012884e2858d7b478fde916f0f163d7`.

A post-closeout review found that several product-level requirements were over-closed by component evidence or by runtime-state evidence that did not prove physical listener ownership. The reopened remediation is governed by:

- Spec: `docs/WAKE_WORD_V1_POST_CLOSEOUT_REMEDIATION_SPEC_2026-09-25.md`
- TODO: `docs/WAKE_WORD_V1_POST_CLOSEOUT_REMEDIATION_TODO_2026-09-25.md`

## Reopened findings

The review reopened the following mandatory issues:

1. Settings changes updated `WakeWordApplicationRuntime` phase but did not control the native listener thread.
2. Manual conversation startup did not intentionally transfer shared `AudioCapture` ownership away from the Wake listener before command ASR startup.
3. Wake-triggered command handoff required local Moonshine ASR even though prior docs/spec text described the existing normal command ASR path.
4. WWR-310 first-command-word downstream ASR acceptance was not proven by the recorded evidence.
5. Clean-install Wake model/runtime provisioning was not proven for normal user installs.
6. Performance evidence measured KWS session behavior more strongly than the full production idle listener path.
7. Documentation and UI state could be stale, contradictory, or misleading relative to actual listener/capture ownership.
8. Final audit and required gates did not distinguish every product-level requirement from component-only evidence.

## Affected code paths

The reopened scope includes these source and evidence areas:

- Settings persistence/runtime preferences: `src-tauri/src/commands/settings.rs`, `src-tauri/src/app/runtime_preferences.rs`.
- Native listener lifecycle: `src-tauri/src/app/wake_word_state.rs`, `src-tauri/src/app/wake_word_local_listener_thread.rs`.
- Manual conversation startup and terminal lifecycle: `src-tauri/src/commands/conversation/core.rs`, `src-tauri/src/conversation/session.rs`.
- Wake-triggered conversation handoff: `WakeCommandHandoffAudio`, local ASR handoff, and selected ASR policy tests.
- Artifact provisioning: `wake-word-artifacts.json`, `scripts/prepare_wake_word_model.py`, `scripts/prepare_wake_word_runtime.py`, Tauri resources, and app-data startup behavior.
- Performance evidence: real listener startup, idle capture routing, route latency, handoff latency, and repeated-cycle reports.
- Documentation/UI/diagnostics: Wake docs, Settings Wake panel, diagnostics payloads, documentation audit rules, and required gate manifests.

## Baseline rule

This evidence file records why the remediation was reopened and the exact current `master` observed while recording WPCR-000 evidence. It does not claim implementation closure for any reopened finding. Each WPCR section must be closed only by source changes, tests, and exact-head/exact-master evidence that directly satisfy that section's acceptance criteria.
