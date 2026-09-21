# WWR-510 Wake Word privacy/log audit

Date: 2026-09-21
Audited base: `0bb26753d4b3f1acffa9c6f5fe90dd5c6a115040`
Scope: Wake Word runtime/engine diagnostics and the production command-lifecycle boundary.

## Result

The audited Wake Word paths expose bounded state/count/timing data and sanitized errors; they do not serialize or log retained PCM. No Wake Word error path inspected here emits credentials or an unnecessary absolute model/runtime path.

## Evidence

- `src-tauri/src/app/wake_word_engine.rs` constructs public `WakeWordError` values through `WakeWordError::sanitized`. `sanitize_error_message` replaces path-like tokens with `<path>` and long token-like values with `<redacted>`.
- The engine regression `errors_sanitize_paths_and_token_like_secrets` proves both an absolute artifact path and a token-shaped secret are removed from the public error message.
- Artifact/runtime verification errors use fixed messages such as `missing required wake artifact`, `wake artifact identity mismatch`, and `failed to load Wake Word native C API library`; the missing-library regression explicitly proves the temporary absolute path is absent.
- `WakeWordDetection` contains only the canonical keyword and bounded score. The native result boundary reduces sherpa output to keyword presence and does not expose the native JSON/tokens/timestamps fields.
- `src-tauri/src/app/wake_word/runtime.rs` snapshots expose phase, trigger count/age, sanitized last error, ring/capacity sample counts, handoff sample count, and initialization duration. The regression `runtime_snapshot_reports_counts_not_retained_pcm_payloads` proves distinctive retained PCM values do not appear in the snapshot debug representation.
- `src-tauri/src/asr/wake_word_handoff.rs` exposes only queue/capacity/drop counts. Its overflow diagnostics do not expose PCM payloads.
- `src-tauri/src/commands/conversation/core.rs` logs Wake Word lifecycle failures via sanitized Wake Word error values. The inspected Wake-specific lifecycle logging does not log PCM, credentials, model/runtime paths, or the retained handoff buffer.

## WWR-510 reconciliation

This audit supplies objective evidence for these remaining WWR-510 checklist items:

- Audit errors/logs for credentials.
- Audit errors/logs for unnecessary absolute paths.
- Audit errors/logs for audio content.

The optional measured CPU/memory/inference/handoff timing fields remain open and belong with WWR-630 performance evidence. This audit does not claim those measurements exist.

## Limitations

This is a source/privacy audit, not real KWS acceptance. It does not close WWR-600/610/620 or prove platform support. It also does not substitute for the final WWR-900 whole-feature audit after production microphone/handoff integration is complete.
