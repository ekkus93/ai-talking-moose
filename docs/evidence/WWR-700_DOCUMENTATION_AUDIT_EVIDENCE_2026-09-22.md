# WWR-700 documentation audit evidence — 2026-09-22

## Scope

This evidence records the current documentation truthfulness guardrail for Wake Word V1. It does not claim user-facing final feature readiness or production acceptance.

## Source evidence

`docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md` documents the current disabled-by-default Wake Word behavior, fixed `Hey, Moose` phrase, local keyword spotting, local active-microphone disclosure, no-barge-in limitation, in-memory ring/pre-roll behavior, diagnostics, live enable/disable setting behavior, and one-stream microphone routing boundary.

`docs/WAKE_WORD_V1_ARCHITECTURE.md` documents the one `WakeWordApplicationRuntime` in `AppState`, single canonical `WakeWordRuntimeManager`, `AppState::audio_capture` as the authoritative microphone owner, 16 kHz mono V1 policy, one inference thread, two-second pre-roll, no-barge-in policy, and the distinction between component tests and required real-KWS/platform acceptance.

`docs/WAKE_WORD_V1_CI_GATES.md` documents ordinary CI, specialized Wake policy gates, skipped-workflow semantics, corpus-contract limitations, native-packaging limitations, pending real KWS acceptance, pending integrated lifecycle acceptance, and pending measured performance acceptance.

`src/components/Settings/WakeWordSettingsPanel.tsx` contains the user-facing local/offline KWS, active local microphone, no full-time cloud transcription, and no-barge-in disclosures checked by the documentation audit.

`scripts/check_wake_word_documentation.mjs` audits all of the above, rejects unqualified final-acceptance claims, checks README Wake Word truthfulness boundaries, and requires performance evidence to remain pending until real measurements exist.

## Validation

Exact merged-master validation for `540e49f1a600bde646724058cb67207abfaa045a` passed ordinary CI `35766949369`.

## Non-claims

This evidence does not claim that all WWR-700 checklist items are complete. README/user-doc updates for final usability, platform support claims based on real acceptance, and final user-facing production-readiness language remain constrained by the still-open WWR-610/620/630/640/950/960 acceptance tasks.