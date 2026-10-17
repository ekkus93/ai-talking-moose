# WSP-400 — Whisper abort discards queued PCM and suppresses stale final

**Recorded:** 2026-10-17 UTC
**Qualified source SHA:** `1ddce089211e6655ef4a44215884f52ffabec218`
**Test:** `whisper_abort_discards_queued_pcm_and_never_delivers_stop_time_final`
**Source:** `src-tauri/src/asr/pipeline_tests/shutdown.rs`

## Test scope

The regression uses the shared `LocalAsrPipeline` with
`LocalAsrArchitecture::WhisperSmall`, the 56-chunk queue capacity and a
deterministic fake inference engine. A gated first PCM push is followed by
two accepted queued chunks. After `request_abort` and release of the
in-flight engine call, the test checks that exactly one PCM chunk reached
inference, the two queued chunks were discarded, the pipeline stopped,
the diagnostic queue depth returned to zero, and the engine stopped once.
The in-flight partial event remains observable, while a fake stop-time
final transcript is suppressed. No physical microphone, weights or
network endpoint participates.

## Exact-source CI

- [Main CI 39042201712](https://github.com/ekkus93/ai-talking-moose/actions/runs/39042201712):
  **success** at source `1ddce089211e6655ef4a44215884f52ffabec218`. Rust quality, complete Rust tests,
  generated backend contract and CI scope classification all passed.
- [ASR-015 39042201721](https://github.com/ekkus93/ai-talking-moose/actions/runs/39042201721):
  **failure** at the same source in the retiring Moonshine native
  preparation step, independent of this Whisper regression.
- [Whisper macOS Native Compile 39042201677](https://github.com/ekkus93/ai-talking-moose/actions/runs/39042201677):
  **in progress** when recorded. Check both architecture jobs before
  asserting Mac qualification.

## Remaining acceptance

This automated fake-engine test does not establish real Whisper inference,
macOS enforced offline egress, physical microphone behavior, packaged
application startup, real TTS playback or full provider-routing acceptance.
WSP-200/300/400/500 and release/physical gates remain open. Moonshine
cutover is not authorized by this test, and KittenTTS plus its separately
owned ONNX Runtime were not changed.
