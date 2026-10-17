# WSP-400 — In-flight Whisper inference error after abort (2026-10-17)

## Exact source and regression

- Source commit: `a86fe71a875435dd4a7ddcf114426161de882005` on `master`.
- Modified: `src-tauri/src/asr/pipeline/worker.rs` and `src-tauri/src/asr/pipeline_tests/shutdown.rs`.
- New deterministic regression: `whisper_abort_during_failed_inference_suppresses_late_error_event`.
- Behavior: a Whisper-architecture fake engine blocks inside `push_pcm`; the owner explicitly aborts; the engine then returns a synthetic inference error. The worker retains the typed error for the joining owner, invokes engine stop once, and emits no late ASR event into the retired generation.
- Implementation checks `abort_requested` before publishing an in-flight inference error. An ordinary, non-aborted inference failure still follows the existing typed error event path; that separate regression remains in `pipeline_tests/audio.rs`.

## Observed CI

- [CI run 39056592023](https://github.com/ekkus93/ai-talking-moose/actions/runs/39056592023), exact source `a86fe71a875435dd4a7ddcf114426161de882005`: **success**. Rust quality job `117465722926` **success**, Rust tests `117465723000` **success**, generated backend contract `117465722974` **success**.
- [Whisper macOS Native Compile run 39056592008](https://github.com/ekkus93/ai-talking-moose/actions/runs/39056592008), same source: Pinned Whisper static link (x86_64): in_progress/pending (job 117465766526); Pinned Whisper static link (arm64): in_progress/pending (job 117465766704). Do not count these as passed until both architecture jobs finish successfully.
- [ASR-015 Native Acceptance run 39056591858](https://github.com/ekkus93/ai-talking-moose/actions/runs/39056591858), same source: **failure** in legacy Moonshine preparation, before native acceptance. Job `117465694608` reported `ONNX Runtime size mismatch: expected 27623992, got 35138784`. This is not evidence of a Whisper inference regression, and does not justify deleting KittenTTS's independently required ONNX Runtime.

## Evidence boundary

The new test uses synthetic PCM, a deterministic fake inference engine and no model weights. It does **not** prove actual Whisper model inference, physical microphone or TTS playback, packaged macOS app, enforced offline egress, signed/notarized release or complete WSP-400/500. Preserve the Moonshine removal safety gates and the independent KittenTTS dependency until full cutover acceptance is qualified.
