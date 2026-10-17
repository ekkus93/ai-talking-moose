# WSP-400: Abort cleanup cannot publish a late ASR error

**Date:** 2026-10-17
**Repository:** `ekkus93/ai-talking-moose`
**Exact source SHA:** `b7428c32524abf9db3866495738d6133066c467b`

## Implementation and contract

- `src-tauri/src/asr/pipeline/worker.rs`: when an explicit abort is active, a failure from `engine.stop()` remains the worker's returned `AsrError`, but no new `AsrEvent::Error` is scheduled to the retired generation. Graceful non-abort stop retains the previous error delivery policy.
- `src-tauri/src/asr/pipeline_tests.rs`: fake engine exposes an injectable native cleanup error.
- `src-tauri/src/asr/pipeline_tests/shutdown.rs`: `whisper_abort_cleanup_failure_does_not_emit_post_cancel_error` verifies that abort discards queued PCM, calls engine stop once, yields no late ASR events, and propagates the exact synthetic cleanup failure to the joining owner.

These tests use a deterministic fake inference engine and synthetic PCM. They do not instantiate actual Whisper weights, CPAL hardware, text providers, or standalone TTS.

## Exact-source checks

- Initial source `3fb32cf788cda657f0c99c980e802add46f38cf5`: CI `39054861787` failed solely in `Check Rust formatting`. A rustfmt-only corrective commit produced the qualified source SHA above.
- Corrected source: [CI run 39054990358](https://github.com/ekkus93/ai-talking-moose/actions/runs/39054990358), completed **success**: Rust tests, Rust quality, generated backend contract, scope classification all passed. Scope-based skips are not claimed as full release validation.
- [Whisper macOS Native Compile 39054990340](https://github.com/ekkus93/ai-talking-moose/actions/runs/39054990340) was **in progress** when this evidence was composed; both arm64 and x86_64 final conclusions remain subject to separate inspection.
- ASR-015 Native Acceptance `39054990334` failed during `Prepare pinned Moonshine native runtime`. That legacy workflow is independent of this new Whisper cancellation test.

## Unqualified and protected gates

No actual Whisper inference, real microphone recording, network-denied macOS test, speech playback, KittenTTS integration audition, signed/notarized bundle or physical Mac acceptance was performed in this change. Do not close WSP-200/300/400/500/600/700 on this isolated regression. Preserve Moonshine until the full cutover gates and retain independently pinned KittenTTS ONNX Runtime.
