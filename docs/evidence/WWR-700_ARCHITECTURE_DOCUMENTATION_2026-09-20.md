# WWR-700 — architecture documentation evidence

Date: 2026-09-20
Baseline master: `e113f624659a7d67678709b061ae1043a41aca68`

`docs/WAKE_WORD_V1_ARCHITECTURE.md` is the source-aligned developer architecture document for the consolidated Wake Word subsystem.

It objectively documents the following WWR-700 requirements: one authoritative AppState-owned Wake runtime; removal of duplicate runtime ownership; fixed `Hey, Moose` phrase; disabled-by-default policy; local/offline KWS policy; locally active microphone behavior while listening; wake phrase plus prompt potentially reaching command ASR; Talking suspension; no-barge-in V1; memory-only pre-roll; pinned/hash-verified model and runtime identity policy; privacy-safe diagnostics; and explicit source ownership boundaries.

The document also contains a dedicated acceptance-boundary section that prevents planned behavior from being presented as already production-qualified. It explicitly leaves continuous microphone→KWS routing, wake→command-ASR activation, gap-free handoff, real corpus/platform acceptance, lifecycle/resource stability, performance baselines, and final privacy/security audit unclaimed. It makes no subjective accuracy claims.

This evidence therefore supports the completed developer-documentation portions of WWR-700 without claiming final user-facing documentation acceptance. README/user documentation should remain gated on a usable, fully qualified production feature, and supported-platform wording must remain limited by WWR-610/620 real acceptance.
