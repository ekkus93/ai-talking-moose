# WWR-510 privacy-safe diagnostics audit — 2026-09-19

Base commit: `a61108401d585c1685f76a47fa7444c9fd3d884e`

## Scope

This audit compares `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md` WWR-510 against the current authoritative implementation in `src-tauri/src/asr/wake_word_diagnostics.rs` and `src-tauri/src/asr/wake_word_runtime.rs`.

## Already implemented

`WakeWordDiagnostics` currently exposes the following privacy-safe fields required by WWR-510:

- enabled state derived from the authoritative runtime phase;
- authoritative runtime phase;
- exact engine/model identity and pinned model archive SHA-256;
- exact sherpa runtime identity/license and platform-specific C-API SHA-256 where supported;
- platform and architecture;
- one-thread inference policy;
- canonical sample rate and channel count;
- ring capacity in samples and milliseconds plus current retained sample count;
- fixed threshold and score;
- trigger count;
- monotonic last-trigger age in milliseconds;
- initialization duration;
- Talking suspension state;
- sanitized last error.

The diagnostics type intentionally has no field capable of carrying raw PCM, transcripts, credentials, or filesystem paths. Unit coverage serializes diagnostics and asserts those classes of data are absent. Runtime errors are represented by the fixed sanitized message `The Wake Word runtime encountered an internal error.` rather than propagating arbitrary underlying error text.

## Remaining WWR-510 work

The current implementation does **not** yet provide optional measured CPU, memory, inference-latency, or handoff-latency fields. Those TODO entries are explicitly optional (`as available`) and should only be added when a trustworthy measurement boundary exists.

A final repository-wide log/error audit is still required before WWR-510 can be marked completely closed. In particular, production KWS artifact/runtime load failures should be checked end-to-end to ensure no unnecessary absolute model/runtime path is surfaced outside internal setup evidence.

## Qualification boundary

This document records implementation evidence only; it does not by itself mark WWR-510 complete. Closure should reconcile the TODO only after the remaining repository-wide error/log audit is performed and exact-head CI is green.
