# WSP-100 release-profile qualification and WSP-300/400 regression checkpoint

**Date:** 2026-10-16 UTC  
**Repository:** `ekkus93/ai-talking-moose`  
**Commit workflow:** direct expected-head commits to `master` via Ralph Bridge exclusively.

## WSP-100 — two-architecture native release-profile gate

Source commit: `14382087c8ca1f43d775531071a4b79dc5cf089d`.

[Whisper macOS Native Compile run 38930368117](https://github.com/ekkus93/ai-talking-moose/actions/runs/38930368117) completed **successfully** on both `macos-15` Apple Silicon (`arm64`) and `macos-15-intel` (`x86_64`). Both native-link jobs reported **success** for:

- Existing pinned submodule/source provenance, C-header/FFI ABI compile probe and native static archive/Mach-O architecture checks.
- Existing debug runtime probe and required-native missing-archive negative check.
- New **Cargo production release-profile** `--locked --release --features whisper-acceptance --bin whisper_acceptance` build, invocation of linked native `probe-native`, correct architecture, no dynamically linked developer/Homebrew Whisper/ggml and no packaged `ggml-small.bin`.
- New negative release-profile build with a specified developer `TALKING_MOOSE_WHISPER_LIB_DIR`: aborts for the intended policy error.
- New negative release-profile build with the pinned `third_party/whisper.cpp/CMakeLists.txt` temporarily absent: aborts with the intended missing-source build error. Source file is restored via shell trap.

This is actual release-profile native compilation/linkage, not a release-distributable `.app`, notarization, genuine model transcription, strict macOS network-denial test or physical microphone audition. WSP-100 release-profile checkbox is complete within that limited scope; all packaging and real-device gates remain open.

## WSP-300/400 — direct-to-master source and regression sequence

The source series following baseline `ef80c3c5c5c6815af9269f7d15e6f65348a4b697` implements:

- `67a36c601d58e8953df3fdcdeaa4be6c5ddada54` and formatting `a81a45d18c27a6766badf653ffa68ac8ff14ab9c`: Whisper cancellation while local preparation is blocked, active pipeline retained on barge-in, and explicit cloud wake-audio handoff rejected before microphone or provider connection.
- `10d595ef238f22376758a8b03b05a8982747f9d8`: Whisper queue uses the 56-chunk bound and preserves wake PCM before newly captured PCM; a full queue refuses wake handoff instead of overwriting previously queued PCM. Shared-ASR documentation correction.
- `86829ba80d651ebbf82e91919aa9c3467ef10c7c`: explicit saved Whisper/Gemini Live settings survive two SQLite app restarts with text/TTS/voice/privacy preferences unchanged and no model auto-install; invalid persisted ASR choice fails without overwriting SQLite.
- `eff16de48b80b11ae39eb7f7b770b1ae3af07566`: model mutation/start lock acceptance for active/connecting Whisper modes and retained retired legacy tests.
- `9961a678b3e608a57284c4c2fcb3b76916549eca`: model-neutral recorded text session confirms Whisper FinalTranscript sends one trimmed text turn; partial/empty/stale updates and local raw microphone PCM do not go to the selected text provider.
- `cd3f84bc8eae08f434d10cd2db81ebff4d4de25f`: cloud audio is rejected immediately for local model install/delete, with no Moonshine-specific incorrect error.
- `bfbfa4d3b8b10155f68e93c1c8e3ab29865e5011`: apply exact Rust formatting corrections and qualify combined source.

At exact head `bfbfa4d3b8b10155f68e93c1c8e3ab29865e5011`:
- [CI 38933055120](https://github.com/ekkus93/ai-talking-moose/actions/runs/38933055120): Rust tests, Rust quality (rustfmt/Clippy) and generated-backend-contract jobs **passed**.
- [Wake Word security audit 38933054996](https://github.com/ekkus93/ai-talking-moose/actions/runs/38933054996): **passed**.
- [Wake Word lifecycle stability 38933055145](https://github.com/ekkus93/ai-talking-moose/actions/runs/38933055145): **passed**.

Cancelled/superseded intermediate runs are not counted as passed. Existing separate Moonshine ASR-015 native dependency failure is not Whisper failure; runtime is intentionally retained pending WSP-500 gates, as is KittenTTS's independent ONNX Runtime.

## Still missing

- WSP-200 complete macOS installer/lifecycle/performance matrix, enforced offline egress-denial evidence, and app packaging with both targets.
- WSP-300 Mac installed/fresh/migrated profile round-trip and actual microphone/model-state tests.
- WSP-400 integrated installed-Whisper end-to-end text/TTS provider routing, microphone/wake audio tests and physical playback.
- WSP-500 coordinated Moonshine retirement only **after** preceding gates; preserve independent KittenTTS ONNX, Gemini Live explicit opt-in, and archived historical acceptance.
- WSP-600 release/package gates and WSP-700 clean-Mac, Intel-Mac and Linux real-hardware acceptance.

Do **not** interpret the Rust fake-session tests or native CLI as packaged macOS/TTS or real-mic evidence. All WSP-400 checkbox-level hardware and full-app requirements remain unchecked.
