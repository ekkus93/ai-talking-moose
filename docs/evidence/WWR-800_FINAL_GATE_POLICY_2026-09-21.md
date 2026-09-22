# WWR-800 — Final Wake Word gate policy evidence

**Date:** 2026-09-21
**Scope:** Make the specialized-gate inventory and final merge-eligibility policy executable rather than documentation-only.

## Gate

`.github/workflows/wake-word-final-gate-policy.yml` runs `scripts/check_wake_word_final_gate_policy.mjs` on the exact workflow SHA. The check fails closed if an implemented specialized Wake Word workflow disappears, if a specialized workflow loses pull-request or master-push coverage, or if the gate documentation stops stating that ordinary CI and skipped workflows are insufficient final acceptance evidence.

The check also requires the remediation final-qualification section to retain explicit Linux real KWS, macOS arm64 real KWS, native packaging, integrated lifecycle, performance, privacy/security, and documentation requirements.

## Boundary

This is a policy/inventory gate. It does not manufacture the still-missing real positive/negative corpus, Linux/macOS native inference acceptance, production audio soak, or measured performance evidence. Those acceptance runs remain mandatory and cannot be converted into a pass by this policy workflow.
