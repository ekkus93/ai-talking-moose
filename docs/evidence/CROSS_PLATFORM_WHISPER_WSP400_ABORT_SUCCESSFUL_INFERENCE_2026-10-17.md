# WSP-400: Suppress successfully completed inference after explicit abort

**Observed:** 2026-10-17
**Repository:** `ekkus93/ai-talking-moose`
**Qualified source commit:** `b18fa182688a861e5adb94e91a5c4266f69033b3`

## Code and behavior

- `src-tauri/src/asr/pipeline/worker.rs`: the successful `engine.push_pcm` branch now checks `abort_requested` before applying any transcript-state update or scheduling delivery acknowledgements. A native inference returning after cancellation cannot publish a stale partial/final into the retired conversation. Graceful stop (without abort) still processes normal in-flight and stop-time events.
- `src-tauri/src/asr/pipeline_tests/shutdown.rs`: update the existing generic and Whisper-specific deterministic fake-engine queue/abort regressions. Both block the in-flight call, queue more PCM, request abort, release the call and assert no post-abort transcript events; the Whisper test checks one push, one stop, drained shutdown and discarded queued PCM.
- Previous tests intentionally accepted an in-flight partial after abort. This change tightens that contract to meet the spec's stale-generation/privacy requirement. No Moonshine code or native runtime is removed.

## Exact-source qualification

- [CI 39059061656](https://github.com/ekkus93/ai-talking-moose/actions/runs/39059061656) at source `b18fa182688a861e5adb94e91a5c4266f69033b3`: **completed success**. Rust tests (job `117473112759`), Rust quality (job `117473112820`) and generated backend contract (job `117473112828`) all **success**; scope classification **success**. Scope-skipped jobs do not count as complete release qualification.
- [Whisper macOS Native Compile 39059061707](https://github.com/ekkus93/ai-talking-moose/actions/runs/39059061707): arm64 and Intel builds **in progress** when observed; final conclusions not claimed.
- [Legacy ASR-015 39059061616](https://github.com/ekkus93/ai-talking-moose/actions/runs/39059061616): **failure** in Moonshine runtime preparation; expected ONNX Runtime archive size 27,623,992 bytes and observed 35,138,784 bytes. This is a legacy Moonshine dependency preparation failure, not a Whisper regression test failure.

## Acceptance limitations

Tests use synthetic PCM, blocked fake inference and deterministic events, not 487,601,967-byte real weights, live capture, provider playback, an enforced macOS offline environment or physical Apple Silicon/Intel microphone verification. This change does not close the full WSP-200/300/400 lifecycle or authorize WSP-500 Moonshine removal. Preserve independently pinned KittenTTS ONNX Runtime and its offline/no-fallback guarantees.
