# Cross-platform Whisper — exact-source two-Mac real-CPU acceptance and lease cancellation

**Observed UTC:** 2026-10-18. **Evidence source commit:** `b7002e726d9bd768f969a6b2cc47c564edbf9c31`. **Evidence recording base:** `06ddcfdb4ae53d23034205928fa39f035f0f6746`.

## Exact-source qualification

- Manual `Whisper macOS Real CPU Acceptance` [run 39091829528](https://github.com/ekkus93/ai-talking-moose/actions/runs/39091829528): **completed success**; both matrix jobs completed successfully.
- ARM64 job [117570022132](https://github.com/ekkus93/ai-talking-moose/actions/runs/39091829528/job/117570022132), artifact ID `12123570575` (`whisper-macos-real-arm64-b7002e726d9bd768f969a6b2cc47c564edbf9c31`), JSON only.
- Intel x86_64 job [117570022226](https://github.com/ekkus93/ai-talking-moose/actions/runs/39091829528/job/117570022226), artifact ID `12123322566` (`whisper-macos-real-x86_64-b7002e726d9bd768f969a6b2cc47c564edbf9c31`), JSON only.
- Both jobs verified pinned Whisper source `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774` and verified model `ggml-small.bin`, 487,601,967 bytes, SHA-256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b` before genuine JFK corpus transcription. Production installer, deletion, clean reinstall, missing/corrupt model fail-closed checks, native CPU-only logs (`use gpu = 0`, `flash attn = 0`) and JSON report assertions passed.

| Metric | ARM64 Apple M1 virtual runner | Intel i7-8700B |
| --- | ---: | ---: |
| Transcription wall | 3,112 ms | 5,767 ms |
| First partial event | 7,980 ms | 10,885 ms |
| First final event | 17,167 ms | 23,822 ms |
| Pipeline real-time factor | 0.85916084 | 1.680607 |
| Peak resident bytes | 900,104,192 | 842,997,760 |
| Normal cadence dropped chunks | 0 | 0 |
| Synthetic overload dropped chunks | 8 | 8 |
| Observed Metal library initialization, despite CPU selection | ~17.923 s | ~32.669 s |

**Performance concern:** Intel pipeline RTF exceeds 1.0, and both native logs exhibit costly Metal library initialization. These observations require representative sustained tests and careful investigation of backend startup; a CPU-only selected inference path is not proof that no Metal library was initialized elsewhere.

**Offline limitation:** macOS real-CPU workflow sets `network_denied_required=false` and explicitly asserts this in its JSON acceptance contract. It downloads the pinned weight before inference. No enforced macOS egress-denial proof was collected, so **WSP-200 network-denied acceptance remains open**. Failed socket connects alone are not enforcement evidence.

## Separate lifecycle and packaging qualification

- Rust source `e6b6114b79ce74dbe35fceef24bbbc946c273f8d`, formatted `b7002e726d9bd768f969a6b2cc47c564edbf9c31`: `WhisperModelInstaller::install` can now respond to cancellation while waiting for a model lease; fixture regression preserves installed model bytes and marker and prevents another download. [CI 39090486399](https://github.com/ekkus93/ai-talking-moose/actions/runs/39090486399): Rust tests, Rust quality, generated backend contract and scope classification all **success**.
- [Whisper macOS Native Compile 39090486346](https://github.com/ekkus93/ai-talking-moose/actions/runs/39090486346): ARM64 `117566059037` and Intel `117566059238` **success**. Native source, static link, deterministic installer/pipeline, settings and non-Live provider regressions; these are not real microphone tests.
- [Whisper-only unsigned app 39090486347](https://github.com/ekkus93/ai-talking-moose/actions/runs/39090486347): ARM64 `117566042055` and Intel `117566041854` **success**.
- [Pre-cutover unsigned app 39090486363](https://github.com/ekkus93/ai-talking-moose/actions/runs/39090486363): ARM64 `117566038873` and Intel `117566038657` **success**.
- CI-only successor `06ddcfdb4ae53d23034205928fa39f035f0f6746`: [run 39093678168](https://github.com/ekkus93/ai-talking-moose/actions/runs/39093678168) passed CI scope/plumbing only. Rust/front-end/contract/release tests **skipped**, not accepted as a full qualification at this newer SHA.
- Legacy [ASR-015 39090486394](https://github.com/ekkus93/ai-talking-moose/actions/runs/39090486394) **failed** in independent Moonshine dependency preparation (`ONNX Runtime size mismatch: expected 27623992, got 35138784`). This is neither a Whisper inference failure nor a reason to delete KittenTTS's independently pinned ONNX Runtime.

## Acceptance boundary and next work

Still open: enforced macOS offline inference with an actual egress restriction; full live audio/worker/interrupt lifecycle and sustained Intel latency; clean physical Apple Silicon and Intel microphone/TTS/wake/bubble acceptance; production Moonshine removal under WSP-200/300/400 dependency gates; release signing/notarization/physical P13; Linux exact final-head replay. Keep KittenTTS ONNX, Sherpa wake word, Gemini Live isolation and existing privacy/no-fallback invariants intact. Do not mark WSP-500, WSP-600 or WSP-700 complete on these records.
