# WSP-200 Whisper installer deletion guard — exact-head evidence

**Observed UTC:** 2026-10-18T11:01:51.386Z
**Source commit:** `c831a2f68e6bb83a96bb7e8bcd3c2dd8fee796bc`
**Scope:** Whisper Small installer safety and regression qualification. **WSP-200 remains open.**

## Corrective source change

The `src-tauri/src/asr/whisper/installer/delete.rs` helper now inspects the model path using `symlink_metadata`, rejects an unexpected directory with a sanitized I/O error, and removes only a file or symlink with `remove_file`. It no longer recursively deletes arbitrary contents found where the model weight file is expected. `WhisperModelInstaller::delete` checks the marker path before deleting the model, so the unexpected-directory error preserves the marker.

Two fixture regressions in `src-tauri/src/asr/whisper/installer/tests.rs` assert that (1) an unexpected model directory and its unrelated child remain intact, with the marker preserved, and (2) deleting a model symlink does not delete its external target. These are deterministic filesystem tests, not real-model or physical-device tests.

## Exact-head automated qualification

All runs below were checked against `c831a2f68e6bb83a96bb7e8bcd3c2dd8fee796bc` on 2026-10-18:

| Workflow | Run | Result | Scope |
| --- | --- | --- | --- |
| CI | 39118206722 | success | Rust tests, Rust quality and generated backend contract |
| Whisper macOS Native Compile | 39118206690 | success | ARM64 and Intel pinned native static-link qualification |
| Whisper-only macOS App Smoke | 39118206842 | success | ARM64 and Intel unsigned app smoke |
| Whisper macOS Pre-Cutover App Smoke | 39118206697 | success | ARM64 and Intel unsigned app smoke |
| Legacy ASR-015 Native Acceptance | 39118206708 | failure | Moonshine native dependency preparation, not the Whisper deletion guard |

These CI results qualify the committed fixture correction, but **do not** establish physical microphone acceptance, signed/notarized release, or enforced offline inference.

## Real-model evidence and unresolved qualification

Earlier manual macOS real-CPU run `39091829528` passed genuine pinned-model JFK corpus transcription and production install/reinstall on both ARM64 and Intel at `b7002e726d9bd768f969a6b2cc47c564edbf9c31`. Its recorded Intel pipeline real-time factor was 1.680607, and the workflow did not enforce network denial (`network_denied_required=false`). This is historical exact-source evidence, **not** a current-head real-weight replay.

A new Ralph Bridge `dispatch_validation` attempt for profile `whisper-macos-real-cpu`, requested at `c831a2f68e6bb83a96bb7e8bcd3c2dd8fee796bc`, returned `INVALID_ARGUMENT` with generic `Error executing tool dispatch_validation`; no workflow run or operation ID was returned. This is an unclassified Bridge dispatch rejection, **not** a platform safety denial or an inference failure. No alternative profile was hammered or push-triggered heavy-model workaround introduced.

## Remaining gates

Keep WSP-200 installer/lifecycle, enforced macOS egress isolation, sustained Intel performance, real app/microphone and physical TTS acceptance open. WSP-300/400 routing and lifecycle remain partly qualified; WSP-500 Moonshine removal must wait for its prerequisite acceptance. Preserve independent KittenTTS ONNX Runtime, wake-word runtime, and explicit Gemini Live separation. Do not infer user hardware results from CI.
