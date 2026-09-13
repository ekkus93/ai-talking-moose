# AI Talking Moose — Post-KittenTTS Code Review Remediation Specification

**Date:** 2026-09-12  
**Repository:** `ekkus93/ai-talking-moose`  
**Qualified baseline:** `3e63d51c68888a23cd98ceef052007eb3c833a45` (`master`)  
**Companion tracker:** `docs/POST_KITTENTTS_CODE_REVIEW_REMEDIATION_TODO_2026-09-12.md`

## 1. Purpose

This specification converts the post-KittenTTS code review into a finite remediation program. The implementation through KTT-505 is already merged and qualified on the baseline above. The remaining work is not a rewrite of the Local TTS feature; it is the set of concrete gaps that remain before the Local KittenTTS path can be considered fully diagnosable, privacy-audited, packaged, licensed, platform-qualified, and release-ready.

The remediation MUST preserve the architecture already accepted by the project:

- standalone TTS is selected independently from text provider and ASR;
- Google Gemini TTS and Local KittenTTS are explicit standalone providers;
- Gemini Live remains a separate cloud-native live-audio path;
- `SpeechSynthesizer` plus the existing standalone invocation controller remain the authoritative synthesis boundary;
- `AudioPlayback`/CPAL remains the authoritative standalone playback path;
- Local TTS MUST NOT silently fall back to Google and Google MUST NOT silently fall back to Local;
- installed Local TTS synthesis is offline and MUST NOT transmit utterance text;
- the ordinary CI path MUST stay model-weight-free and path-scoped;
- repository distribution remains permissive: no unapproved GPL/copyleft runtime, code, or data may be introduced.

## 2. Review Findings Requiring Remediation

### 2.1 Diagnostics are fragmented instead of product-level

The runtime already exposes useful lifecycle and timing telemetry and the installer already exposes model status/error information, but there is no single production response that answers, in one request, which provider/model/voice is selected, whether the model is installed, whether the runtime is loaded/generating/failed, and what the most recent safe timing/error state is.

This creates an operational gap: troubleshooting still requires reading developer logs or correlating multiple command surfaces. The fix MUST be a composed diagnostic DTO, not a second runtime state machine.

### 2.2 Installer error classification loses fidelity at the composition boundary

The installer records typed `LocalTtsInstallErrorKind` values, while status-oriented views may collapse the failure into a generic corrupt/error state. The diagnostics response SHOULD surface the exact safe typed category for the selected model when one exists, with a conservative fallback when only a status-level failure is available.

No raw URL, filesystem path, HTTP body, credential, utterance text, or arbitrary error string is required to satisfy the diagnostic use case.

### 2.3 Diagnostics are not yet carried end-to-end through the generated contract

The repository already uses representative Rust values plus generated JSON and TypeScript shape checks to prevent IPC drift. Local TTS diagnostics MUST participate in that same mechanism. A handwritten TypeScript-only interface is insufficient because Rust-side field drift must fail CI.

The negative contract probe MUST include the new Local TTS diagnostic shape so that a deliberate field removal is proven to fail the frontend contract checker.

### 2.4 There is no production Local TTS diagnostics UI

The Settings diagnostics surface does not yet show Local TTS provider/model/voice/install/runtime state and performance metrics. A production panel is required. It MUST update while the diagnostics tab is mounted, avoid overlapping requests, clean up polling on unmount, and render a generic unavailable state rather than dumping raw IPC errors.

### 2.5 Lifecycle/privacy coverage is strong but not explicit enough for diagnostics

Runtime tests cover lazy load, cancellation, failures, invalidation, reuse, and shutdown, but the review requires one direct proof that an observer can see the lifecycle transition:

`unloaded -> loading -> ready -> generating -> ready|failed`

The diagnostics privacy test MUST be non-vacuous: inject a unique utterance sentinel, then prove the serialized diagnostics/UI payload does not contain the sentinel while positively proving that selected model, selected voice, and runtime state are present.

### 2.6 Native packaging and release-license closure remain incomplete

The runtime works in development/acceptance contexts, but release closure still needs explicit proof for Linux x86_64, macOS arm64, and macOS x86_64. Any native library must be pinned and bundle-local, with no runtime dependence on Homebrew, apt, Python, Conda, or developer-machine paths.

Release license collection must include the actual shipped Local TTS runtime, Kitten model/notice, G2P/tokenizer assets, voices, ONNX Runtime if shipped, and all other support assets. CI must fail if required evidence is absent or if an unapproved eSpeak/GPL artifact appears.

### 2.7 Real-model acceptance is not yet complete across supported targets

Linux real Kitten CPU acceptance exists and is useful, but release qualification requires a focused, reproducible acceptance harness with exact artifact verification, CPU-only proof, offline synthesis, cancellation/preemption, finite PCM/sample-rate assertions, and latency/RTF reporting.

macOS arm64 needs real-model acceptance. macOS x86_64 must at minimum compile/package with architecture and load-path proof; real-model execution may be deferred only with an explicit recorded reason.

### 2.8 Performance policy needs to be frozen from measured evidence

The runtime should not expose a user-facing thread-count control without evidence. The project needs recorded cold-load, warm latency, RTF, memory where practical, and bounded thread-count measurements. Hard release policy is warm RTF < 1.0 on accepted reference systems; target policy is median warm RTF <= 0.5, short-line warm synthesis <= 1.5 s, and cold initialization <= 3 s where hardware permits.

### 2.9 Human voice acceptance is an explicit owner gate

Automated tests cannot decide whether a voice is a good fit for Talking Moose. All eight Kitten voices must be rendered from the same representative corpus. The owner must explicitly accept at least one voice and the default must be frozen from that evidence. This is the only expected remediation item that inherently requires human judgment.

### 2.10 Tracker/documentation evidence is stale

The older `docs/TODO(20260909-120003).md` still shows several KTT-300/KTT-500 items unchecked even though the code and qualified CI evidence show them implemented. The old tracker must be reconciled without rewriting history or claiming unexecuted gates.

Documentation must describe production behavior rather than aspiration: Google vs Local standalone TTS, separate Gemini Live voice, English-only Local V1, CPU-only/no-GPU-required Local runtime, network needed for installation but not synthesis, no Local/Google fallback, separate voice settings, and provider-specific pitch/rate capability.

## 3. Required Design

### 3.1 Rust diagnostic DTO

Add one typed `LocalTtsDiagnostics` response owned by the production backend. It SHOULD contain, at minimum:

- `provider`;
- `selected_model_id`;
- `selected_voice_id`;
- install state;
- expected bytes;
- installed bytes when known;
- safe installer error category and retryability when known;
- the existing `LocalTtsRuntimeStatus` as the runtime subsection.

The response MUST NOT contain:

- utterance text;
- PCM/raw audio;
- API keys/tokens/credentials;
- full filesystem paths where model IDs are sufficient;
- arbitrary raw exception strings.

The command must capture one coherent settings snapshot before composing installer/runtime state.

### 3.2 Installer typed-error lookup

Expose a narrow selected-model lookup for the installer's recorded typed error. It must not expose mutable internals or raw error bodies. If model status reports failure but there is no typed operation error, diagnostics may conservatively report `corrupt_install` or another existing safe status category.

### 3.3 IPC contract

Add a representative `LocalTtsDiagnostics` value to the Rust contract exporter and regenerate `src/generated/backendContract.json`. TypeScript types and the native/browser-preview bridges must match the generated Rust shape exactly.

The contract negative probe must deliberately damage a Local TTS diagnostics field and assert the frontend shape checker rejects it.

### 3.4 Production diagnostics UI

Add a Local TTS section to the existing Settings diagnostics tab. It must display useful product-level state, including selected provider/model/voice, install state, runtime phase, loaded model identity when present, sample rate, inference thread count, load/synthesis/audio duration, RTF, and safe installer/runtime error categories.

Polling must be bounded and stop when the component unmounts. The panel must never display raw utterance text, audio, credentials, or full model paths because those fields must not exist in the DTO.

### 3.5 Lifecycle and privacy tests

Add deterministic test control points so tests can observe `Loading` and `Generating` rather than racing fast fake operations. A diagnostics serialization/UI test must use a unique sentinel utterance and assert its absence while asserting positive control fields are present.

### 3.6 Packaging and license gates

All supported targets must have deterministic runtime provenance. Static/release gates must verify:

- pinned runtime source/binary identity;
- intended native architecture per target;
- required bundle-local runtime files;
- no developer-machine load paths;
- no unapproved GPL/eSpeak artifacts;
- immutable HTTPS model sources;
- complete license/notices for all shipped Local TTS assets;
- ordinary builds do not embed/download model weights.

### 3.7 Real-model acceptance

The explicit Local TTS acceptance workflow must be exact-head guarded, cached with content-sensitive keys, bounded by timeout, parallelized across independent target jobs where practical, and must not rerun unrelated expensive repository validation.

After pinned artifacts are present, the synthesis phase must operate with network denied and must assert no utterance text appears in normal logs.

## 4. Non-Goals

This remediation does NOT add:

- a new playback stack;
- a second Local TTS runtime manager;
- local Gemini Live replacement;
- arbitrary user-imported TTS models;
- GPU controls;
- user-configurable inference thread count without performance evidence;
- automatic Local<->Google fallback;
- automatic model installation triggered merely by selecting Local;
- new cloud transmission of Local utterances.

## 5. Validation Strategy

Each implementation slice must be validated at the strongest available layer:

1. focused Rust/frontend unit tests and static checks;
2. generated-contract and command-registration gates;
3. path-scoped ordinary CI on the exact PR head;
4. explicit real-model CPU acceptance when runtime behavior changes;
5. macOS architecture/bundle proof when packaging changes;
6. guarded merge only after required exact-head gates pass;
7. exact merged-master CI verification.

Unavailable local toolchains must be delegated to CI and recorded as such; no unexecuted gate may be marked passed.

## 6. Completion Criteria

The remediation is complete only when all automatable tasks in the companion TODO are checked with evidence and the focused source audit finds no unresolved mandatory defect. The final project closeout additionally requires the owner voice-audition decision from KCR-330. If all automated work is complete before that decision, the loop must stop at that explicit human gate and request only the required audition choice.
