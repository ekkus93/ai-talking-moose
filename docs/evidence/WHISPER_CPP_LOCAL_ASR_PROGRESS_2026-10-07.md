# Whisper.cpp Local ASR Remediation Progress Evidence

**Date:** 2026-10-07
**Status:** Partial evidence only. This is not final closeout evidence.

## Source head

Production source remediation discussed here is present through:

- `f096cd2d932e7e3f500ac70d39a0cae772244c0d`

Documentation-only follow-up commit:

- `91a2d8eb0e3992f5ccad7d9cf8d2488411db893d`

The documentation-only commit did not change production source.

## Exact-head validation observed

For production source head `f096cd2d932e7e3f500ac70d39a0cae772244c0d`:

- Wake Word source security audit run `37670737998`: success.
- Wake Word lifecycle stability run `37670737985`: success.
- Ordinary CI run `37670737989`:
  - Rust quality job `112961777350`: success.
    - Native runtime manifest validation passed.
    - Whisper source/model/license provenance check passed.
    - Whisper build-policy checker passed through the provenance step.
    - Rust formatting passed.
    - Clippy passed.
  - Rust tests job `112961777296`: success.
  - Generated backend contract job `112961777345`: cancelled during Linux dependency installation before the generated-contract check executed.

For documentation-only master head `91a2d8eb0e3992f5ccad7d9cf8d2488411db893d`:

- CI run `37676086867`: success.

## Implemented/proven source areas

The following areas have exact-source Rust quality/test evidence from `f096cd2d932e7e3f500ac70d39a0cae772244c0d`, but checklist closeout still requires final TODO reconciliation and any required final exact-head/exact-master gates:

- Whisper provenance/build-policy checks are wired into the Rust quality provenance step.
- Native Whisper build policy rejects `/proc/nproc` regression and missing native source rerun roots.
- `TALKING_MOOSE_WHISPER_BUILD_JOBS` preserves an explicit positive-integer operator override while retaining portable `available_parallelism()` fallback.
- Whisper preparation regression proves Whisper mode does not require the Moonshine installer and fails only for Whisper/shared reasons in the missing-model or unlinked-runtime fixture.
- Clean-profile installer root creation, canonical per-model layout, and legacy-layout migration tests exist.
- Streaming verifier success and corrupt/truncated/wrong-size/wrong-SHA coverage exists.
- Public Whisper install/verification error mapping coverage exists for integrity, cancellation, invalid manifest, and load/promotion/network/disk classes.
- Pipeline tests cover stop-time final delivery, accepted queue drain, idempotent stop, blank final handling, bounded queue behavior, privacy-sensitive payload logging, and deterministic worker synchronization.

## Open validation gaps

The remediation is not complete because the following are still open:

- Generated backend contract did not execute on the exact production source head because its CI job was cancelled during dependency installation.
- Real-CPU Whisper acceptance has not been successfully dispatched and completed for the final source/master head in this run.
- Real-CPU artifacts have not yet recorded final repository SHA, native source revision, model SHA/bytes, test-audio identity, latency, RTF, CPU, true/high-water RSS, and overload/drop evidence.
- The remediation TODO and original Whisper TODO have not yet been fully reconciled against final exact-source and final exact-master evidence.

## Dispatch/rerun notes

Attempts to rerun the cancelled generated-backend-contract job through Ralph Bridge were rejected without actionable detail. Attempts to dispatch likely real-CPU acceptance profile names were also rejected by the bridge without actionable detail. These tool-level rejections are not source-code evidence and do not close any checklist item.
