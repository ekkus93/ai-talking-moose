# WWR-510 Privacy-Safe Wake Diagnostics Audit — 2026-09-21

## Scope

This evidence records the current source-backed audit status for the remaining WWR-510 diagnostic/privacy checklist items:

- audit errors/logs for credentials;
- audit errors/logs for unnecessary absolute paths;
- audit errors/logs for audio content.

It deliberately does **not** claim that optional measured CPU, memory, inference, or handoff timing fields are implemented. That remains open until measured diagnostics exist.

## Exact base

- Repository: `ekkus93/ai-talking-moose`
- Base commit: `07bf2758481a749191b3ac56a2a61d9c1e775526`
- Base CI: push CI `35650183067` passed on exact merged master.

## Source audit summary

### Sanitized Wake Word engine errors

`src-tauri/src/app/wake_word_engine.rs` centralizes Wake Word engine errors through `WakeWordError::sanitized`, which applies `sanitize_error_message` before storing the public diagnostic string.

The existing unit test `errors_sanitize_paths_and_token_like_secrets` verifies that:

- path-like tokens such as `/tmp/private/model.onnx` are replaced with `<path>`;
- long token-like values such as `abcdefghijklmnopqrstuvwxyz123456` are replaced with `<redacted>`;
- the public error message preserves only the bounded operational context.

The source also avoids surfacing native loader paths in common runtime failures. For example, `missing_native_c_api_library_is_sanitized_before_inference` verifies that the public error text is exactly `missing required Wake Word native C API library` and does not contain the temporary runtime path used by the test.

### Runtime-state diagnostics do not expose raw audio

`src-tauri/src/asr/wake_word_runtime.rs` exposes `WakeWordRuntimeSnapshot` as structured diagnostic state. The snapshot contains phase, trigger count, last-trigger age, sanitized last error, ring/pre-roll sample counts, buffer capacity, and initialization duration. It does not include `Vec<i16>`, raw PCM bytes, transcript text, or any serialized sample payload.

The runtime error path stores a fixed static string: `The Wake Word runtime encountered an internal error.` That string is independent of audio payloads, paths, credentials, model filenames, or device details.

### Detection diagnostics are bounded

`WakeWordDetection` contains only the fixed wake phrase and a score. The existing test `detection_event_is_bounded_and_never_contains_audio` verifies the bounded event shape. Detection does not store PCM, user speech, transcripts, token arrays, model paths, or native-library paths.

### Native KWS result handling remains bounded

Native KWS result handling reduces the native result to keyword presence only. `keyword_result_has_detection` reads whether the native result keyword pointer is non-null and non-empty. It does not propagate native result JSON, token arrays, timestamps, or audio samples into Wake Word diagnostics.

The existing test `keyword_result_detection_is_bounded_to_keyword_presence` covers this boundary.

## Current conclusion

The current code provides source-backed evidence that Wake Word diagnostic errors and runtime snapshots avoid exposing credentials, unnecessary absolute paths, and audio content through the implemented Wake Word diagnostic surfaces.

The following WWR-510 item remains open:

- optional measured CPU/memory/inference/handoff timing fields as available.

This audit is source/evidence reconciliation only. It does not assert real-audio acceptance, performance measurements, or production end-to-end Wake Word usability.
