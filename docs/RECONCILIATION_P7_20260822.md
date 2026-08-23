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

The remaining V1R-035 final hide/appearance transition row belongs to V1R-076 below and is not accepted by this reconciliation yet.

## Current closure candidate: V1R-074 through V1R-076

The next source checkpoint implements the remaining P7 criteria. These rows remain **candidate/pending CI acceptance** until the new commit is reported green.

- V1R-074: V1 explicitly uses no secondary model classifier; deterministic local policy remains authoritative.
- V1R-075: ambient model output is locally bounded, disabled memory/observer data remains excluded, and provider/TTS failure never creates a fallback remark.
- V1R-076: ambient-only appearance follows appear → think/speak → short idle → hide, never calls native window focus APIs, and remains interruptible by explicit user actions.

See `docs/AMBIENT_V1_POLICY.md` for the implementation contract and deferral rationale.
