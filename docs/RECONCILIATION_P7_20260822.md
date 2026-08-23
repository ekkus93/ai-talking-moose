# Phase P7 reconciliation — 2026-08-22

This reconciliation records evidence for the ambient-behavior work without rewriting the legacy monolithic TODO file through a whole-file GitHub contents replacement.

## Accepted checkpoint: V1R-070 through V1R-073

Accepted base commit: `24ffcf9a71361c110da127d9ea0657e14af198f2`.

The user reported GitHub Actions run `32608694686` passing for that checkpoint. On that accepted base:

### V1R-070 — Central ambient scheduler

- [x] One bounded Rust scheduler owns ambient request serialization and timing.
- [x] Scheduler starts with application setup and is stopped/awaited during application shutdown.
- [x] Deterministic local policy checks mute, unsolicited-comments, quiet-hours, and conversation state before generation.
- [x] Ambient scheduling never starts microphone capture.
- [x] Current and queued ambient work is cancellation-safe and explicit user actions interrupt in-flight work.

### V1R-071 — Cooldown/dedup policy

- [x] Cooldown primitives exist.
- [x] Event categories are normalized before policy evaluation.
- [x] Near-identical event summaries are reduced to SHA-256 fingerprints; raw summaries are not retained in dedup history.
- [x] Deterministic tests cover duplicate suppression and the dedup expiry boundary.

### V1R-072 — Annoyance budget

- [x] Annoyance primitives exist.
- [x] Successful ambient speech, dismissal, and interruption update one recovering budget.
- [x] A hard maximum of 12 unsolicited remarks per hour is enforced even when persisted configuration is higher.
- [x] Recovery/decay behavior is covered by deterministic tests.

### V1R-073 — Deterministic should-speak policy

- [x] Local policy evaluates privacy, mute, active conversation, unsolicited toggle, quiet-hours, cooldown, annoyance, dismissal cooldown, deduplication, hourly budget, and event importance.
- [x] Model-controlled text cannot override a local deny result.
- [x] Diagnostic decisions contain only normalized event category and reason metadata, not the private event summary.

### Cross-phase rows closed by the accepted P7 base

- [x] V1R-038: application shutdown cancels and awaits the ambient scheduler.
- [x] V1R-035: deterministic dismissal-cooldown coverage proves immediate ambient reappearance is suppressed.

## Accepted checkpoint: V1R-074 through V1R-076

Accepted implementation commit: `52a1b3bef3e2e3b2eac89681d8fcc7a425409df8`.

The user reported GitHub Actions run `32609781814` passing for this checkpoint. GitHub reports the frontend-quality job, Rust-quality job, dependency/security audits, and both macOS arm64/x86_64 Tauri smoke-bundle jobs completed successfully. This closes the remaining P7 rows:

### V1R-074 — Optional classifier decision

- [x] V1 deliberately uses no secondary model classifier after local gating.
- [x] The deterministic Rust policy is authoritative and remains safe when model providers are unavailable.
- [x] A future classifier, if introduced, must run only after local allow, use separately bounded/minimized context and a typed output schema, and never override a local deny.

### V1R-075 — Safe ambient generation

- [x] Ambient generation uses only bounded/minimized event context and optional memory facts when memory is enabled.
- [x] Application/window-title observations are excluded unless their explicit privacy gates are enabled; unknown categories fail closed.
- [x] Tool results and transcript history have no ambient PromptBuilder input surface.
- [x] Provider output is trimmed and hard-capped locally at 320 Unicode scalar values; empty output is dropped.
- [x] Provider/TTS failure clears transient presentation state and never fabricates a fallback remark.
- [x] Speech-bubble text is emitted only after TTS audio has successfully queued.

### V1R-076 — Ambient appearance lifecycle

- [x] Ambient-only appearance follows appear → think/speak → short idle → hide.
- [x] Talking duration is bounded by queued-audio duration with a hard 10-second ceiling, followed by a 750 ms idle interval.
- [x] The ambient path does not call native show/focus/raise/always-on-top APIs and therefore does not request focus.
- [x] Explicit conversation start, mute, dismissal, barge-in, and application shutdown interrupt the central scheduler.
- [x] Dismissal records cooldown/annoyance state so ambient behavior cannot immediately reappear.
- [x] The frontend does not re-show stale ambient bubble text after the backend lifecycle clears it.

### Cross-phase row closed by V1R-076

- [x] V1R-035: final hide/appearance lifecycle transition is implemented and regression-covered.

## Phase P7 status

**P7 ambient behavior is accepted complete.** The legacy monolithic `docs/TODO(20260818-163801).md` still contains stale unchecked P7 rows; this reconciliation is authoritative for V1R-070 through V1R-076 until those rows are mechanically folded back into the monolithic tracker.

See `docs/AMBIENT_V1_POLICY.md` for the classifier decision, safe-generation contract, and appearance-lifecycle rationale.
