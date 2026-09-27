# WPCR-500 Performance Scope Evidence

**Date:** 2026-09-27
**Remediation item:** WPCR-500 — Measure production idle listener performance
**Base master:** `ee15b41c20b80a7b07dd4e63e8c395f0ea4df134`

## Scope

This evidence records report-hardening work for WPCR-500. It does not claim final production native-listener performance acceptance.

The pre-existing `docs/wake-word-performance-evidence.json` report is accepted for the earlier WWR-630 baseline because it contains Linux x86_64 and macOS arm64 real KWS session measurements plus deterministic lifecycle and continuous-ASR comparison evidence. WPCR-500 reopened the performance requirement with a narrower demand: the report must distinguish KWS-only or deterministic lifecycle measurements from full production native-listener measurements.

## Changes

This slice updates `docs/wake-word-performance-evidence.json` and `scripts/check_wake_word_performance_evidence.mjs` so that:

- the report explicitly records `wpcr500_scope.production_listener_measurement_required: true`;
- the report explicitly records `wpcr500_scope.production_listener_status: pending_measurement`;
- the report explicitly states that accepted WWR-630 evidence is not full WPCR-500 closeout;
- each measurement records a `measurement_path` describing whether it is a standalone real-KWS session, deterministic lifecycle harness, continuous-ASR comparison, or accepted WWR-630 composite;
- the validator fails if accepted WWR-630 measurements are mislabeled as full production native-listener measurements.

## Evidence boundary

This slice satisfies the report-scope and anti-overclaim portion of WPCR-500. It does not satisfy the still-open production listener measurements for startup duration, active-capture idle CPU, memory overhead, route/inference latency under representative frames, wake-to-command activation, pre-roll startup, repeated enable/disable and wake/command/resume cycles, or no resource multiplication across those cycles.

Final WPCR-500 closeout still requires representative production-listener measurements and exact-head/exact-master validation.
