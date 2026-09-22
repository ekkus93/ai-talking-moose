# WWR-700 — Wake Word Gate Documentation Reconciliation Evidence

**Date:** 2026-09-21
**Scope:** Keep Wake Word V1 CI/acceptance gate documentation aligned with current master behavior.

## Change

`docs/WAKE_WORD_V1_CI_GATES.md` now documents:

- both deterministic corpus gates: the Node manifest checker and the Python corpus contract workflow;
- the current `docs/wake-word-corpus.json` schema boundary and the fact that schema/contract passes do not imply real audio fixture acceptance;
- the expanded privacy audit that scans production Wake Word Rust error/log surfaces for direct logging macros and sensitive outward-facing string literals.

`scripts/check_wake_word_documentation.mjs` now requires those truthfulness boundaries to remain present.

## Qualification

`node scripts/check_wake_word_documentation.mjs` passes locally against the updated documentation.

## TODO reconciliation note

This advances WWR-700 documentation truthfulness and WWR-800 gate documentation. It does not mark real corpus, native KWS, performance, or final source/security acceptance complete.
