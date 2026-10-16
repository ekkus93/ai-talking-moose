# WSP-200 macOS Whisper real-model acceptance — partial evidence

**Observed:** 2026-10-15 (America/Los_Angeles) / 2026-10-16 (UTC)
**Status:** PARTIAL — **the WSP-200 cutover gate is not satisfied**
**Source SHA:** `df9411fde91ecbe245de662cc92b2f4a3af3f96f`
**Workflow:** [Whisper macOS Real CPU Acceptance, run 38846896241](https://github.com/ekkus93/ai-talking-moose/actions/runs/38846896241)
**Model:** `ggml-small.bin`, 487,601,967 bytes, SHA-256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`
**whisper.cpp source:** `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`

## Results verified through Ralph Bridge

- **macOS arm64:** Job `116799364037` passed the real Whisper Small installer, fixed-corpus JFK inference, transcript content assertions ("ask", "country", "you"), and deletion. The native runtime was 1.9.4; transcribe wall-clock = **5,655 ms**, reported peak process RSS = **931,201,024 bytes**. Job uploaded JSON artifact `12023986115` (`whisper-macos-real-arm64-df9411fde91ecbe245de662cc92b2f4a3af3f96f`). Workflow asserts source and model pins, bytes, architecture, and `production_installer_verified=true`. The workflow as a whole failed because the Intel job failed.
- **macOS x86_64, attempt 1:** Job `116799363827` compiled and invoked linked Whisper 1.9.4, then failed at initial HTTPS model download timeout before inference. No Intel transcript was produced.
- **macOS x86_64, attempt 2:** A targeted Ralph Bridge failed-job retry was accepted, launching job `116819455524` on the **same historical SHA**. The full **487,601,967-byte model installed and passed production verification**. The native runtime then selected Metal (`whisper_init_with_params_no_state: use gpu    = 1`; `whisper_backend_init_gpu: using MTL0 backend`) and aborted during decoding at `ggml-metal-device.m:1842` with `GGML_ASSERT(buf_dst) failed`, exit code **134**. The failure is specific to the old Metal-enabled acceptance build/runner, not proof that CPU-only Intel Whisper fails. Real Intel CPU transcription remains unproven.
- **Native compilation:** Independent Whisper macOS Native Compile run `38848996780` at `0314cab3124be9e141c54e6c5f839ad93130c228` passed both ARM64 and Intel static linkage/probe jobs; it cannot substitute for real inference.

## Follow-up fixes (not yet qualified by real-model macOS acceptance)

1. `0314cab3124be9e141c54e6c5f839ad93130c228`: retry connection timeouts before a partial model download begins, with cancellation-aware backoff; preserve full SHA-256 verification. A new manual workflow dispatch is required to prove the Intel network/download case is resolved.
2. `5327ac1c82c1cdf693c77e4ec50bb55b5edbcff5`: remove CI-clock dependence from the ambient provider failure test by disabling quiet hours **in the test fixture only**. The prior Rust-suite failure occurred because the policy could deny the ambient request before the deliberately injected 503.
3. `01b963d4cb1ad456b187b47d22a79f857684a56c`: disable `use_gpu` and `flash_attn` explicitly when constructing the native Whisper context. The historical ARM and Intel attempt-2 logs initialized Metal despite the intended CPU test; the latter actually crashed in that backend. Do **not** claim strict CPU-only acceptance for that older run; repeat fixed-corpus acceptance after this change.

## Migration safety and dependencies

Moonshine remains enabled and selected by default on macOS under the approved two-stage migration gate. Its broken pinned ONNX runtime fallback (ASR-015 run `38848996738`, expected 27,623,992 bytes, got 35,138,784) is **not** to be re-pinned or rebuilt as part of the Whisper migration. Instead, after real macOS Intel Whisper inference passes, complete settings/provider routing, remove Moonshine runtime/build/assets and substitute equivalent Whisper-based native and release acceptance. Independently owned KittenTTS `ort` / ONNX Runtime must survive.

Ralph Bridge still rejects `dispatch_validation(repo="ekkus93/ai-talking-moose", ref="master", profile="whisper-macos-real-cpu")` (and the longer profile candidate) with an opaque outer `INVALID_ARGUMENT`. GitHub Actions manual dispatch worked. This does not prove whether the Ralph config or app authorization is responsible. The next macOS run must target the current `master` exact SHA, *not* rerun the old source SHA and call it a fix verification. The real-model workflow now also rejects a transcript if native diagnostics do not prove `use gpu = 0`, `flash attn = 0`, and no Metal backend selection, and retries only clean initial installer timeouts; these edits have not yet passed a real-model run.

The workflow does not enforce network egress denial, does not exercise an actual microphone, and is not a notarized/packaged application test. These separate gates remain open. Do not remove Moonshine or mark WSP-200 complete until both supported macOS architectures have actually transcribed the pinned model on the newer code.
