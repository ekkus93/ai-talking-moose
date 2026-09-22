# WWR-800 — Required Wake Gates Manifest Evidence

**Date:** 2026-09-21
**Scope:** Add a deterministic required-gates source of truth for Wake Word V1 closeout policy.

## Change

`docs/wake-word-required-gates.json` now records the implemented Wake Word policy/source gates and the still-pending specialized acceptance requirements. `scripts/check_wake_word_required_gates.mjs` validates that:

- ordinary CI is not final Wake Word V1 qualification;
- every final-closeout gate requires exact-head evidence;
- no gate treats a skipped workflow conclusion as passing evidence;
- implemented gates point at real workflow files with pull-request triggers;
- pending Linux real KWS, macOS real KWS, integrated lifecycle, and measured performance acceptance entries remain pending instead of being represented by passing policy-only gates;
- `docs/WAKE_WORD_V1_CI_GATES.md` documents every implemented and pending gate truthfully.

`.github/workflows/wake-word-required-gates.yml` runs that audit on pull requests, master pushes, and manual dispatch.

## TODO reconciliation note

This advances WWR-800 by defining a machine-readable required-gate inventory and making skipped-workflow non-acceptance enforceable in source. It does not complete WWR-610, WWR-620, WWR-630, or WWR-640 real acceptance.
