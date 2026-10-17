# WSP-400 — Whisper startup preflight error handoff

**Date:** 2026-10-17  
**Repository:** `ekkus93/ai-talking-moose`  
**Exact source commit:** `993d8e043f25fae90bac1cbf709403e19477ec87`  
**Modified source:** `src-tauri/src/asr/pipeline.rs`, `src-tauri/src/asr/pipeline_tests/readiness.rs`

## Defect and correction

Before this change, the startup worker could send a typed factory error over its readiness channel, then exit returning `Err(error)`. The parent called `join_finished_startup_worker`, which treated that expected worker `Err` as a generic join failure and discarded the originally reported model/native preflight diagnostic. The correction distinguishes a successful thread join carrying a worker error from a thread panic or task-join failure. When readiness explicitly reports a failure, that exact diagnostic is preserved; when the channel closes without a readiness response, an available worker error is recovered rather than fabricated.

New `whisper_startup_preserves_preflight_error_instead_of_reporting_a_join_failure` regression checks `ModelNotInstalled`, `ModelCorrupt`, `RuntimeUnavailable`, and `ModelLoadFailed` with expected message and retryable flag, and asserts that failed startup emits no ASR event or successful audio-ingress handle. Existing preflight, cancellation, lifecycle and no-fallback tests remain intact.

## Exact-source qualification

- Ordinary GitHub Actions CI: [run 39053389008](https://github.com/ekkus93/ai-talking-moose/actions/runs/39053389008), completed **success** on source `993d8e043f25fae90bac1cbf709403e19477ec87`. Relevant jobs: Rust tests, Rust quality, generated backend contract, CI scope classification **passed**. Other full/release jobs skipped by scope classification.
- Wake Word lifecycle stability: [run 39053389015](https://github.com/ekkus93/ai-talking-moose/actions/runs/39053389015), completed **success** at the same source.
- Whisper macOS Native Compile: [run 39053389034](https://github.com/ekkus93/ai-talking-moose/actions/runs/39053389034) was **in progress** when recorded. Its arm64 and Intel job conclusions must be independently checked; this document does not claim that they passed.
- Legacy ASR-015: [run 39053389111](https://github.com/ekkus93/ai-talking-moose/actions/runs/39053389111) failed in `Prepare pinned Moonshine native runtime`. KittenTTS-to-Moonshine intelligibility and voice-audition runs `39053389009` and `39053389001` likewise failed in legacy Moonshine runtime preparation. These do not establish a Whisper preflight regression or an independent KittenTTS synthesis defect.

## Acceptance limits and gate

The regression uses a synthetic failing engine factory. No downloaded `ggml-small.bin`, real microphone, CPU-weight inference, network-denied Mac, packaged app, native TTS playback, System Voices physical audition or release signing was exercised by this test. No user model or credential was committed. Keep the WSP-200/300/400/500/600/700 unfinished gates open. In particular, **do not remove Moonshine** until the app-level and native acceptance gates are met, and retain separately pinned ONNX Runtime for KittenTTS.
