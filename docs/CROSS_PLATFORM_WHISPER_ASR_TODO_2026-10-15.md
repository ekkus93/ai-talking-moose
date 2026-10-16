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
- [x] Search every tracked file, module, workflow, test, UI fixture, generated contract, documentation, native resource and manifest for `Moonshine`, `moonshine`, both legacy ASR values, `whisper_native_linked`, `libonnxruntime`, `prepare_moonshine`, and Mac deployment-floor references. Produce an owned disposition list (replace, remove, retain as history, or retain because unrelated). (CI inventory runs `38822092330` and `38823244110`, all 868 tracked paths and 222 matched files classified; one pinned gitlink separately checked. Remaining deletion review is gated under WSP-500.)
- [x] Record the canonical Whisper source/submodule gitlink and model revision, SHA-256, 487,601,967-byte weight, model path, license, default selection and installer behavior before edits. (Pins and historical Linux real-CPU verification in baseline evidence.)
- [x] Identify every macOS app/release CI job and acceptance tool that needs updating: `ci.yml`, `release.yml`, `whisper-real-cpu-acceptance.yml`, Mac native smoke, release validators, signing, notarization, license collection, release docs and physical `P13`. (Owned gate matrix: `docs/evidence/CROSS_PLATFORM_WHISPER_MACOS_GATE_MATRIX_2026-10-15.md`; this inventories required changes, not their completion.)
- [ ] Run/inspect baseline frontend, Rust, generated contract/tree and packaging-policy checks without downloaded weights; distinguish failures already present at baseline.

**Gate WSP-000:** reproducible baseline and reference inventory; no production changes yet.

## WSP-100 — Port the existing Whisper native runtime to macOS (do not remove Moonshine yet)

- [x] Generalize `src-tauri/build.rs` beyond `target_is_linux` to supported Linux x86_64, `aarch64-apple-darwin` and `x86_64-apple-darwin`. Reuse pinned `third_party/whisper.cpp` with CMake, explicit target arch and compatible toolchain. (Mac arm64 and Intel source build, static archives, and real FFI version invocation passed at `6cc3fe302baad4d8d5a579c7bffd65770542131c`, run `38821907488`; other WSP-100 and real-model acceptance remain open.)
- [x] Verify native static libraries `whisper`, `ggml`, `ggml-cpu`, `ggml-base`, transitive macOS frameworks/libc++, and platform link flags. Never accidentally link a developer-installed Homebrew/shared whisper.cpp at runtime. (Both architectures passed exact-source run `38879683273` at `8c9cc830ce80d888d205b53323410e99a200c677`, jobs `116903300742` arm64 and `116903300436` Intel; four archives, `lipo`, `otool -L`, native probe and no dynamic Whisper/ggml dependency. No build/Whisper source change between this SHA and `a133a82daeaf05f496e691a45e87b071a2843079`; see WSP-100 evidence.)
- [ ] Preserve `whisper_native_linked` compilation cfg, source provenance, rerun rules and test-only explicit lib-directory escape hatch. Required macOS release builds must **fail** if source/native link or ABI cannot be established, rather than shipping RuntimeUnavailable.
- [x] Check Whisper FFI signatures, `repr(C)` parameter layout, alignment, pointer ownership and callbacks against pinned headers on both Mac architectures; add native ABI/availability smoke with a real invocation, not just a symbol grep. (Pinned-header C ABI compile and actual native `whisper_version` invocation passed on both Macs in `38853657652` at `35e23b6be427d7856e98f58773171df4dbf2996d`; see `docs/evidence/CROSS_PLATFORM_WHISPER_WSP100_VALIDATION_2026-10-16.md`. This is ABI/loadability evidence, not Intel inference.)
- [x] Keep CPU inference as the qualified baseline on both platforms. Record whether ggml Metal is enabled/disabled in each build and do not add Metal unless optional and independently tested. (macOS arm64 and Intel genuine CPU-only inference passed in run `38886320925` on SHA `999a04e3aefaa9b93f30ad0c53b188009f9f228c`; both logs report `use gpu = 0`, `flash attn = 0`, `no GPU found`. Pinned native CMake disables Metal, Accelerate, BLAS. Linux CPU baseline previously verified; see new WSP-200 evidence.)
- [x] Prove Linux build behavior, pinned source provenance and existing build/test outcomes are not regressed by the macOS CMake changes. (Linux Rust tests, formatting, Clippy and Whisper provenance all passed in exact-source run `38853657618` at `35e23b6be427d7856e98f58773171df4dbf2996d`; see `docs/evidence/CROSS_PLATFORM_WHISPER_WSP100_VALIDATION_2026-10-16.md`. No real-model Linux replay claimed.)

**Gate WSP-100:** real native Whisper runtime is linked and loadable on macOS arm64 and x86_64; Linux still builds. Moonshine is **still present** pending WSP-200.

## WSP-200 — Real Whisper inference, installer and performance on both Macs (mandatory pre-cutover gate)

- [ ] Use the existing production `WhisperModelInstaller` and exact manifest on Mac. Verify explicit install, clean reinstall, cancellation, corruption/hash mismatch, missing weight, disk space, model lease and safe delete/update interaction.
- [x] With the actual pinned `ggml-small.bin` downloaded into user/test data only, run `whisper_acceptance` fixed-corpus inference on macOS arm64; capture exact commit, source/model SHA, real transcript and JSON reports. (Passed arm64 job `116799364037` in manual run `38846896241` at SHA `df9411fde91ecbe245de662cc92b2f4a3af3f96f`; artifact `12023986115`. CPU-only backend assurance requires the new explicit-context fix to be rerun. See `docs/evidence/CROSS_PLATFORM_WHISPER_MACOS_REAL_MODEL_PARTIAL_2026-10-15.md`.)
- [x] Repeat equivalent pinned-model/fixed-corpus native inference on macOS x86_64; do not treat arm64 success, a mock, or compile-only smoke as Intel success. (Passed exact-head `999a04e3aefaa9b93f30ad0c53b188009f9f228c` run `38886320925`, Intel job `116924832215`; CPU-only, production-installer verified, genuine JFK corpus transcription. Evidence: `docs/evidence/CROSS_PLATFORM_WHISPER_MACOS_REAL_CPU_SUCCESS_2026-10-16.md`.)
- [ ] Verify genuine offline inference after install on both Macs using an enforced macOS-appropriate egress-denial mechanism; if the enforcement cannot be demonstrated, explicitly record the limitation and leave that acceptance checkbox open.
- [ ] Record first partial/final latency, utterance accuracy/content recall, real-time factor, model load time, CPU thread count, RSS/peak memory, buffer occupancy/drops and sustained/repeated utterance behavior for both architectures; compare against Linux's pinned baseline.
- [ ] Run native lifecycle tests on Mac (model loading/teardown; PCM sample rate/format; silence endpoint; abort; worker join; rapid start/stop; multi-utterance; overload), with no hang, zombie worker, uncaught FFI failure or duplicate final events.
- [ ] Document any accuracy/latency/thermal regression; tune batch/window/queue limits using evidence, or escalate any unacceptable target-platform regression as an explicit product decision.

**HARD CUTOVER GATE WSP-200:** *Both* Mac architectures demonstrably transcribe real audio with the pinned Whisper model and the packaged-compatible native runtime. **Do not remove Moonshine or switch production Mac default before this gate.**

**Update 2026-10-16: two-architecture real-model CPU transcription PASS.** Workflow `38886320925` on SHA `999a04e3aefaa9b93f30ad0c53b188009f9f228c` passed both ARM64 and Intel with pinned `ggml-small.bin`, CPU-only backend, real JFK text and JSON artifacts. See `docs/evidence/CROSS_PLATFORM_WHISPER_MACOS_REAL_CPU_SUCCESS_2026-10-16.md`. This satisfies the mandatory *two-architecture transcription prerequisite* for moving on to WSP-300, but **does not** prove enforced offline macOS egress, full WSP-200 lifecycle/performance, packaged app, real microphone or WSP-400/WSP-700. Continue to preserve Moonshine until the app-level migration is qualified; KittenTTS stays separate.

**Historical acceptance state (2026-10-15; superseded by the 2026-10-16 two-architecture pass):** Human-dispatched run `38846896241` at `df9411fde91ecbe245de662cc92b2f4a3af3f96f` passed real arm64 inference. Intel initially hit an HTTPS timeout; targeted rerun attempt 2, job `116819455524`, verified all 487,601,967 model bytes but aborted with `GGML_ASSERT(buf_dst)` under the older Metal backend. The retry and CPU-only changes were later qualified by genuine **CPU-only arm64 and Intel transcription success** in run `38886320925` at `999a04e3aefaa9b93f30ad0c53b188009f9f228c`; the former "Intel unverified" conclusion no longer applies. Earlier Ralph Bridge dispatch errors were recorded in `docs/evidence/CROSS_PLATFORM_WHISPER_WSP200_DISPATCH_2026-10-15.md` and should not be presented as current without rechecking. The **remaining** WSP-200 checks for enforced offline egress, complete lifecycle/performance, and physical packaged-app acceptance remain open. Do not repair the retired Moonshine ONNX/LFS fallback merely to keep it alive; coordinate replacement of legacy gates after WSP-300/400 while preserving independent KittenTTS ONNX Runtime.

## WSP-300 — Unify ASR selection and persist settings safely

- [x] Make `AsrMode::WhisperSmall` the fresh-profile local ASR default on Linux and macOS. Retain explicit `GeminiLiveAudio` cloud mode with no implicit selection. (`AsrMode` platform cfg and `AppSettings::default`/migration tests, exact-source main CI `38893284623` at `fe3ddc642b73f57aa88ab8ccd1cb0ad4272f3934`: Rust quality, Rust tests, generated contract all passed. This is a code/defaults gate, not physical Mac app acceptance.)
- [x] Design an idempotent migration for persisted `moonshine_tiny_streaming` and `moonshine_small_streaming` to `whisper_small`. Decide and document settings-version treatment and preserve unknown/additive fields. (Design: `docs/evidence/CROSS_PLATFORM_WHISPER_SETTINGS_MIGRATION_DESIGN_2026-10-15.md`; implementation now rewrites legacy values in `AppSettings::from_persisted_json` under settings schema v9. Full direct-IPC, restart, Mac and model-state qualification remains open in the following checkboxes.)
- [ ] Ensure migration **does not** claim a Whisper model is installed, download weights, silently enable cloud, substitute Moonshine model files, overwrite API/voice/TTS/text selections, or lose privacy/memory flags.
- [ ] Test fresh Linux/Mac profiles, both persisted Moonshine variants, explicit saved Whisper, explicit Gemini Live, corrupted/invalid/future-version values, and save/restart round trips. Confirm removed choices are never resurrected.
- [ ] Make ASR catalog/commands/diagnostics truthfully show Whisper availability, download state, model revision, batch partial semantics and runtime errors on both Macs. Preserve install/delete/session ownership rules.
- [x] Update Voice/AI Control Panel and onboarding to expose only supported ASR choices; preserve clear local/cloud disclosure, no fallback and optional explicit download. Remove obsolete Moonshine UI from active choices only after WSP-200. (Whisper-only local selector, explicit Gemini Live, onboarding optional install and truthful batch diagnostics; frontend CI `38893061667` passed at `ba6f871acd496db4f6fa584e32852c9f2834ea38`. Evidence: `docs/evidence/CROSS_PLATFORM_WHISPER_WSP300_UI_2026-10-16.md`. This does not qualify packaged Mac app or WSP-300 as a whole.)
- [ ] Update TypeScript/Rust type switches, IPC fixtures, backend contract generation and tests. Preserve provider snapshots, Gemini Live isolation and settings compatibility.

**WSP-300 direct-IPC gap corrected (2026-10-16):** At `bde643a71498e22f9b5d06632d03eadc74dca906`, the shared settings validation boundary rejects new Moonshine Tiny/Small selections, while startup migration still decodes both historical persisted enum variants into `whisper_small`; the obsolete macOS-only Whisper rejection was removed. Formatting and regression coverage were completed through `ab30cca6438d88b9853ac2deccc7b5cda6d3ff27`. Exact-head main CI `38897533479` passed Rust tests, Rust quality, and generated contract; Wake Word audit `38897533475` and lifecycle `38897533476` passed. SQLite migration tests preserve existing provider/privacy flags, reject unknown ASR values, verify idempotence, and do not download/install Whisper weights. See `docs/evidence/CROSS_PLATFORM_WHISPER_WSP300_IPC_2026-10-16.md`. The enum variants remain decodable *only* for legacy migration; they are not valid new settings writes. macOS packaged-app/physical acceptance and complete WSP-300 provider matrix are still open.

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

**Partial WSP-400 lifecycle regression (2026-10-16):** Direct-to-master `50dcce604acb7fbe0b20e4ec7b96a1f5c0f34f39` adds three deterministic Whisper utterance-state tests for stale/duplicate final-delivery acknowledgements across consecutive utterances, silence endpoint accumulation across chunks, and maximum-duration finality taking precedence over partial refresh. Exact-head main CI `38901291197` passed Rust tests, Rust quality and generated contract. These are isolated state-machine unit tests, **not** a claim of live capture, real-model native lifecycle, end-to-end provider routing, packaged Mac operation or physical microphone acceptance; all WSP-400 acceptance boxes remain open.

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
| WSP-200 | `999a04e3aefaa9b93f30ad0c53b188009f9f228c` (two-arch real-corpus evidence, other gates open) | macOS ARM64 and x86_64 CPU-only | Run `38886320925`, jobs `116924832423` / `116924832215` | Both passed pinned install and real JFK transcript with CPU-only logs; ARM64 2,319 ms, Intel 11,502 ms transcribe wall; peak RSS 865,730,560 / 843,063,296 bytes | Enforced macOS network denial, packaged app, native lifecycle matrix and physical mic still unqualified |
| WSP-300 | `ba6f871acd496db4f6fa584e32852c9f2834ea38` (UI); `fe3ddc642b73f57aa88ab8ccd1cb0ad4272f3934` (defaults); `ab30cca6438d88b9853ac2deccc7b5cda6d3ff27` (IPC/SQLite regressions) | Frontend and Linux Rust unit tests | CI `38893061667` frontend success; CI `38893284623` defaults quality/tests/contract success; CI `38897533479` Rust quality/tests/contract success; Wake audit `38897533475` + lifecycle `38897533476` success | Whisper-only UI, retired modes rejected on new settings writes, both persisted Moonshine variants migrate to Whisper once without implicit model installation; explicit Gemini Live preserved | Native macOS packaged app, complete provider/routing matrix, ASR-015 replacement, physical Mac acceptance still open |
| WSP-400 | Pending | Pending | Pending | Pending | Pending |
| WSP-500 | Pending | Pending | Pending | Pending | Pending |
| WSP-600 | Pending | Pending | Pending | Pending | Pending |
| WSP-700 | Pending | Pending | Pending | Pending | Pending |
