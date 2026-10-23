# Cross-Platform Whisper.cpp Local ASR — Specification

> **Owner override, 2026-10-22:** The owner explicitly instructed removal of Moonshine before the remaining WSP-200/300/400 acceptance gates. Removal proceeds under that instruction; outstanding performance, physical-device, and release acceptance are still open. Earlier gating language below records the original plan.

**Date:** 2026-10-15
**Status:** Whisper implementation and owner-authorized Moonshine removal landed; performance, physical-device and release acceptance remain open. See the current checklist and `docs/evidence/CROSS_PLATFORM_WHISPER_CLOSEOUT_2026-10-23.md`.
**Repository:** `ekkus93/ai-talking-moose`
**Authoritative implementation checklist:** `docs/CROSS_PLATFORM_WHISPER_ASR_TODO_2026-10-15.md`
**Related completed Linux work:** `docs/WHISPER_CPP_LOCAL_ASR_SPEC.md`, `docs/WHISPER_CPP_LOCAL_ASR_TODO.md`
**Related active acceptance:** `docs/EXTERNAL_LLM_SYSTEM_VOICES_TODO_2026-10-13.md`

## 1. Decision and motivation

Make the **existing app-owned whisper.cpp / Whisper Small implementation** the sole local speech-recognition engine on supported Linux and macOS targets. Remove Moonshine Tiny and Small, their models, FFI/runtime, installers, configuration choices, native libraries, release integration, and obsolete tests **only after** the Whisper macOS path has compiled and demonstrated genuine transcription and app packaging on both macOS architectures.

The current Moonshine-based macOS packaging relies on pinned upstream Moonshine Git LFS artifacts. Unavailable LFS bandwidth has blocked native release smoke. We choose to remove this extra ASR implementation rather than replace or repin its ONNX binary merely to preserve Moonshine. The scope is an ASR architecture change, **not** removal of ONNX Runtime from the whole project: Local KittenTTS uses its own, independently pinned ONNX Runtime installation. Do not break KittenTTS or its offline/no-fallback guarantees.

This migration is deliberately **not complete** when documentation lands. Completion requires the executable, tests, CI, native artifacts, data migration, and acceptance evidence below.

## 2. Current repository baseline at decision

Observed on `master` `2582159ee36dd8690e7443822d0da4be4e911d9a`:

- `src-tauri/src/asr/types.rs` defaults `AsrMode::WhisperSmall` on Linux and `MoonshineTinyStreaming` on macOS. The serialized ASR values include `whisper_small`, `moonshine_tiny_streaming`, `moonshine_small_streaming`, and `gemini_live_audio`.
- `src-tauri/src/asr/whisper/` contains the pinned whisper.cpp model manifest, safe installer, FFI, engine, and acceptance support. `src-tauri/src/asr/pipeline.rs` already dispatches Whisper through `LocalAsrPipeline::start_whisper` alongside Moonshine engines; microphone capture, PCM conversion, transcript events, bounded queues, worker lifetime, and conversation integration are shared.
- `src-tauri/build.rs` builds whisper.cpp from `third_party/whisper.cpp` via CMake only when the target is Linux (`target_is_linux`). Non-Linux builds lack `whisper_native_linked` and Whisper fails closed. There is an explicit development-only `TALKING_MOOSE_WHISPER_LIB_DIR` override.
- `src-tauri/src/asr/whisper/manifest.rs` pins whisper.cpp source commit `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774` and `ggml-small.bin` (487,601,967 bytes; SHA-256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`). The model is an explicit, verified user-data download, not a bundled asset.
- `src-tauri/tauri.conf.json` bundles `native/macos/libmoonshine.dylib` and `native/macos/libonnxruntime.1.23.2.dylib`. `scripts/prepare_moonshine_macos.sh` builds/stages/signs those libraries and native notices. `src-tauri/native/moonshine-runtime.json` pins their provenance and a macOS 13.4 runtime floor.
- `.github/workflows/whisper-real-cpu-acceptance.yml` is Linux-only and proves the pinned Whisper installer and real offline CPU transcription. `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `docs/MACOS_RELEASE.md` and native smoke/release verifiers contain Moonshine-specific work.
- Whisper batch updates currently use a nominal five-second partial-inference cadence, ~500 ms silence endpoint and 30-second maximum utterance; the Whisper queue accepts up to 56 100 ms chunks to tolerate synchronous CPU inference. Preserve truthful semantics; do not relabel them as streaming or promise equal latency to Moonshine.

Treat this inventory as a **starting point, not an exhaustive file listing**. Before changing code, search all tracked source, tests, generated contracts, documentation, scripts, workflows, native resources, manifests, and submodules for Moonshine and ASR-mode references. Record results and determine ownership for each occurrence. Do not remove unrelated ONNX usage.

## 3. Scope and non-goals

**In scope:** one local Whisper Small engine; Linux x86_64, macOS Apple Silicon (`aarch64-apple-darwin`), and macOS Intel (`x86_64-apple-darwin`); source-built, pinned whisper.cpp native integration; existing verified model installer; backward-compatible settings migration; real ASR and end-to-end app acceptance; Moonshine removal; native packaging, release, licensing, documentation and CI cleanup.

**Out of scope:** replacing Gemini Live audio or its explicit opt-in; altering independent text-generation providers (Local, Google, OpenAI, Ollama, llama-server); changing KittenTTS, Google standalone TTS or macOS System Voices; downloading weights as part of normal startup/CI; adding a remotely hosted Whisper service; introducing automatic cloud/model fallback; adding Linux system TTS; claiming Linux aarch64 or earlier macOS support without separate qualification.

## 4. Target architecture and contracts

### 4.1 One local ASR implementation

`AudioCapture / CPAL -> 16 kHz mono PCM -> bounded LocalAsrPipeline -> WhisperEngine -> TranscriptStateMachine -> AsrEvent -> existing ConversationManager / Talk routing`.

- Keep one authoritative microphone, bounded queue, drop-newest overload behavior, dedicated inference thread, capture-before-worker shutdown ordering, model lifetime lease, cancellation/abort ownership, diagnostics, transcript privacy gates, and generation-scoped event delivery.
- `WhisperEngine` remains the sole local `PipelineEngine` used by production. Remove the Moonshine-specific enum branches and engine factories only at cutover; simplify misnamed Moonshine-only helpers after checking tests and wake-word command handoff.
- Maintain exact final-transcript identity, no duplicate partial-to-final emission, silence-endpoint behavior, pre-roll ordering for wake-word handoff, barge-in, stale-generation suppression, and no forwarding microphone PCM to text or TTS providers. Gemini Live is still an explicitly selected, separate Google audio session.
- Inference is **batch with periodic partial refresh**, not true streaming. Keep UI/diagnostics accurate. Any cadence/endpoint/queue tuning must come from recorded representative benchmarks and maintain resource bounds.

### 4.2 Cross-platform native build

- Generalize `src-tauri/build.rs` so the **pinned** `third_party/whisper.cpp` source is built and statically linked for supported Linux and macOS targets. Avoid separate bundled libwhisper dylibs or external Homebrew libraries in the shipped app.
- Use CMake/Apple toolchain appropriate to the runner and explicit architecture/deployment target. Verify all required static libs (`whisper`, `ggml`, `ggml-cpu`, `ggml-base`) and platform frameworks/link arguments; preserve accurate `whisper_native_linked` cfg and rerun rules. Keep FFI ABI/struct layout aligned with the pinned upstream revision on each target.
- **Production missing-runtime conditions must fail a real release/build gate**, not silently yield a distributable app whose Whisper selector returns RuntimeUnavailable. A fail-closed runtime is appropriate only for intentionally unsupported/development configurations. Do not make the source build a best-effort warning for required release targets.
- CPU inference on both Mac architectures is mandatory. Apple Metal acceleration is optional, separately gated, and must not be needed to establish correctness or cause a second ASR provider path; if included, document resource and signing requirements and retain deterministic CPU fallback within the **same** Whisper engine only when explicitly qualified. No fallback to cloud/Moonshine.
- Verify binary architecture, Mach-O deployment target, codesigning/notarization and absence of developer-machine absolute library paths. Keep `third_party/whisper.cpp` source revision, model hash/bytes and all upstream license evidence pinned.

### 4.3 Model lifecycle

- One Whisper Small catalog entry on all supported targets. Keep `whisper-small-ggml` model identity, explicit Download & Verify, on-disk location under `<app-data>/models/whisper/whisper-small/`, immutable revision, byte-length and SHA-256 verification, safe staging and atomic promotion, disk-space checks, cancellation, install/delete interlocks, verified model leases, and honest error categories.
- No weight is checked into Git, committed to app resources, or downloaded implicitly at build, app launch, settings migration, or default selection. Missing/corrupt/incompatible model must report an actionable local error with **no Google/Gemini/Moonshine/Fake fallback**.
- Existing Whisper installations must not be invalidated, silently redownloaded, or relocated without evidence and an idempotent migration. Existing Moonshine model data must not be silently reused or reinterpreted as Whisper weights.

### 4.4 Settings and UI migration

- Retain the stable serialized `whisper_small` name. Preserve explicitly saved `gemini_live_audio` choices and all independent text, TTS, voice, API key, and privacy settings.
- Change the macOS **fresh-profile default** to Whisper Small. For a persisted Moonshine Tiny/Small selection, explicitly and deterministically migrate the saved ASR selection to `whisper_small` without pretending its model is installed. Document whether an old selection needs an installer prompt; no automatic download, cloud substitution, surprise Google connection or silent settings reset.
- Maintain unknown/additive settings fields and migration idempotence. Decide and test whether settings schema version must change; reject invalid/future versions as before. A save/restart must not resurrect a removed Moonshine value. Do not alter unrelated existing user-selected provider choices.
- Remove Moonshine Tiny/Small settings cards, installer actions, progress messages, device diagnostics, model selection, default assumptions, and obsolete descriptions only after the new Whisper path is operational. The Control Panel must state Whisper is local, batch-partial, requires an explicit model installation, and is the default on Linux and macOS. Gemini Live stays clearly cloud.
- Update Rust/TypeScript IPC and generated frontend contract together (`src/generated/backendContract.json`). Audit versioned fields, switches, match arms, test fixtures, persisted values, request snapshots, and frontend bridge stubs for obsolete ASR variants.

### 4.5 Packaging and dependency separation

- At the final cutover, eliminate Moonshine source/FFI/installer, native dylibs, Moonshine runtime manifest, preparation/provenance scripts, native smoke assumptions, and Moonshine-only license notices, workflows and documentation references, except clearly labeled historical evidence.
- Remove the Moonshine-specific `bundle.macOS.frameworks` list from `src-tauri/tauri.conf.json` **only after verifying** there is no other legitimate use for its entries. Do **not** delete the Local KittenTTS `ort` crate, its separately verified ONNX Runtime artifacts/packaging policy, model manifests, licenses, or tests.
- Re-evaluate macOS minimum deployment version from the complete remaining dependency/runtime surface; do not automatically lower 13.4 simply because Moonshine disappears.
- Rework `scripts/verify_macos_bundle.sh`, `scripts/verify_macos_release.sh`, release manifest/notices, signed builds, CI source and native-provenance checks, both Mac architectures and physical `P13` acceptance. Remove stale flags/paths and prohibit missing or extra Moonshine dylibs in newly built app bundles.

## 5. Functional, security and performance acceptance

### Behavioral invariants

1. Linux and both supported macOS architectures start local Whisper from verified user data, produce meaningful partial/final transcripts from fixed corpus and a real microphone, and route final text to **only** the chosen text provider.
2. Startup fails before microphone capture for missing/corrupt/unusable Whisper; no fallback to Gemini Live, cloud ASR, fake or alternate ASR. The same holds for native library/ABI failure.
3. Existing privacy semantics are unchanged: local PCM remains on device, local text stays local unless a different text provider is explicitly chosen, diagnostics/logs contain no raw transcripts or secrets, and transcript persistence honors saved consent.
4. Barge-in and stop release capture and inference ownership; no orphan worker, zombie audio, duplicate final, stale bubble, or post-cancel text/TTS generation. Wake-word command handoff remains ordered and tested.
5. Model installation/deletion is safe during active sessions; macOS and Linux expose consistent catalog, state, install progress, cancellation and truthful readiness.

### Benchmark and regression policy

Capture an exact commit, target/CPU, OS and architecture, source/model hashes, CPU threads, audio duration, first partial/final latency, real-time factor, queue drops, RSS/peak memory, overload/recovery, and transcript accuracy (reference corpus/WER or deterministic content assertions). Compare to the existing Linux Whisper baseline and any recorded Moonshine/macOS measurements **as evidence**, not as a promise of parity. Run on both Apple Silicon and Intel with CPU enabled; add Metal only after baseline and explicit review. Severe latency, repeated queue drops, or UI stalls require correction or an explicit owner decision before release.

### CI and hardware matrix

| Gate | Linux x86_64 | macOS arm64 | macOS x86_64 |
| --- | --- | --- | --- |
| Pinned source/provenance, build, lint, tests, contract | Required | Required | Required |
| Native Whisper link/load + actual fixed-corpus transcription | Required | Required | Required |
| Explicit model installation and verified offline transcription | Existing real-CPU workflow; rerun | Required macOS-qualified path | Required macOS-qualified path |
| Tauri .app build and dependency/provenance smoke | Not applicable | Required | Required |
| Signed/notarized draft release and clean machine P13 | Not applicable | Before public release | Before public release |
| Physical mic, wake handoff, spoken-response/bubble, device selection | Where available; report limits | Required human hardware evidence | Required human hardware evidence or explicitly deferred with owner authorization |

Keep ordinary CI hermetic: no live Google/OpenAI keys, download of GGUF ASR/model weights, microphone-device requirement, or network-dependent inference. Run real-weight acceptance as opt-in/manual CI with pinned artifacts. On macOS, do **not** describe a network socket probe alone as proof of network denial; use an appropriate enforced egress restriction or report the limitation honestly. A test compilation or an uploaded WAV does not prove live app microphone, real playback, or human listening.

## 6. Rollout, ordering and rollback

1. Freeze baseline and inventory all references; establish macOS linker/FFI and acceptance checks.
2. Port the existing **Whisper** implementation to both Macs **while Moonshine remains the macOS default**. Prove native source build, load, verified test-model install and offline actual transcription; test no dynamic Moonshine dependency for the Whisper path.
3. Switch the macOS default and persisted Moonshine settings to Whisper; qualify full app lifecycle and provider routing on Linux and both Macs. Do not auto-install weights.
4. Remove Moonshine code, native packages, tests, CI, UI and notices in coordinated changes; audit for unintended deletion of KittenTTS ONNX or Whisper evidence. Maintain one consistent `master` history; commit directly to `master` using Ralph Bridge, with exact-head checks.
5. Run ordinary/full CI and release smoke at exact head, then physical acceptance; reconcile the TODO and linked ELSV checklist. Do not mark a gate complete on cancelled, stale-head or docs-only CI.

If Mac Whisper cannot be reliably built or transcribed, **stop before removing the currently selected local Mac ASR** and record the concrete evidence. Revert or correct a failed migration without substituting cloud ASR or falsifying completion. Historical Moonshine evidence may remain in archival documentation but cannot count toward new Whisper acceptance.

## 7. Relationship to external LLM and System Voices scope

This document supersedes Moonshine assumptions in the local-ASR portions of `docs/EXTERNAL_LLM_SYSTEM_VOICES_TODO_2026-10-13.md`; it does **not** waive ELSV-500/900 obligations. Keep macOS `AVSpeechSynthesizer` / System Voices, independent KittenTTS, the provider selection matrix, credential isolation, Gemini Live boundary and no-fallback policy. Update old three-ASR-mode routing tests to the new supported domain rather than deleting coverage without replacement. Human System TTS audible-completion and speech-bubble-duration checks remain independent and open until genuine evidence exists.

## 8. Exit criteria

The migration closes only when every item in its linked TODO has a named exact-head artifact or recorded manual observation; all supported targets use Whisper Small for local ASR; obsolete Moonshine code and distribution binaries are absent; generated contracts, settings upgrades, privacy, release documentation and license evidence agree; and full qualified build/CI plus physical/deferred acceptance are honestly recorded. Passing a documentation-only GitHub Actions workflow does **not** satisfy these criteria.
