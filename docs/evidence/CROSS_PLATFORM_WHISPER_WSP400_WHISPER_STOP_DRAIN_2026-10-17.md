# WSP-400 — Whisper graceful-stop queue-drain regression

**Recorded:** 2026-10-17 UTC
**Repository:** `ekkus93/ai-talking-moose`
**Qualified source SHA:** `7553f631b35ed2c7803528d80bda955caf172538`
**Authoritative TODO:** `docs/CROSS_PLATFORM_WHISPER_ASR_TODO_2026-10-15.md`

## Change and automated evidence

- `src-tauri/src/asr/pipeline_tests/shutdown.rs`: `whisper_stop_drains_queued_pcm_in_order_and_delivers_one_final` exercises the production `LocalAsrPipeline::start_architecture` worker with `LocalAsrArchitecture::WhisperSmall`, the 56-chunk Whisper ingress bound and a deterministic fake inference engine.
- The test blocks the first PCM push, queues two additional distinct samples, requests graceful stop, releases the worker, then checks all three chunks were processed in order, the queue drained, the engine stopped once, and one final transcript was emitted between one speech-start and one speech-end.
- [Main CI run 39015810304](https://github.com/ekkus93/ai-talking-moose/actions/runs/39015810304) at the **exact qualified source SHA**: completed **success**. Jobs `Rust tests`, `Rust quality`, `Generated backend contract`, and `Classify CI scope` all concluded **success**. Frontend, package and other skipped jobs are **not** counted as passing evidence.
- [ASR-015 Native Acceptance 39015810276](https://github.com/ekkus93/ai-talking-moose/actions/runs/39015810276) at the same SHA concluded **failure** in the existing `Prepare pinned Moonshine native runtime` step. This is an unresolved legacy Moonshine CI gate, not evidence that the new Whisper worker test failed. The Moonshine gate must be replaced with equivalent real Whisper qualification after WSP-200/300/400 safety prerequisites, not silently skipped.
- [Whisper macOS Native Compile 39015810267](https://github.com/ekkus93/ai-talking-moose/actions/runs/39015810267) was **in progress** at evidence capture; no success claimed for that run here.

## Scope and remaining gates

This is deterministic queue/worker behavior using mock inference. It does **not** establish pinned-weight inference, installer lifecycle, macOS offline egress enforcement, Tauri `.app` packaging, live microphone, physical wake-word handoff, real text/TTS playback, KittenTTS offline synthesis, or full provider routing. All WSP-200, WSP-300, WSP-400, WSP-500, WSP-600 and WSP-700 acceptance boxes that require these remain open. Moonshine runtime, native dependencies and the independently owned KittenTTS ONNX Runtime were not changed or removed.
