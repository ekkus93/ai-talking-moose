# AI Talking Moose — Post-KittenTTS Code Review Remediation TODO

**Date:** 2026-09-12  
**Companion specification:** `docs/POST_KITTENTTS_CODE_REVIEW_REMEDIATION_SPEC_2026-09-12.md`  
**Qualified baseline:** `3e63d51c68888a23cd98ceef052007eb3c833a45` (`master`)  
**Status:** Active remediation queue

Task IDs use the `KCR-###` prefix (**KittenTTS Code-review Remediation**). This tracker is intentionally narrower than `docs/TODO(20260909-120003).md`: already-qualified KTT-000 through KTT-505 implementation is not reopened unless a task below finds a regression.

---

# R0 — Freeze the Remediation Baseline

## KCR-001 — Record exact qualified baseline and scope

- [x] Record baseline `3e63d51c68888a23cd98ceef052007eb3c833a45`.
- [x] Record ordinary CI `34694611931` passing on the exact baseline.
- [x] Record real Kitten CPU acceptance `34694611936` passing on the exact baseline.
- [x] Preserve existing standalone synthesis/playback architecture.
- [x] Preserve explicit Google/Local no-fallback policy.
- [x] Preserve separate Gemini Live native-audio path.
- [x] Keep ordinary CI model-weight-free.

**Acceptance**

- [x] Remediation begins from one exact, already-qualified master SHA.

---

# R1 — Coherent Local TTS Diagnostics

## KCR-100 — Add one safe composed Local TTS diagnostics DTO

- [ ] Add typed Rust `LocalTtsDiagnostics` or equivalent.
- [ ] Include selected standalone TTS provider.
- [ ] Include selected Local model ID.
- [ ] Include selected Local voice ID.
- [ ] Include Local model install state.
- [ ] Include expected artifact bytes.
- [ ] Include installed bytes when known.
- [ ] Include existing `LocalTtsRuntimeStatus` as the runtime subsection.
- [ ] Include runtime lifecycle state.
- [ ] Include sample rate and inference thread count when known.
- [ ] Include model-load, synthesis, generated-audio-duration, and RTF metrics when known.
- [ ] Include safe installer error category/retryability when known.
- [ ] Include safe runtime error category when known.
- [ ] Exclude utterance text.
- [ ] Exclude raw PCM/audio.
- [ ] Exclude credentials/API keys/tokens.
- [ ] Exclude full filesystem paths where model IDs suffice.
- [ ] Avoid arbitrary raw exception/error strings.
- [ ] Capture a coherent settings snapshot before composing installer/runtime state.

**Acceptance**

- [ ] One response answers selected provider/model/voice plus installed/loaded/generating/ready/failed state without developer logs.
- [ ] DTO has no field capable of carrying utterance text, raw audio, credentials, or full model paths.

## KCR-101 — Preserve typed installer error fidelity

- [ ] Add a narrow read-only selected-model installer error lookup.
- [ ] Return existing safe `LocalTtsInstallErrorKind` rather than exposing installer internals.
- [ ] Return retryability separately where useful.
- [ ] Use a conservative safe fallback when status reports failure but no typed operation error is recorded.
- [ ] Add focused tests for selected-model filtering and stale/unrelated installer errors.

**Acceptance**

- [ ] Diagnostics do not misattribute another model's last installer error.
- [ ] No raw network/body/path data is exposed to the UI.

## KCR-110 — Expose diagnostics through production Tauri IPC

- [ ] Add `get_local_tts_diagnostics` or equivalent production command.
- [ ] Register the command in the Tauri invoke handler.
- [ ] Reuse the shared `AppState` runtime manager and installer.
- [ ] Do not add a parallel diagnostics/runtime state machine.
- [ ] Add command-level tests for installed/uninstalled/failed states as practical.

**Acceptance**

- [ ] Production Tauri can fetch the composed response through a registered command.
- [ ] Command composition does not perform network access or synthesis.

## KCR-111 — Carry Local TTS diagnostics through the generated contract

- [ ] Add representative Rust `LocalTtsDiagnostics` contract value.
- [ ] Regenerate `src/generated/backendContract.json`.
- [ ] Add exact TypeScript runtime/diagnostics interfaces.
- [ ] Add exact Local TTS runtime phase union.
- [ ] Add exact Local TTS runtime error-kind union.
- [ ] Update native Tauri bridge return type.
- [ ] Update browser-preview bridge with privacy-safe representative diagnostics.
- [ ] Update frontend IPC shape checker coverage.
- [ ] Extend the negative contract probe to damage a Local TTS diagnostics field.
- [ ] Prove the negative probe fails for Rust/TS diagnostic drift.

**Acceptance**

- [ ] Rust-side diagnostics field drift is caught by the generated-contract gate.
- [ ] No unchecked handwritten JSON cast bypasses the contract.

## KCR-120 — Add production Local TTS diagnostics UI

- [ ] Add Local TTS diagnostics section/panel to Settings diagnostics.
- [ ] Display selected provider.
- [ ] Display selected Local model ID.
- [ ] Display selected Local voice ID.
- [ ] Display install state.
- [ ] Display runtime lifecycle phase.
- [ ] Display loaded model ID when present.
- [ ] Display sample rate when present.
- [ ] Display inference thread count when present.
- [ ] Display model-load duration when present.
- [ ] Display synthesis duration when present.
- [ ] Display generated audio duration when present.
- [ ] Display RTF when present.
- [ ] Display safe installer/runtime error categories when present.
- [ ] Refresh while diagnostics tab is mounted.
- [ ] Prevent overlapping refresh requests.
- [ ] Stop polling/ignore stale responses after unmount.
- [ ] Render a generic unavailable state instead of raw IPC exception text.
- [ ] Add frontend tests for loading/success/unavailable states.

**Acceptance**

- [ ] Production UI shows live Local TTS operational state without developer logs.
- [ ] UI never renders utterance text, raw audio, credentials, or full model paths.

## KCR-130 — Add explicit lifecycle and non-vacuous privacy tests

- [ ] Add deterministic test control point for observing runtime `loading`.
- [ ] Prove initial state is `unloaded`.
- [ ] Prove `unloaded -> loading -> ready` around model load.
- [ ] Add deterministic test control point for observing `generating`.
- [ ] Prove `ready -> generating -> ready|failed` around synthesis/cancellation.
- [ ] Inject a unique utterance sentinel into a synthesis path.
- [ ] Serialize/fetch Local TTS diagnostics after the sentinel operation.
- [ ] Assert the sentinel is absent from diagnostics serialization.
- [ ] Assert the sentinel is absent from rendered diagnostics UI.
- [ ] Add positive controls proving selected model, selected voice, and runtime state are present.

**Acceptance**

- [ ] Lifecycle assertions observe real production manager state transitions rather than a test-only mirror.
- [ ] Privacy assertions would fail if diagnostics accidentally started carrying utterance text.

---

# R2 — Native Packaging, Provenance, and Licensing

## KCR-200 — Qualify Local TTS runtime integration for all supported targets

Required targets:

- [ ] Linux x86_64.
- [ ] macOS arm64.
- [ ] macOS x86_64.

For every native/runtime component:

- [ ] Pin exact source/release identity.
- [ ] Pin binary checksum when prebuilt artifacts are used.
- [ ] Keep runtime independent of Homebrew at application runtime.
- [ ] Keep runtime independent of apt/system packages at application runtime.
- [ ] Keep runtime independent of Python/Conda at application runtime.
- [ ] Add deterministic build/preparation script where repository preparation is required.
- [ ] Verify a fresh packaged application has every non-model runtime dependency it needs.

**Acceptance**

- [ ] Local TTS runtime does not depend on a developer machine or package manager after packaging.

## KCR-201 — Add macOS architecture and bundle-load proof

- [ ] Verify arm64 native libraries contain arm64 architecture.
- [ ] Verify x86_64 native libraries contain x86_64 architecture.
- [ ] Verify app bundles contain required Local TTS runtime files.
- [ ] Verify load commands contain no developer-machine absolute paths.
- [ ] Verify rpaths/install names are bundle-safe.
- [ ] Pass unsigned arm64 bundle smoke.
- [ ] Pass unsigned x86_64 bundle smoke.
- [ ] Add exact provenance verifier for packaged Local TTS native runtime.

**Acceptance**

- [ ] Wrong-architecture, missing-runtime, and developer-linked bundles fail before release.

## KCR-210 — Extend release dependency/license collection to Local TTS

- [ ] Collect runtime library license text.
- [ ] Collect Kitten model license/notice evidence.
- [ ] Collect G2P/tokenizer license evidence.
- [ ] Collect voice embedding/license evidence.
- [ ] Collect ONNX Runtime license/notice if shipped.
- [ ] Collect every other shipped Local TTS support asset/license.
- [ ] Update generated release inventory/`THIRD_PARTY_NOTICES` as appropriate.
- [ ] Fail release-static gate when required evidence is absent.
- [ ] Do not infer sub-artifact license terms from a parent project.

**Acceptance**

- [ ] Release license inventory matches the resources actually shipped by the application.

## KCR-211 — Add Local TTS packaging-policy static gates

- [ ] Fail if Local TTS model weights are embedded in ordinary application resources without explicit approval.
- [ ] Fail if unapproved eSpeak/GPL code or data is packaged.
- [ ] Fail if selected runtime identity is unpinned.
- [ ] Fail if required native libraries are present for the wrong target.
- [ ] Fail if model source URLs are mutable/unpinned or non-HTTPS.
- [ ] Verify application bundle-size policy remains truthful.

**Acceptance**

- [ ] Forbidden, unlicensed, unpinned, or wrong-target Local TTS resources fail CI before release packaging.

## KCR-220 — Preserve model-weight-free ordinary builds and tests

- [x] Real Kitten model is not committed to Git.
- [x] Ordinary baseline CI does not download the real model.
- [ ] Ensure ordinary Local TTS logic tests use fixtures/mock runtime.
- [ ] Keep real-model acceptance in an explicit separately-gated workflow.
- [ ] Add/retain path classification so unrelated changes do not invoke expensive Local TTS acceptance.

**Acceptance**

- [ ] Ordinary unrelated PRs remain fast and do not incur Kitten model downloads.

---

# R3 — Real-Model CPU Acceptance and Performance

## KCR-300 — Harden the focused real KittenTTS acceptance harness

- [ ] Acquire exact pinned artifacts with cache support.
- [ ] Verify exact byte counts before use.
- [ ] Verify SHA-256 before use.
- [ ] Explicitly load CPU runtime.
- [ ] Synthesize representative short Moose lines.
- [ ] Synthesize at least one longer sentence.
- [ ] Exercise multiple Kitten voices.
- [ ] Assert non-empty finite PCM.
- [ ] Assert expected source sample rate.
- [ ] Measure model-load latency.
- [ ] Measure synthesis latency.
- [ ] Measure generated audio duration.
- [ ] Compute RTF.
- [ ] Measure memory where practical.
- [ ] Exercise cancellation/preemption.
- [ ] Deny network after artifacts are ready and synthesize successfully.
- [ ] Assert normal logs do not contain the utterance sentinel/text.
- [ ] Emit machine-readable/step-summary evidence.

**Acceptance**

- [ ] One focused harness provides reproducible runtime, privacy, cancellation, and performance evidence.

## KCR-301 — Requalify Linux x86_64 real-model CPU behavior

- [ ] Record runner/CPU identity where available.
- [ ] Run the hardened real acceptance harness.
- [ ] Prove no GPU is required/detected by the accepted path.
- [ ] Pass network-denial synthesis.
- [ ] Pass cancellation/preemption.
- [ ] Pass no-utterance-in-logs assertion.
- [ ] Enforce warm RTF < 1.0.

**Acceptance**

- [ ] Linux CPU Local TTS remains practically usable and privacy-correct on the exact implementation head.

## KCR-302 — Qualify macOS arm64 real-model CPU behavior

- [ ] Record runner/hardware identity where available.
- [ ] Run real Kitten Mini synthesis.
- [ ] Prove no GPU requirement.
- [ ] Pass network-denial synthesis.
- [ ] Pass cancellation/preemption.
- [ ] Enforce warm RTF < 1.0.

**Acceptance**

- [ ] Apple Silicon CPU Local TTS is practically usable from the packaged/runtime configuration.

## KCR-303 — Qualify macOS x86_64 compile/package/runtime boundary

- [ ] Compile Local TTS runtime for x86_64 macOS.
- [ ] Package x86_64 application bundle.
- [ ] Verify native architecture and load paths.
- [ ] Run real-model CPU synthesis if CI/runtime cost is reasonable.
- [ ] If real execution is deferred, record exact reason and do not claim real-model acceptance.

**Acceptance**

- [ ] Distribution target is not silently arm64-only.

## KCR-310 — Freeze measured CPU performance/thread policy

Measure accepted Linux/macOS reference systems:

- [ ] Cold model initialization.
- [ ] First synthesis latency.
- [ ] Repeated warm latency.
- [ ] Median RTF.
- [ ] p95 RTF/latency where sample count is meaningful.
- [ ] Memory footprint where practical.
- [ ] 1-thread behavior.
- [ ] 2-thread behavior.
- [ ] Bounded-N thread behavior.
- [ ] Repeated-utterance stability.

Policy:

- [ ] Hard gate: warm RTF < 1.0.
- [ ] Target: warm median RTF <= 0.5 where reference hardware supports it.
- [ ] Target: representative short-line warm synthesis <= 1.5 s where reference hardware supports it.
- [ ] Target: cold initialization <= 3 s where reference hardware supports it.
- [ ] Freeze conservative default inference thread count from evidence.
- [ ] Do not expose user thread-count control unless evidence shows it is needed.

**Acceptance**

- [ ] Runtime default is evidence-based and documented.

## KCR-320 — Make explicit real-model CI operationally practical

- [ ] Use explicit trigger/label/workflow semantics.
- [ ] Guard execution against exact PR head SHA.
- [ ] Run independent Linux/macOS jobs in parallel where possible.
- [ ] Use content-sensitive model/runtime cache keys.
- [ ] Preserve expensive valid cache inputs when later assertions fail.
- [ ] Enforce bounded workflow timeout.
- [ ] Avoid rerunning unrelated expensive repository validation.
- [ ] Publish latency/RTF summary.

**Acceptance**

- [ ] Real-model qualification is reproducible and bounded rather than a universal serial CI bottleneck.

## KCR-330 — Human audition and Local voice default

Render the same representative Moose corpus for:

- [ ] Bella.
- [ ] Jasper.
- [ ] Luna.
- [ ] Bruno.
- [ ] Rosie.
- [ ] Hugo.
- [ ] Kiki.
- [ ] Leo.

Owner evaluates:

- [ ] Intelligibility.
- [ ] Naturalness.
- [ ] Dry/comedic fit for Talking Moose.
- [ ] Pronunciation of representative Moose corpus terms.
- [ ] Artifacts/noise.
- [ ] Speed at intended default rate.

**Acceptance**

- [ ] Owner explicitly accepts at least one Kitten voice as reasonably good.
- [ ] Freeze default Local voice from this evidence.
- [ ] Do not reuse historical Gemini voice acceptance as Kitten evidence.

**Expected human gate:** This task cannot be closed by automated tests alone. All automatable work should continue before stopping here for the owner's choice.

---

# R4 — CI, Documentation, and Tracker Reconciliation

## KCR-400 — Reconcile the original Kitten tracker against verified implementation

Update `docs/TODO(20260909-120003).md` from code + CI evidence, not assumption:

- [ ] Reconcile KTT-300 implementation/acceptance.
- [ ] Reconcile KTT-302 authoritative Local cancellation-token item.
- [ ] Reconcile KTT-305 runtime telemetry items while leaving provider/voice/install composition for KTT-600/601 until this remediation lands.
- [ ] Reconcile KTT-400 through KTT-404.
- [ ] Reconcile KTT-500 through KTT-505.
- [ ] Reconcile KTT-600 through KTT-602 only after KCR-100 through KCR-130 pass.
- [ ] Reconcile packaging/license/acceptance tasks only after their gates actually execute.
- [ ] Never mark owner audition complete without owner decision.

**Acceptance**

- [ ] Original tracker accurately describes production code and executed evidence.

## KCR-410 — Update user/developer documentation to production behavior

Update applicable docs/README with:

- [ ] Standalone TTS can be Google or Local KittenTTS.
- [ ] Gemini Live remains cloud-native live voice.
- [ ] Local TTS V1 is English-only.
- [ ] Local TTS is CPU-only/no-GPU-required.
- [ ] Model installation requires network.
- [ ] Local synthesis after installation does not require network.
- [ ] No Local<->Google fallback exists.
- [ ] Google standalone, Local standalone, and Gemini Live voices are separate settings.
- [ ] Speaking-rate/pitch capability differences are explicit.
- [ ] Local diagnostics are described without implying sensitive payload capture.
- [ ] Packaging/license/provenance notes match actual release implementation.

**Acceptance**

- [ ] Documentation describes shipped behavior, not planned behavior.

## KCR-420 — Maintain exact remediation evidence

- [ ] Record implementation commit(s).
- [ ] Record exact PR head used for qualification.
- [ ] Record ordinary CI run IDs.
- [ ] Record real-model acceptance run IDs.
- [ ] Record macOS architecture/bundle evidence.
- [ ] Record packaging/license gate evidence.
- [ ] Record measured performance summary.
- [ ] Record owner voice decision when supplied.
- [ ] Avoid recursive evidence-only CI loops.

**Acceptance**

- [ ] Every checked release gate can be traced to executed evidence on an exact SHA.

---

# R5 — Final Verification and Closeout

## KCR-500 — Run canonical local/static verification available to the agent

- [ ] `git diff --check` passes where a local checkout is used.
- [ ] Frontend formatting passes.
- [ ] Frontend lint passes.
- [ ] Frontend typecheck passes.
- [ ] Frontend tests pass.
- [ ] Rust formatting passes.
- [ ] Rust Clippy passes.
- [ ] Rust tests pass.
- [ ] Generated frontend contract is current.
- [ ] Frontend IPC shape checker passes.
- [ ] Tauri command registration checker passes.
- [ ] Packaging/license static checks pass.
- [ ] No generated dependency/model/build trees are tracked.
- [ ] Any unavailable local gate is explicitly delegated to CI rather than falsely claimed.

## KCR-501 — Pass exact final implementation-head CI

On one exact final PR head:

- [ ] Frontend quality passes.
- [ ] Rust quality passes.
- [ ] Rust tests pass.
- [ ] Generated backend contract gate passes.
- [ ] Dependency/security audit passes when path scope requires it.
- [ ] Release metadata/static gate passes when path scope requires it.
- [ ] Linux Local TTS compile/acceptance proof passes.
- [ ] macOS arm64 Local TTS compile/acceptance proof passes.
- [ ] macOS x86_64 compile/package proof passes.
- [ ] Unsigned macOS bundle smoke passes when native packaging changed.
- [ ] Canonical repository gate passes as required by repository policy.
- [ ] Real-model network-denial synthesis passes.
- [ ] Warm RTF hard gate passes.

## KCR-502 — Perform final focused source audit

Re-read end-to-end:

- [ ] TTS provider routing/no-fallback.
- [ ] settings migration and provider-specific voice ownership.
- [ ] Local model artifact catalog.
- [ ] installer cancellation/integrity/path safety.
- [ ] runtime-use verification.
- [ ] runtime load/reuse/inference/cancellation.
- [ ] async/blocking boundaries.
- [ ] PCM adaptation/playback reuse.
- [ ] voice audition.
- [ ] diagnostics Rust -> contract -> TypeScript -> UI.
- [ ] no-network Local synthesis guarantee.
- [ ] Fake synthesizer isolation.
- [ ] logs/diagnostics for utterance/credential/path leakage.
- [ ] native packaging/load paths.
- [ ] third-party license inventory.
- [ ] ordinary CI model-weight-free policy.

**Acceptance**

- [ ] No mandatory defect from the remediation specification remains unresolved.

## KCR-503 — Guarded merge and exact-master verification

- [ ] Merge only after exact final PR-head required CI passes.
- [ ] Use expected-head guard.
- [ ] Record exact merged `master` SHA.
- [ ] Verify ordinary CI on exact merged master.
- [ ] Verify required real-model acceptance on the exact implementation lineage.
- [ ] Apply any docs-only closeout through the docs-only fast path without reopening implementation evidence.
- [ ] Verify final tracker/evidence state on master.

---

# Final Remediation Checklist

- [ ] Local TTS diagnostics are coherent, typed, and privacy-safe.
- [ ] Diagnostics are generated-contract protected from Rust/TypeScript drift.
- [ ] Production UI exposes live Local TTS operational state.
- [ ] Diagnostics liveness/privacy tests are non-vacuous.
- [ ] No unapproved GPL/copyleft Local TTS component/data is shipped.
- [ ] Linux x86_64 runtime/acceptance is qualified.
- [ ] macOS arm64 runtime/bundle/real-model acceptance is qualified.
- [ ] macOS x86_64 compile/package boundary is qualified.
- [ ] Real-model synthesis remains offline after installation.
- [ ] Warm RTF hard gate passes on accepted reference systems.
- [ ] Real-model CI is exact-head guarded, cached, parallelized where practical, and bounded.
- [ ] Original KTT tracker is reconciled to code and executed evidence.
- [ ] Documentation matches production behavior.
- [ ] Owner accepts a Kitten voice and the Local default is frozen from that evidence.
- [ ] Final focused source audit finds no silent fallback, privacy, cancellation, packaging, or licensing defect.
- [ ] Guarded merge and exact post-merge master validation pass.
