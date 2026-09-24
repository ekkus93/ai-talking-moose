# WWR-900 — final source/privacy/security audit

Date: 2026-09-24
Audit status: accepted for WWR-900 source/privacy/security scope
Exact current master: `e7a8dc91f317a62184a8ae7fead90afc1be6343c`

## Scope

This audit records the final Wake Word V1 source/privacy/security review required by WWR-900. It does not close WWR-630 measured performance acceptance, WWR-910 original TODO reconciliation, or WWR-950/960 final exact-head/exact-master closeout.

## Exact validation evidence

The source/security-sensitive Rust paths did not change after the most recent exact source-security validation. Documentation and gate policy were updated afterward, and the exact current master passed the privacy, documentation, and required-gates audits.

Evidence:

- Exact source-security audit for the current Wake source state: Wake Word source security audit run `36036701147`, success, on `8329804e566fe8087959f058a34f2733805750bf`.
- Exact current-master ordinary CI: run `36060690889`, success, on `e7a8dc91f317a62184a8ae7fead90afc1be6343c`.
- Exact current-master Wake Word privacy audit: run `36060690780`, success, on `e7a8dc91f317a62184a8ae7fead90afc1be6343c`.
- Exact current-master Wake Word documentation audit: run `36060690646`, success, on `e7a8dc91f317a62184a8ae7fead90afc1be6343c`.
- Exact current-master Wake Word required-gates audit: run `36060690647`, success, on `e7a8dc91f317a62184a8ae7fead90afc1be6343c`.

## WWR-900 checklist audit

### Single runtime ownership

The source-security audit preserves the single authoritative `WakeWordRuntimeManager` definition and the application-owned `WakeWordApplicationRuntime` composition boundary. The architecture documentation now describes the one-runtime ownership boundary and prohibits duplicate authoritative Wake stacks.

Result: accepted.

### Microphone ownership transitions

The source-security audit preserves shared `AppState::audio_capture` ownership and rejects a competing Wake-specific microphone owner. Wake capture, transfer to command ASR, return to Wake listening, cancellation, disable, and recovery remain serialized around the shared capture owner.

Result: accepted.

### Cancellation and shutdown

The lifecycle/source boundaries include deterministic cancellation, disable, recoverable error, shutdown while Listening, and shutdown during handoff behavior. WWR-640 lifecycle evidence records repeated terminal outcomes and shutdown scenarios.

Result: accepted.

### Ring-buffer clearing

The runtime, lifecycle evidence, and documentation record that retained ring/pre-roll audio is bounded and cleared at Talking, disable, error, shutdown, and invalidated handoff boundaries.

Result: accepted.

### Wake-disabled behavior

Wake defaults disabled; disabling Wake Word leaves manual interaction available and prevents terminal interaction resolution from unintentionally resuming Wake listening.

Result: accepted.

### Talking suspension/resume

The runtime suspends Wake activation before Talking/TTS, clears retained audio on entry, and resumes only through terminal interaction policy when Wake is still enabled. V1 does not implement wake-word barge-in.

Result: accepted.

### One-trigger/one-command invariant

The runtime and router boundaries preserve one accepted wake event per command activation. Repeated positive frames while triggered do not create duplicate trigger counts.

Result: accepted.

### Provider separation and no cloud fallback

Wake Word performs local keyword spotting only. It does not perform full transcription, does not define a second command-ASR provider, and does not silently fall back to cloud/full-ASR for idle wake detection.

Result: accepted.

### Exact artifact/runtime loading

The production KWS path uses pinned model/runtime identities in `wake-word-artifacts.json`. Artifact preparation verifies byte sizes, SHA-256 identities, required files, cache contents, and native architecture before inference and fails closed on mismatch.

Result: accepted.

### Native architecture verification

Linux x86_64 and macOS arm64 runtime architecture checks are represented in the source/security and real-KWS acceptance paths. Wrong architecture cannot reach accepted inference.

Result: accepted.

### Logs, errors, and metrics privacy

The privacy audit passed on exact current master. Wake diagnostics expose privacy-safe state and bounded metadata only; they do not expose raw PCM, transcripts, credentials, secrets, unnecessary absolute paths, or private audio content.

Result: accepted.

### Offline idle KWS inference

The native KWS boundary has no network/cloud dependency in normal inference after verified artifacts are prepared. Real Linux/macOS KWS acceptance executes pinned sherpa inference offline against deterministic generated fixtures.

Result: accepted.

### No full-time ASR for wake detection

Wake Word uses the KWS engine for keyword spotting and does not run the full command ASR path continuously merely to detect the wake phrase.

Result: accepted.

### Documentation truthfulness

The exact current-master documentation audit passed after updating the docs to reflect accepted real-KWS/corpus/lifecycle evidence while still keeping WWR-630, WWR-910, and WWR-950/960 pending.

Result: accepted.

## Boundaries

This WWR-900 audit is source/privacy/security evidence. It is not performance acceptance and does not populate the canonical performance report. It also does not replace the original TODO reconciliation or final exact-head/exact-master closeout.

## Conclusion

No mandatory Wake Word V1 source/privacy/security defect remains unresolved within the WWR-900 audit scope. Remaining work is outside this audit scope: WWR-630 measured performance acceptance, WWR-910 original TODO reconciliation, and WWR-950/960 final qualification and guarded merge verification.
