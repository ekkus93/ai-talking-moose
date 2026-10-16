# WSP-100: Verified native Whisper ABI and Linux regression evidence

**Compiled source SHA:** `35e23b6be427d7856e98f58773171df4dbf2996d`
**Checked:** 2026-10-16 (UTC), Ralph Bridge CI read plane
**Scope:** Verified ABI/loadability and Linux source-level regression gates; **NOT** real Intel CPU inference, packaged Mac app, or physical microphone.
**Reference:** `docs/CROSS_PLATFORM_WHISPER_ASR_TODO_2026-10-15.md`

## macOS arm64 and Intel

[Whisper macOS Native Compile run 38853657652](https://github.com/ekkus93/ai-talking-moose/actions/runs/38853657652) completed successfully at the exact SHA:

| Job | Architecture | Conclusion | Observed successful checks |
| --- | --- | --- | --- |
| `116820427346` | `aarch64-apple-darwin` | success | Pinned submodule/provenance and host; `clang` C ABI layout probe against pinned `whisper.h`; Cargo static source build/link; native `whisper_version()` invocation; Mach-O and static archive evidence |
| `116820427627` | `x86_64-apple-darwin` | success | Same checks on Intel host |

The native Rust FFI code maintains `repr(C)` structure fields, explicit type/offset/size assertions and library-owned string copying; a real C ABI native call ran on both platforms. This is more than a symbol grep. It does not prove fixed-corpus inference or a downloadable model is usable on Intel.

## Linux

[Main CI run 38853657618](https://github.com/ekkus93/ai-talking-moose/actions/runs/38853657618) succeeded at the same exact SHA:
- `Rust tests` passed the complete Rust test suite.
- `Rust quality` passed provenance validation, `rustfmt` and Clippy.
- Generated-backend-contract, frontend, canonical `check:all`, packaging and release jobs were **skipped**, not qualified by this run.
- This was not a fresh Linux real-model inference run. Previous Linux real CPU evidence remains historical until separately rerun at the final cutover.

## Outstanding WSP-100 gates

- A newer native job validates **all four** static archives (`whisper`, `ggml`, `ggml-cpu`, `ggml-base`) and inspects Mach-O dynamic dependencies on both Macs. Its results must be observed before checking the dedicated library/linkage TODO.
- A missing-runtime **negative release-build** test has not yet been demonstrated; code-level production fail-closed handling alone does not count as full qualification.
- Although the app now explicitly sets `use_gpu=false` and `flash_attn=false`, no *new* real-model fixed-corpus run has yet proved actual CPU-only inference on Intel. The old rerun at SHA `df9411fde91ecbe245de662cc92b2f4a3af3f96f` selected a paravirtual Metal device and aborted. Do not count it as CPU proof.
- No packaged application, notarized artifact, physical microphone or strict macOS network-denial evidence is claimed.

The WSP-200 cutover gate remains open. Keep macOS defaults and Moonshine as-is until the newer CPU-only acceptance workflow transcribes the pinned model on both architectures. The existing Moonshine-specific ONNX failure is not a request to repair Moonshine; preserve independent KittenTTS ONNX dependencies.
