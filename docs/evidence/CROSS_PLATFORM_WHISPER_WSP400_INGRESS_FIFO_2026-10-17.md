# WSP-400 — Whisper bounded ingress preserves accepted PCM ordering

**Recorded:** 2026-10-17 UTC
**Qualified source SHA:** `d0f7327d4d299459403fa40f1e3ab1324cf21cb7`
**Test:** `whisper_ingress_rejects_newest_chunk_and_drains_older_pcm_in_order`
**Source:** `src-tauri/src/asr/pipeline_tests/queue.rs`

## Deterministic test and assertions

The regression constructs the shared `LocalAsrPipeline` with
`LocalAsrArchitecture::WhisperSmall` and the production 56-chunk bounded
ingress. A fake inference worker is deliberately blocked after receiving
one distinguishable PCM chunk. The test queues 56 additional distinguishable
chunks, verifies the reported depth equals the bound, and checks that
`try_send` rejects a newest overflow chunk with `Full`. After releasing
the worker, graceful stop drains the previously accepted chunks in exact
FIFO order, without substitution by the rejected chunk, and stops the
worker exactly once. PCM is converted through the existing 16-bit-to-f32
conversion. No audio device or model weights are used.

## Exact-source qualification

- First source commit `9146d43b6c9ce583ce4a0cc4910dbfe4a4296598`:
  main CI `39041074901` found a **Rust formatting failure** in the
  new test's length assertion. The failure was corrected; this commit is
  not treated as a passing qualification.
- Corrected source `d0f7327d4d299459403fa40f1e3ab1324cf21cb7`: main CI
  [run 39041187710](https://github.com/ekkus93/ai-talking-moose/actions/runs/39041187710)
  **success**: Rust quality, complete Rust tests, generated backend
  contract, and CI scope classification all succeeded.
- At the same source SHA, legacy ASR-015
  [run 39041187764](https://github.com/ekkus93/ai-talking-moose/actions/runs/39041187764)
  **failed** in `Prepare pinned Moonshine native runtime`; this is not
  a Whisper inference/test failure and is not repaired by repinning the
  retiring Moonshine dependency.
- At the same source SHA, Whisper macOS Native Compile
  [run 39041187752](https://github.com/ekkus93/ai-talking-moose/actions/runs/39041187752)
  was **in progress** when this note was written. Recheck its final
  arm64 and Intel job conclusions before counting Mac qualification.

## Limits and cutover safety

This is deterministic fake-inference worker/queue evidence only. It does
**not** demonstrate physical microphone capture, genuine Whisper model
transcription, offline egress enforcement, end-to-end provider/TTS routing,
a signed or packaged Mac application, or physical acceptance. WSP-200,
WSP-300, WSP-400, WSP-500 and release/physical gates remain open as stated
in the authoritative TODO. Moonshine removal is **not** authorized by
this test; independent KittenTTS/ONNX Runtime dependencies are untouched.
