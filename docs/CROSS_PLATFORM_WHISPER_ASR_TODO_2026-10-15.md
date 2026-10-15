# Cross-Platform Whisper.cpp Local ASR — Implementation TODO

**Date:** 2026-10-15
**Status:** Open — architecture authorized; implementation and acceptance outstanding
**Specification:** `docs/CROSS_PLATFORM_WHISPER_ASR_SPEC_2026-10-15.md`
**Repository:** `ekkus93/ai-talking-moose`, direct-to-`master` workflow
**Current baseline:** `2582159ee36dd8690e7443822d0da4be4e911d9a`
**Previous Linux Whisper work:** `docs/WHISPER_CPP_LOCAL_ASR_TODO.md` (historically completed; do not reopen completed work without a regression)
**Related active checklist:** `docs/EXTERNAL_LLM_SYSTEM_VOICES_TODO_2026-10-13.md`

Check a box **only** when a concrete implementation and its stated evidence exist at an observed commit. Record exact SHA, workflow/run and job conclusions, target/arch, commands, numerical measurements where requested, and limitations. A cancelled run, successful docs-only gate, generated stub transcript, or compilation without running inference is not sufficient proof. No item is completed merely because this TODO was committed.

## Ralph Loop operating rules

- [ ] On every run, inspect current `master` exact SHA, this spec/TODO, relevant existing branches/PRs/commits and CI through **Ralph Bridge only**; reconcile any concurrent work before acting.
- [ ] Work directly on `master` with exact-head compare-and-swap commits; do **not** create a new branch or PR for each task. Do not use the default GitHub connector.
- [ ] After every commit/merge, re-read the current TODO and CI at the actual head. Correct normal test failures, stale-head writes, conflicts and transient errors instead of treating them as user blockers.
- [ ] Preserve all prior passing features and evidence; write concise per-phase completion evidence in this TODO or `docs/evidence/`. Do not mark hardware/credential-dependent checks passed without actual evidence.
- [ ] Enforce the **two-stage safety gate**: port and demonstrate genuine macOS Whisper transcription **before** changing Mac defaults or deleting any working Moonshine runtime. Prefer fixing/qualifying existing work over duplicate implementations.

## WSP-000 — Inventory and baseline (first)

- [x] Resolve the current head and record the Linux Whisper exact-source, current Mac Moonshine CI status, model/source pins, failing native dependency and historical benchmarks; distinguish historical claims from newly observed evidence. (Baseline evidence: `docs/evidence/CROSS_PLATFORM_WHISPER_BASELINE_2026-10-15.md`.)
- [ ] Search every tracked file, module, workflow, test, UI fixture, generated contract, documentation, native resource and manifest for `Moonshine`, `moonshine`, both legacy ASR values, `whisper_native_linked`, `libonnxruntime`, `prepare_moonshine`, and Mac deployment-floor references. Produce an owned disposition list (replace, remove, retain as history, or retain because unrelated).
- [x] Record the canonical Whisper source/submodule gitlink and model revision, SHA-256, 487,601,967-byte weight, model path, license, default selection and installer behavior before edits. (Pins and historical Linux real-CPU verification in baseline evidence.)
- [ ] Identify every macOS app/release CI job and acceptance tool that needs updating: `ci.yml`, `release.yml`, `whisper-real-cpu-acceptance.yml`, Mac native smoke, release validators, signing, notarization, license collection, release docs and physical `P13`.
- [ ] Run/inspect baseline frontend, Rust, generated contract/tree and packaging-policy checks without downloaded weights; distinguish failures already present at baseline.

**Gate WSP-000:** reproducible baseline and reference inventory; no production changes yet.

## WSP-100 — Port the existing Whisper native runtime to macOS (do not remove Moonshine yet)

- [ ] Generalize `src-tauri/build.rs` beyond `target_is_linux` to supported Linux x86_64, `aarch64-apple-darwin` and `x86_64-apple-darwin`. Reuse pinned `third_party/whisper.cpp` with CMake, explicit target arch and compatible toolchain.
- [ ] Verify native static libraries `whisper`, `ggml`, `ggml-cpu`, `ggml-base`, transitive macOS frameworks/libc++, and platform link flags. Never accidentally link a developer-installed Homebrew/shared whisper.cpp at runtime.
- [ ] Preserve `whisper_native_linked` compilation cfg, source provenance, rerun rules and test-only explicit lib-directory escape hatch. Required macOS release builds must **fail** if source/native link or ABI cannot be established, rather than shipping RuntimeUnavailable.
- [ ] Check Whisper FFI signatures, `repr(C)` parameter layout, alignment, pointer ownership and callbacks against pinned headers on both Mac architectures; add native ABI/availability smoke with a real invocation, not just a symbol grep.
- [ ] Keep CPU inference as the qualified baseline on both platforms. Record whether ggml Metal is enabled/disabled in each build and do not add Metal unless optional and independently tested.
- [ ] Prove Linux build behavior, pinned source provenance and existing build/test outcomes are not regressed by the macOS CMake changes.

**Gate WSP-100:** real native Whisper runtime is linked and loadable on macOS arm64 and x86_64; Linux still builds. Moonshine is **still present** pending WSP-200.

## WSP-200 — Real Whisper inference, installer and performance on both Macs (mandatory pre-cutover gate)

- [ ] Use the existing production `WhisperModelInstaller` and exact manifest on Mac. Verify explicit install, clean reinstall, cancellation, corruption/hash mismatch, missing weight, disk space, model lease and safe delete/update interaction.
- [ ] With the actual pinned `ggml-small.bin` downloaded into user/test data only, run `whisper_acceptance` fixed-corpus inference on macOS arm64; capture exact commit, source/model SHA, real transcript and JSON reports.
- [ ] Repeat equivalent pinned-model/fixed-corpus native inference on macOS x86_64; do not treat arm64 success, a mock, or compile-only smoke as Intel success.
- [ ] Verify genuine offline inference after install on both Macs using an enforced macOS-appropriate egress-denial mechanism; if the enforcement cannot be demonstrated, explicitly record the limitation and leave that acceptance checkbox open.
- [ ] Record first partial/final latency, utterance accuracy/content recall, real-time factor, model load time, CPU thread count, RSS/peak memory, buffer occupancy/drops and sustained/repeated utterance behavior for both architectures; compare against Linux's pinned baseline.
- [ ] Run native lifecycle tests on Mac (model loading/teardown; PCM sample rate/format; silence endpoint; abort; worker join; rapid start/stop; multi-utterance; overload), with no hang, zombie worker, uncaught FFI failure or duplicate final events.
- [ ] Document any accuracy/latency/thermal regression; tune batch/window/queue limits using evidence, or escalate any unacceptable target-platform regression as an explicit product decision.

**HARD CUTOVER GATE WSP-200:** *Both* Mac architectures demonstrably transcribe real audio with the pinned Whisper model and the packaged-compatible native runtime. **Do not remove Moonshine or switch production Mac default before this gate.**

## WSP-300 — Unify ASR selection and persist settings safely

- [ ] Make `AsrMode::WhisperSmall` the fresh-profile local ASR default on Linux and macOS. Retain explicit `GeminiLiveAudio` cloud mode with no implicit selection.
- [x] Design an idempotent migration for persisted `moonshine_tiny_streaming` and `moonshine_small_streaming` to `whisper_small`. Decide and document settings-version treatment and preserve unknown/additive fields. (Design only, not activated: `docs/evidence/CROSS_PLATFORM_WHISPER_SETTINGS_MIGRATION_DESIGN_2026-10-15.md`; requires WSP-200 before implementation.)
- [ ] Ensure migration **does not** claim a Whisper model is installed, download weights, silently enable cloud, substitute Moonshine model files, overwrite API/voice/TTS/text selections, or lose privacy/memory flags.
- [ ] Test fresh Linux/Mac profiles, both persisted Moonshine variants, explicit saved Whisper, explicit Gemini Live, corrupted/invalid/future-version values, and save/restart round trips. Confirm removed choices are never resurrected.
- [ ] Make ASR catalog/commands/diagnostics truthfully show Whisper availability, download state, model revision, batch partial semantics and runtime errors on both Macs. Preserve install/delete/session ownership rules.
- [ ] Update Voice/AI Control Panel and onboarding to expose only supported ASR choices; preserve clear local/cloud disclosure, no fallback and optional explicit download. Remove obsolete Moonshine UI from active choices only after WSP-200.
- [ ] Update TypeScript/Rust type switches, IPC fixtures, backend contract generation and tests. Preserve provider snapshots, Gemini Live isolation and settings compatibility.

**Gate WSP-300:** installed and freshly created Mac/Linux profiles select usable Whisper or deliberately surface missing-model error; legacy Moonshine settings migrate predictably without silent network behavior.

## WSP-400 — Unified capture, conversation, wake word and regression matrix

- [ ] Reuse existing shared `LocalAsrPipeline::start_whisper`, one CPAL microphone, 16 kHz mono PCM, bounded 56-chunk Whisper queue and drop-newest policy; delete duplicated engine selection branches only when the unified path is proven.
- [ ] Verify local wake-word handoff (pre-roll ordering, wake/command split and microphone ownership), no double capture, delayed partials, local finalization and no duplicated or reordered `AsrEvent`.
- [ ] Verify conversation start, stop, barge-in, user-selected input/output devices, permissions, cancellation and stale-generation suppression. Ensure worker joins and active leases prevent unsafe model replacement.
- [ ] Update non-Live Talk routing/negative tests for **Whisper Small × each supported text provider × supported standalone TTS**. Keep fail-closed no-key/no-model/no-voice cases and immutable snapshot assertions. Do not delete an old Moonshine matrix test without a replacement covering the new supported mode.
- [ ] Test explicit Gemini Live audio separately: no accidental Live connection from local Whisper selection, no silent fallback, and correctly disclosed provider scope.
- [ ] Verify typed replies, ambient responses, actual TTS playback and speech-bubble lifecycle are not regressed; distinguish automated event tests from physical playback evidence.
- [ ] Record tests for model missing/corrupt, native unavailable, mic unavailable, queue overflow, malformed PCM, repeated start/stop, cancellation and in-flight provider switch.

**Gate WSP-400:** correct single-engine app routing across targets; no loss of previous ELSV-400 or privacy guarantees.

## WSP-500 — Remove Moonshine and obsolete dependencies (only after WSP-200/300/400)

- [ ] Remove `src-tauri/src/asr/moonshine/`, Moonshine factories/types, architecture variants and default branches, old installers/commands, native load probes and active Moonshine test fixtures; keep useful provider-neutral tests.
- [ ] Remove the Moonshine-specific build/link logic from `src-tauri/build.rs`, `TALKING_MOOSE_MOONSHINE_LIB_DIR` and Moonshine cfg; use only the cross-platform Whisper native build.
- [ ] Remove `scripts/prepare_moonshine_macos.sh`, `src-tauri/native/moonshine-runtime.json` and generated/staged Moonshine dylibs/notices from the shipping path; delete deprecated provenance validators and wrappers only if no other feature uses them.
- [ ] Update `src-tauri/tauri.conf.json`: eliminate Moonshine-only `bundle.macOS.frameworks` entries and resources; forbid unused dynamic lib payloads. Verify static whisper.cpp license notice still ships.
- [ ] Remove Moonshine build/source/LFS references from `.github/workflows/ci.yml`, `.github/workflows/release.yml`, Mac smoke, native architecture checkers, release verifier, package policy and dependency-license reports. Replace with real Whisper provenance/native checks, **never** simply relax a gate.
- [ ] Remove obsolete Moonshine model installation UI/progress, API calls, test data, generated contract variants and user-facing instructions. Keep archived Moonshine benchmarking/incident evidence clearly labeled historical if useful.
- [ ] Preserve separately owned `ort` and verified ONNX Runtime artifacts required by **KittenTTS**. Verify `Local KittenTTS` downloads, offline synthesis, playback, no fallback and packaging policy before and after deletion.
- [ ] Search the entire tracked repository again for Moonshine/legacy enum variants/dylib paths. Every remaining match must be archived history or have a documented reason. Check no third-party submodule/source or license needed by Whisper was accidentally removed.

**Gate WSP-500:** no Moonshine runtime, fallback, download or distributable dylib remains; Whisper and KittenTTS still work.

## WSP-600 — CI, native release and documentation reconciliation

- [ ] Run exact-head `CARGO_BUILD_JOBS=2 npm run check:all`, `npm run check:generated-trees`, `python3 scripts/check_local_llm_packaging_policy.py`, `python3 scripts/check_local_tts_packaging_policy.py`, provenance, licensing and audited dependency gates; repair all failures.
- [ ] Extend `.github/workflows/whisper-real-cpu-acceptance.yml` or add clearly named Mac acceptance jobs for arm64 and Intel. Keep weight downloads in explicit/manual acceptance, not in ordinary CI; preserve Linux network-denied proof and report Mac egress limitations truthfully.
- [ ] Run ordinary macOS arm64 and x86_64 checks at the **exact current head**: CMake compile/static link, native availability, frontend/backend, contract, Tauri unsigned `.app`, architecture/loader/provenance smoke and clean-machine startup.
- [ ] Update release workflow, `scripts/verify_macos_bundle.sh`, `scripts/verify_macos_release.sh`, `docs/MACOS_RELEASE.md`, deployment-target assumptions, notarization/signing license inventory and P13 physical acceptance to assert Whisper native presence and Moonshine absence. Determine macOS minimum from the *remaining* dependencies; do not lower 13.4 without evidence.
- [ ] Verify no home-machine rpath or library dependencies, no unexpected native dylib, no shipped ASR GGML weights, no secret/log/database artifact, and valid notarization/Gatekeeper checks on both architectures.
- [ ] Update `README.md`, `docs/PRIVACY.md`, `docs/WHISPER_CPP_LOCAL_ASR_PIPELINE.md`, `docs/LOCAL_LLM_ARCHITECTURE.md`, `docs/VOICE_SELECTION.md`, onboarding, settings copy and third-party notices to describe the unified Whisper behavior and Gemini Live exception.
- [ ] Reconcile `docs/EXTERNAL_LLM_SYSTEM_VOICES_TODO_2026-10-13.md`: replace outdated Moonshine routing assumptions, retain exact qualified non-Live test coverage for new ASR domain, and **leave physical macOS System TTS audition/bubble and ELSV-900 release gates open** until actually observed.
- [ ] Record exact commit, CI workflow/run IDs, relevant job conclusions and source/model hashes for Linux x86_64, macOS arm64 and macOS x86_64. Do not count superseded/cancelled/docs-only CI as a production qualification pass.

**Gate WSP-600:** CI and release artifacts fully verify Whisper on all supported targets, with complete up-to-date platform documentation and no weakened checks.

## WSP-700 — Physical acceptance and closeout

- [ ] On a supported Apple Silicon Mac, install the intended packaged app on a clean profile: explicitly install/verify Whisper Small, grant microphone permission, speak multiple utterances, test wake word, interruption, partial/final transcript, selected text/TTS, audio device and speech-bubble completion. Record OS/build, voice/device, expected/observed behavior and evidence.
- [ ] Repeat required clean-machine acceptance on an Intel Mac, or record a **named owner-authorized deferral** as a release blocker/exception rather than declaring success. CI compilation does not substitute for a physical observation.
- [ ] Re-check Linux real mic and original Whisper CPU acceptance on a representative supported host. No accidental network or provider fallback.
- [ ] Run macOS System Voices human audition/bubble checks separately under ELSV-500/900; do not mislabel local ASR qualification as macOS System TTS acceptance.
- [ ] Verify clean install, upgrade from saved Moonshine selection, explicit model installation, offline operation, update/rollback, privacy/consent, uninstall/reinstall, and absence of residual Moonshine binary/resource references.
- [ ] Audit all TODO checkboxes against recorded evidence; document remaining intentional deferrals, stop-ship decisions and user-required hardware/credentials. Mark complete only when every requirement is proved or an explicitly approved exception is documented.
- [ ] Re-read final `master` SHA after last commit; inspect exact-head CI and final diff; confirm only intended files and artifacts landed and no user models, tokens or generated machine output are committed.

**Final gate:** one verified local Whisper Small ASR implementation on Linux/macOS, no Moonshine packaging/build/code path, preserved Local TTS/KittenTTS and Gemini Live isolation, correct migration, real macOS and Linux evidence, complete release checks, and honest ELSV reconciliation.

## Evidence log

Record each phase's completion here or link a dedicated file under `docs/evidence/` with the table below. Do not fabricate values.

| Phase | Head SHA | Platform / architecture | Exact CI run / command | Outcome / measurement | Remaining limitation |
| --- | --- | --- | --- | --- | --- |
| WSP-000 | Pending | Pending | Pending | Pending | Pending |
| WSP-100 | Pending | Pending | Pending | Pending | Pending |
| WSP-200 | Pending | Pending | Pending | Pending | Pending |
| WSP-300 | Pending | Pending | Pending | Pending | Pending |
| WSP-400 | Pending | Pending | Pending | Pending | Pending |
| WSP-500 | Pending | Pending | Pending | Pending | Pending |
| WSP-600 | Pending | Pending | Pending | Pending | Pending |
| WSP-700 | Pending | Pending | Pending | Pending | Pending |
