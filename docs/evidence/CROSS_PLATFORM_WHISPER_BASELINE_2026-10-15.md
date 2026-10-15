# Cross-Platform Whisper ASR — Baseline and Partial Inventory

**Recorded:** 2026-10-15  
**Read-only baseline before implementation:** `f9427e6be4563841d2b303e045588ae363abb654` (`master`).  
**Status:** WSP-000 partially documented. This is **not** an exhaustive repository scan or a WSP-100/200 acceptance report.  
**Follow-up checklist:** `docs/CROSS_PLATFORM_WHISPER_ASR_TODO_2026-10-15.md`.

## Confirmed code and model identities

- `src-tauri/src/asr/types.rs`: Linux fresh-profile ASR default is `WhisperSmall`; Mac defaults to `MoonshineTinyStreaming`. Existing serialized local variants are `whisper_small`, `moonshine_tiny_streaming` and `moonshine_small_streaming`; `gemini_live_audio` remains an explicit cloud mode.
- Pinned whisper.cpp source commit/gitlink: `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`. The checked-in provenance checker and historical exact-source Linux acceptance verify this pinned identity; newly observed current-commit submodule checkout/proof is pending.
- Model: `whisper-small-ggml`, upstream artifact revision `5359861c739e955e79d9a303bcbc70fb988958b1`, `ggml-small.bin`, exactly `487601967` bytes, SHA-256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`, MIT license. Explicit production installer stores a verified model under `<app-data>/models/whisper/whisper-small/`, with exact-byte/hash checks, cancellation and no implicit downloads.
- `src-tauri/src/asr/pipeline.rs` already has `LocalAsrPipeline::start_whisper`. Whisper uses a 56-chunk (5.6 seconds of 100 ms PCM) bounded queue; partial updates use an ~5-second batch cadence, 500 ms endpoint silence and 30-second utterance cap. Moonshine's 8-chunk queue is separate until cutover.
- `src-tauri/build.rs` had a Linux-only native Whisper source build and macOS Moonshine native linking. `src-tauri/tauri.conf.json` contains two Moonshine-specific macOS dylibs; `scripts/prepare_moonshine_macos.sh` stages them through source/LFS and checksum validation.

## Historical acceptance versus currently observed status

- Historical Linux x86_64 real CPU: `docs/evidence/WHISPER_CPP_LOCAL_ASR_POST_QUALIFICATION_2026-10-08.md`, exact acceptance commit `cf3280ea1d4e91d298828e4935e3369f8cbd6be6`, GitHub Actions `37756211710` (**historical success**), AMD EPYC 7763 four available workers. Eleven-second JFK WAV transcribed with two partials, one final, no normal-cadence queue drops; first partial 8279 ms, final 17099 ms, process RSS peak 707932160 bytes; an isolated no-network inference was recorded. This does **not** qualify new Mac binaries.
- Earlier Moonshine-backed Mac native smoke remained blocked by its upstream ONNX Runtime LFS/binary requirement (not a Whisper inference failure). A directly inspected GitHub Actions failure is `38815057443` at prior `cf7e7b4`; architecture-specific release qualification is still outstanding.
- Immediately preceding migration `master` `f9427e6` CI run `38818693961` **passed the documentation fast path only**. It did not exercise macOS linking, full Rust suites, or actual Whisper transcription.
- First in-progress source change `0906dc8` began target-specific Mac static-build support. `0c79703` added a runtime FFI probe and Mac native compile matrix. Its early Rust quality gate identified a module-order rustfmt issue (run `38820545051`) corrected in the next commit. Neither a cancelled nor pending run is accepted as a Mac qualification.
- At this baseline checkpoint there is no evidence yet of genuine `ggml-small.bin` Whisper transcription on macOS arm64 or Intel, no enforced offline Mac network boundary, and no clean physical microphone acceptance.

## High-priority active-file inventory and disposition

The following paths are **known**, not a claim that every tracked file was searched. Remaining WSP-000 inventory and all cleanup are still open.

| Ownership | Known paths / references | Plan |
| --- | --- | --- |
| Shared Whisper implementation | `src-tauri/src/asr/whisper/`, `src-tauri/src/asr/pipeline.rs`, `src-tauri/src/asr/types.rs`, `third_party/whisper.cpp`, `src-tauri/build.rs` | Port native build and prove real Mac inference, then simplify ASR mode routing |
| Moonshine-owned ASR | `src-tauri/src/asr/moonshine/`, `src-tauri/native/moonshine-runtime.json`, `scripts/prepare_moonshine_macos.sh`, `native/macos/libmoonshine.dylib` | Keep until WSP-200 hard gate; then delete code/native/runtime dependencies |
| Moonshine-specific ONNX runtime | `src-tauri/tauri.conf.json` framework `libonnxruntime.1.23.2.dylib` and native provenance | Remove only when no other shipping feature depends on this **specific** dylib |
| KittenTTS-owned ONNX | `src-tauri/src/ai/local_tts/`, `ort` crate and dedicated verified runtime assets | **Retain**; qualify offline TTS across removal |
| macOS release surface | `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `scripts/verify_macos_bundle.sh`, `scripts/verify_macos_release.sh`, `docs/MACOS_RELEASE.md`, P13 acceptance | Replace Moonshine checks with actual pinned Whisper native, source and artifact evidence |
| Existing Linux Whisper acceptance | `.github/workflows/whisper-real-cpu-acceptance.yml`, `docs/WHISPER_CPP_CPU_BENCHMARK.md` | Preserve and re-run at target post-change SHA |
| Historical docs | Original Linux Whisper and Moonshine benchmarking/acceptance records | Retain with clear historical status; avoid counting as current Mac acceptance |

The full repository search, generated-contract/UI fixture disposition inventory, baseline comprehensive test run, and remaining WSP-000 gate remain **unchecked**. Mac release minimum stays at 13.4 until the complete dependency surface is requalified.

## Work-order protection

Moonshine remains available and the saved/fresh macOS ASR selection is unchanged during WSP-100/200. Only a real verified native model transcription on **both** Mac architectures permits the subsequent WSP-300 switch and WSP-500 removal. Gemini Live and KittenTTS remain independent.

## Tracked-reference inventory (2026-10-15, exact-head automated evidence)

The first full **tracked-file** search ran successfully at `e4354b3937ecf3cccdc96b1b60d5023fc3798f94`: workflow `Whisper Migration Reference Inventory`, run `38822092330`, job `116720826717` (success). It scanned 868 tracked paths, found 222 files with migration-related references, and skipped one nonregular/binary path. The JSON per-file/per-line reference and proposed-owner report is artifact `12012788130` (`whisper-reference-inventory-e4354b3937ecf3cccdc96b1b60d5023fc3798f94`).

Provisional owner groups (file counts): app-native 38; ASR backend 12; CI workflows 18; documentation 69; frontend 14; generated contract 1; historical evidence 25; KittenTTS 1; Moonshine runtime 16; native assets 2; other 10; release tooling 16. These sum to 222 matched tracked files. The script `scripts/audit_whisper_migration_references.py` records the exact matching line numbers and proposed disposition, and can be rerun after code changes.

**Not yet accepted as a completed WSP-000 disposition audit:** classifications are heuristic, 10 files are currently in `other`, the remaining groups require owner-by-owner review, and pinned submodule contents are outside the main repository's tracked-file scan. This is inventory evidence, not native build/transcription, no-network, Mac packaging or physical microphone acceptance.

## First macOS static-build failure and correction

The previous Mac native compile workflow `38821017184` on `a46005cd3166e4f772900907e57251030d0e7226` built all four pinned Whisper static archives on arm64, then failed during the unrelated Tauri app resource check: `Library not found: native/macos/libmoonshine.dylib`. This is not proof of a Whisper FFI failure or of a usable app. Commit `6cc3fe302baad4d8d5a579c7bffd65770542131c` isolates the standalone Whisper CLI compile from Moonshine-only app framework resources via a scoped `TAURI_CONFIG` override. The actual shipping Tauri configuration and Moonshine runtime remain unchanged pending WSP-200. Both-architecture rerun `38821907488` must finish before WSP-100 can be evaluated.

An opt-in, real-model Mac arm64/Intel transcription workflow was added in `d7736ca681044731181c669d2fb08a98814b4041`. Its existence is **not** WSP-200 acceptance; no successful pinned-weight Mac transcript, egress-denied proof, or human mic test has yet been recorded.

## macOS native static-link acceptance — 2026-10-15

Run `38821907488` at `6cc3fe302baad4d8d5a579c7bffd65770542131c` completed successfully on macOS arm64 (job `116720489506`) and Intel x86_64 (job `116720489783`). Both jobs compiled the pinned whisper.cpp static archives and the real `whisper_acceptance` CLI, invoked the native FFI version function (returned `1.9.4`), and verified their Mach-O architecture. Metal, Accelerate and BLAS were disabled for the CPU baseline; `TALKING_MOOSE_REQUIRE_WHISPER_NATIVE=1` enforced a hard build failure on missing Whisper. The standalone CLI bypassed Moonshine-only Tauri bundle framework resources without modifying production packaging.

This is **compile/link and native invocation evidence only**, not pinned-model loading, real transcription, offline egress denial, physical microphone testing, full ABI exercise, or signed app acceptance. WSP-200 is still unqualified, so the Mac default and Moonshine runtime remain unchanged. A final exact-head rerun is required after later changes.

## Residual owner review completed — 2026-10-15

The tracked reference inventory initially left ten matches in the generic `other` bucket. Commit `aab612fc825153d0f2fe0ab34088c028646268a4` assigned each to an explicit owner and reran the exact-head inventory in workflow run `38823244110` (success, both matching and skipped-path evidence logged). Scope remains a reference census, **not** approval to delete artifacts or proof of a complete platform release.

- `.gitignore`: repository hygiene; ignore architecture-isolated `build/whisper-*/` native output as of `7897bbd`; retain app-generated-output exclusions.
- `AGENTS.md`, `CLAUDE.md`: developer guidance; replace active Moonshine-only descriptions after WSP-200, not during current dual-engine qualification.
- `memory.md`: historically dated Whisper integration handoff with stale source/runtime details; treat as archival, never as authoritative current build instructions.
- `tests/test_freeze_wake_word_runtime_identity.py`: separate sherpa-onnx wake-word runtime integrity tests, **retain**.
- `wake-word-artifacts.json`: separately pinned sherpa-onnx wake-word assets and ONNX dependencies, **retain** independently of Moonshine.
- `tools/asr-miri/README.md`, `tools/asr-miri/build.rs`, `tools/asr-miri/src/asr/mod.rs`, `tools/asr-miri/src/asr/moonshine/mod.rs`: Moonshine-specific memory/FFI safety harness; reconcile after WSP-200/300/400, potentially replace with Whisper-appropriate safety coverage. Do not delete early.
- `third_party/whisper.cpp`: one skipped gitlink, deliberately represented by pinned source SHA `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`; native Mac source CI checks the exact source with `scripts/check_whisper_provenance.py` on both architectures.

All tracked reference matches have an assigned functional owner category and a proposed retained/replaced/removed disposition. Per-file manual risk review and any actual deletion remain future WSP-500 tasks; this closes **inventory**, not cutover.
