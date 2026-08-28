# Final reconciliation and tracker-integrity audit — 2026-08-28

This document closes V1R-201 by auditing the complete `docs/RECONCILIATION_*.md` set and the legacy feature tracker `docs/TODO(20260818-163801).md` against current `master`. It does not create new product behavior or retroactively convert manual/physical acceptance into automated evidence.

## Scope

The audit reviewed all reconciliation overlays present on `master`:

- `RECONCILIATION_P0_P1_P6_20260822.md`
- `RECONCILIATION_P2_P3_PHYSICAL_20260823.md`
- `RECONCILIATION_P3A_20260823.md`
- `RECONCILIATION_P5_20260823.md`
- `RECONCILIATION_P7_20260822.md`
- `RECONCILIATION_P8_20260822.md`
- `RECONCILIATION_P9_20260822.md`
- `RECONCILIATION_P10_20260822.md`
- `RECONCILIATION_P11_20260823.md`
- `RECONCILIATION_P12_20260823.md`
- `RECONCILIATION_P13_20260823.md`

Named behavioral-test citations were cross-checked against current test definitions and the claims surrounding each citation. Reconciliation rows that rely on source inspection, CI status, physical/manual reports, packaging probes, or release evidence were kept in those evidence classes rather than being rewritten to imply unit-test proof.

## Corrections found

Three stale named-test citations remained after the V1R-182 integrity repair:

1. P12's Gemini Live retry row cited the removed combined name `retry_policy_has_hard_bounds_and_explicit_close_cancels_reconnect`. The current proof is intentionally split: `retry_policy_has_hard_bounds` asserts the reconnect-attempt/elapsed/backoff ceilings, while `explicit_close_cancels_in_flight_reconnect_attempt` exercises cancellation of a pending reconnect and requires the typed `Closed` result.
2. P12's Memory-Off row cited the removed `ambient_prompt_memories_obey_memory_setting_and_restore_on_reenable`. The current production-path regression is `production_ambient_prompt_obeys_memory_privacy_gate`: it calls the production ambient prompt builder with retained memory, proves the private fact is absent while Memory is disabled, then proves the same retained fact returns after re-enabling Memory.
3. The P2/P3 physical reconciliation cited the pre-V1R-162 mute regression `mute_ipc_tears_down_listening_and_talking_and_unmute_stays_passive`. The current IPC regression is `mute_ipc_uses_boolean_as_single_authority_and_unmute_stays_passive`, which covers Listening and Talking, asserts the authoritative mute boolean, verifies capture/playback teardown and queue/level reset, and proves unmute stays passive.

Those citations are corrected in their original reconciliation documents. The V1R-182 P12 barge-in correction remains intact: `barge_in_flushes_buffered_output_and_interrupts_once` proves the non-empty playback flush/level/play-state/provider-interrupt behavior, while `barge_in_keeps_current_local_asr_active` separately proves Moonshine lifecycle continuity.

## Audit result

No other stale or invented named behavioral-test citation was found in the reconciliation set. The remaining named citations resolve to tests that assert the behavior attributed to them; documents without named test citations continue to rely on their stated source, CI, manual, physical, benchmark, packaging, or release evidence rather than overstating automated coverage.

The legacy feature TODO header already preserves the remediation specification's §7.3 safeguard in substance: its checkbox-sync correction records that completed/tested claims were skeptically re-audited and reverted where their only support was a `#[cfg(test)]` helper re-implementation, explicitly warning that such a test cannot fail when the production path regresses. V1R-201 therefore retains that historical warning rather than rewriting the large feature tracker merely to restate it.

## Execution evidence

The current production/test tree immediately before this documentation-only reconciliation passed GitHub Actions run `33175702200` on commit `813a766f340f4d6047e68db167f105cfd1b4b3ee`, including Rustfmt, Clippy, the full Rust test suite, backend failure and stress matrices, frontend quality/build, dependency/security audits, release metadata, and both supported macOS bundle jobs. That run executes the current replacement tests named above. The V1R-201 edits themselves are documentation-only and are additionally checked for Markdown formatting and whitespace correctness before publication.
