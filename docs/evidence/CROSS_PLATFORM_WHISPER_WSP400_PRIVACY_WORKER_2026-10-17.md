# WSP-400: Whisper-architecture pipeline privacy regression

**Observed:** 2026-10-17 UTC
**Implementation source SHA:** `c3430a8df2e2f4a0212761eb769b51aa899e2d4a`
**Source:** `src-tauri/src/asr/pipeline_tests/privacy.rs`
**Test:** `whisper_worker_does_not_log_pcm_or_transcripts`

## What was tested

The deterministic test creates a `LocalAsrPipeline` using
`LocalAsrArchitecture::WhisperSmall` and the production Whisper
56-chunk queue capacity. A fake inference engine receives synthetic
16-bit PCM through the real worker boundary and emits a synthetic final
transcript. The test asserts the ordered start/final/end events, one
engine invocation, one stop, and absence of the transcript, UTF-8 PCM
sentinel and base64 PCM sentinel from captured tracing logs.

The prior provider-neutral tracing test remains in place.

## Exact-source CI

- Main CI run `39039088621` at source SHA
  `c3430a8df2e2f4a0212761eb769b51aa899e2d4a`: **success**.
  Rust quality, Rust tests and generated backend contract jobs succeeded.
- Whisper macOS Native Compile run `39039088624` at the same SHA:
  **in_progress when this note was prepared**.
  Check the final run conclusion before claiming macOS qualification.
- Legacy ASR-015 run `39039088652` at the same SHA: **failure**
  in `Prepare pinned Moonshine native runtime`. Its log reports an
  ONNX Runtime archive size mismatch (expected 27,623,992 bytes;
  observed 35,138,784 bytes) after Moonshine LFS was unavailable.
  This is not a Whisper test failure. Do not repair or remove Moonshine
  solely to make this legacy workflow green before cutover gates pass.

## Limitations and safety

This is automated worker/event/logging evidence with **fake inference**.
It does not demonstrate pinned-model transcription, real microphone
capture, enforced macOS egress denial, signed/packaged app behavior,
provider/TTS playback, or physical acceptance. No WSP-400 checklist
item or Moonshine-removal gate is closed by this test alone.
KittenTTS and its independently owned ONNX Runtime remain untouched.
