# WPCR-800 Required Gates Evidence

**Date:** 2026-09-26
**Scope:** Wake Word V1 post-closeout required-gate inventory update.
**Base master:** `fccee90023f6c86b47cb705fac0f0b1c580b8c07`
**Branch:** `ralph/wpcr-800-required-gates-20260926`

## Summary

This evidence note records the WPCR-800 required-gate inventory work for the post-closeout remediation checklist. The slice updates the machine-readable gate manifest, the gate documentation, and the manifest audit so the final closeout policy explicitly names the reopened WPCR acceptance areas instead of relying only on the historical WWR gate inventory.

## Files updated

- `docs/wake-word-required-gates.json`
  - Adds final-closeout entries for the reopened WPCR acceptance gates:
    - `settings_listener_lifecycle_acceptance`
    - `manual_shared_capture_transfer_acceptance`
    - `selected_asr_policy_acceptance`
    - `downstream_first_command_word_acceptance`
    - `clean_install_artifact_provisioning_acceptance`
    - `production_listener_performance_acceptance`
  - Keeps exact-head requirements and `skipped_conclusion_counts_as_pass: false` for every required gate.

- `scripts/check_wake_word_required_gates.mjs`
  - Extends the required ID set so the audit fails if any reopened WPCR gate is removed from the manifest.
  - Preserves validation that implemented gates reference workflow files with pull-request triggers and do not use `continue-on-error: true`.
  - Preserves validation that skipped workflow conclusions do not count as accepted evidence.

- `docs/WAKE_WORD_V1_CI_GATES.md`
  - Documents the reopened WPCR gate inventory and distinguishes component, deterministic integrated, real native, and product-level acceptance scopes.
  - Clarifies that the post-closeout WPCR checklist is the authoritative final closeout scope.

## Evidence boundaries

This slice is gate-inventory/policy work. It does not claim that every reopened acceptance scenario has passed. It makes final closeout fail-closed unless exact-head and exact-master evidence is recorded for the required WPCR gates.

The updated manifest/audit must pass at exact PR head before merge. After merge, exact-master CI and the required-gates audit must pass before this evidence is used for TODO reconciliation.
