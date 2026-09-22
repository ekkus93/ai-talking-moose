# WWR-700 README Truthfulness Audit Guard — 2026-09-22

## Scope

This evidence records a documentation-audit hardening slice for Wake Word V1 truthfulness.

The README is a broad user-facing entry point. At this stage Wake Word V1 has implementation and policy guardrails, but real redistributable audio fixtures, real Linux/macOS KWS acceptance, measured performance, and integrated lifecycle/soak acceptance remain pending. The README therefore must not accidentally imply that Wake Word V1 is fully accepted or production-ready.

## Change

`scripts/check_wake_word_documentation.mjs` now reads `README.md` and enforces two boundaries:

1. If the README contains an unqualified Wake Word acceptance claim, the documentation audit fails.
2. If the README mentions Wake Word in the future, it must include an explicit pending/qualification boundary unless the acceptance state has been updated deliberately.

The guard is intentionally phrased so the current README, which does not advertise Wake Word, remains valid while future README edits cannot silently overstate the feature.

## Non-claims

This does not complete real Wake Word acceptance. It does not claim Linux x86_64 or macOS arm64 real KWS acceptance, corpus acceptance, performance acceptance, or integrated lifecycle acceptance.
