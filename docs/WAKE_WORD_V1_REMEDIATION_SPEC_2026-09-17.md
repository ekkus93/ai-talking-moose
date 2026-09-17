# AI Talking Moose — Wake Word V1 Remediation Specification

**Date:** 2026-09-17  
**Status:** Proposed remediation specification  
**Baseline reviewed:** `master` at `9f5d90b4b15f16c1ef2d473e574537392c310f0b`  
**Source TODO:** `docs/WAKE_WORD_V1_TODO_2026-09-14.md`  
**Source review:** `ai-talking-moose-wake-word-v1-code-review-2026-09-17.md`  
**Companion remediation TODO:** `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`

---

## 1. Purpose

This specification defines the corrective architecture and acceptance contract required to finish Wake Word V1 after the 2026-09-17 code review.

The existing repository contains several strong Wake Word building blocks, including a bounded PCM ring buffer, fail-closed artifact verification, persisted settings, component-level runtime state machines, handoff buffering, diagnostics types, and extensive documentation. However, the reviewed `master` does not contain a production-complete Wake Word V1 feature. The implementation is fragmented across parallel `app` and `asr` wake stacks, the native sherpa runtime is not yet loadable from pinned artifacts, Wake Word is not connected to the production microphone/conversation/TTS lifecycle, the Settings UI is absent, and the original TODO remains unreconciled.

This remediation specification has four goals:

1. establish one authoritative Wake Word architecture;
2. fix correctness defects identified by the code review;
3. complete end-to-end production integration and acceptance;
4. make final completion objectively provable through source, tests, CI, platform acceptance, and reconciled documentation.

This document supersedes conflicting implementation details in earlier Wake Word design notes. Earlier documents remain useful historical evidence but must not override this specification where they disagree.

---

## 2. Defect baseline

The remediation work must explicitly close the following review findings.

### 2.1 Production integration is absent

Current Wake Word components are not wired into the application's production path. `AppState`, the authoritative microphone capture path, conversation activation, command ASR, and TTS lifecycle do not collectively execute a wake-driven interaction.

The final implementation must prove the following production path:

`persisted setting -> runtime enable -> authoritative microphone stream -> canonical PCM -> ring buffer + sherpa KWS -> one accepted wake event -> pre-roll/live handoff -> normal command ASR -> Thinking -> Talking -> wake suspension -> TTS completion/cancellation/failure -> clean wake resume`

### 2.2 Duplicate runtime and engine stacks exist

The reviewed tree contains overlapping implementations under:

- `src-tauri/src/app/wake_word_runtime.rs`
- `src-tauri/src/asr/wake_word_runtime.rs`
- `src-tauri/src/app/wake_word_engine.rs`
- `src-tauri/src/asr/wake_word_sherpa.rs`

Wake Word V1 must have exactly one authoritative runtime owner and one authoritative KWS policy/engine contract.

### 2.3 Live settings writes can persist an invalid wake phrase

Persisted startup loading validates and normalizes Wake Word settings, but the live settings write path does not apply the same Wake Word phrase validation. This creates a split-brain schema in which an invalid value may be accepted during a session and then rejected on the next launch.

All settings entry points must use the same validation/normalization function before state mutation or persistence.

### 2.4 Real sherpa inference cannot start

Production model/runtime identities are not populated. The repository intentionally fails closed, which is correct, but Wake Word V1 cannot close until exact model and runtime bytes are pinned and loadable.

### 2.5 KWS policy is inconsistent across implementations

The selected model requires encoder, decoder, joiner, `tokens.txt`, and `bpe.model`, while one of the current abstractions models a different artifact shape. Threshold/score policy also differs between parallel configs.

The remediated design must have one immutable V1 KWS configuration.

### 2.6 License/provenance records are internally inconsistent

The repository contains a hard-coded model license assertion while other repository documentation states the model license remains independently unverified. The final provenance record must contain only independently verified facts.

### 2.7 Artifact freezer scripts lack direct CI protection

The identity freezer scripts and their tests must be included in the Wake artifact CI boundary.

### 2.8 PCM can be inserted into pre-roll before canonical validation

All PCM must be validated/normalized before it is accepted into the Wake Word ring buffer or KWS path.

### 2.9 Some user documentation overstates current behavior

Documentation must distinguish implemented and qualified production behavior from planned behavior. At final closeout, user-facing docs must describe only behavior that is actually present on `master`.

### 2.10 The original TODO is stale

The original TODO contains implemented, partially implemented, and absent work but no reconciled checkboxes. Final closeout requires evidence-based reconciliation without rewriting history.

---

## 3. Scope

### 3.1 In scope

Wake Word V1 provides:

- local/offline keyword spotting with sherpa-onnx;
- fixed canonical phrase `Hey, Moose`;
- disabled-by-default persisted setting;
- local continuous microphone processing only while Wake Word is enabled and allowed by lifecycle state;
- canonical 16 kHz mono PCM for Wake Word processing;
- two-second bounded in-memory PCM pre-roll;
- one accepted wake event producing at most one normal command interaction;
- wake phrase plus immediate command audio handed to the existing command ASR path;
- wake listening suspended during command processing/Talking as defined below;
- wake listening resumed after clean interaction completion or recoverable failure;
- privacy-safe runtime diagnostics;
- deterministic corpus, platform, lifecycle, and performance acceptance;
- Linux x86_64 and macOS arm64 support only when real acceptance passes.

### 3.2 Out of scope

Wake Word V1 does not include:

- arbitrary custom wake phrases;
- user-controlled sensitivity unless a later measured acceptance process qualifies it;
- barge-in while Moose is speaking;
- acoustic echo cancellation;
- acoustic removal of `Hey, Moose` from ASR input;
- cloud keyword spotting;
- full-time cloud ASR used as a substitute for KWS;
- unsupported platform claims;
- mutable or unverified native/model artifacts.

---

## 4. Architectural authority

### 4.1 One Wake Word subsystem

Wake Word must have one authoritative module boundary. The preferred end state is a dedicated module:

```text
src-tauri/src/wake_word/
    mod.rs
    config.rs
    artifacts.rs
    engine.rs
    runtime.rs
    handoff.rs
    diagnostics.rs
```

Equivalent naming is acceptable if ownership remains equally clear. The key requirement is that production code has one canonical import path for Wake Word runtime and KWS behavior.

Existing good code may be moved or adapted rather than rewritten.

### 4.2 Ownership split

The corrected responsibility boundaries are:

- `app` owns application state, persistence, settings commands, and startup/shutdown composition.
- `audio` owns device capture and generic PCM primitives such as `PcmRingBuffer`.
- `asr` owns command transcription providers and ASR-specific processing.
- `conversation` owns normal interaction lifecycle.
- `wake_word` owns Wake Word configuration, artifact resolution, sherpa KWS, wake state machine, wake-trigger debounce, pre-roll handoff coordination, and Wake Word diagnostics.

Wake Word is not itself a full ASR provider and must not be modeled as one.

### 4.3 Duplicate implementation removal

Before final integration:

- select the strongest implementation from each duplicate pair;
- migrate all required tests;
- remove the non-authoritative manager and KWS policy;
- remove dead public exports;
- ensure no production or test code can instantiate a second competing runtime;
- add a structural test or static assertion where practical to prevent duplicate runtime ownership from reappearing.

There must be exactly one authoritative `WakeWordRuntimeManager` type in the production crate.

---

## 5. Frozen V1 configuration

The authoritative V1 configuration is:

- wake phrase: `Hey, Moose`;
- canonical KWS keyword representation source text: `HEY MOOSE`;
- sample rate: `16_000 Hz`;
- channels: `1`;
- feature dimension: `80`;
- inference threads: `1`;
- keyword score/boost: `1.0`;
- keyword threshold: `0.25`;
- pre-roll duration: `2 seconds`;
- KWS provider: local sherpa-onnx;
- sherpa runtime version: pinned immutable version from the artifact manifest;
- no network required during normal idle KWS inference after artifact preparation.

These values must be defined once and imported by settings, engine configuration, diagnostics, tests, and acceptance tooling. Parallel literal copies are discouraged unless compile-time tests prove exact equality.

---

## 6. Artifact and provenance contract

### 6.1 Production eligibility

No production model/runtime input is eligible until all of the following are recorded:

- immutable source URL or source revision;
- filename/path;
- byte size;
- SHA-256;
- platform where applicable;
- architecture where applicable;
- license/provenance;
- required third-party notice text or pointer;
- expected installation/runtime destination.

The production manifest must not contain placeholders such as zero byte counts, empty hashes, or empty required file sets.

### 6.2 Model inputs

The selected GigaSpeech KWS model contract requires the exact consumed inputs:

- encoder ONNX;
- decoder ONNX;
- joiner ONNX;
- `tokens.txt`;
- `bpe.model`;
- deterministic `HEY MOOSE` keyword representation or keyword file if required by the native API.

The manifest and Rust loader must model the same file set.

### 6.3 Native runtime inputs

For each supported platform, the repository must pin every native library actually loaded by the application.

Initial supported targets are:

- Linux x86_64;
- macOS arm64.

Additional architectures must remain unsupported until equivalent real acceptance exists.

### 6.4 Verification

Artifact preparation must:

1. fetch or consume only a frozen source;
2. verify archive size and SHA-256 before extraction/use;
3. extract safely;
4. verify every consumed file size and SHA-256;
5. verify native binary architecture before loading;
6. install to deterministic package/runtime paths;
7. fail closed on missing, corrupt, wrong-architecture, or unexpected inputs.

Cache hits never substitute for hash verification.

### 6.5 License truthfulness

Runtime and model licenses are separate facts. The repository must not infer the model license from the sherpa-onnx code license.

If a license is not independently verified, the manifest/docs must say `unverified` rather than asserting a license.

---

## 7. Settings contract

### 7.1 Authoritative settings shape

Wake Word V1 persisted settings include:

- `wake_word_enabled: bool`;
- `wake_word_phrase: String`.

Defaults:

- enabled: `false`;
- phrase: `Hey, Moose`.

### 7.2 Single validation boundary

There must be one authoritative Wake Word validation/normalization function used by:

- persisted settings load/migration;
- live `update_settings`;
- any future import/restore path;
- tests;
- runtime enable transitions where defensive revalidation is useful.

For V1:

- phrase comparison may normalize surrounding whitespace/case;
- persisted normalized value must become exactly `Hey, Moose`;
- any other phrase must fail safely before state mutation/persistence;
- invalid type/value must not corrupt previously valid settings;
- unrelated ASR/TTS settings must remain unchanged.

### 7.3 Atomicity

A failed Wake Word validation must not partially persist other mutated Wake Word fields. Settings write semantics must remain consistent with project-wide persistence policy.

### 7.4 Runtime application

Changing `wake_word_enabled` must update the runtime without requiring application restart when the application is otherwise in a state where the transition is safe.

If a temporary state requires deferral, the UI and diagnostics must show the actual state rather than claiming the transition already completed.

---

## 8. Settings UI contract

The Settings UI must include a Wake Word section with:

- `Enable wake word` toggle;
- displayed phrase `Hey, Moose`;
- clear local/offline KWS statement;
- clear disclosure that enabling Wake Word keeps the microphone locally active while listening for the phrase;
- no arbitrary phrase editor;
- no sensitivity control for V1;
- truthful runtime status where useful: loading, listening, suspended, error;
- sanitized error/help text.

Turning Wake Word off must restore existing manual behavior immediately or as soon as a currently executing bounded transition safely completes.

The UI must never imply continuous cloud transcription.

---

## 9. Authoritative audio routing

### 9.1 One microphone capture owner

Wake Word and command ASR must not independently open competing continuous microphone streams.

The application must use one authoritative capture ownership/routing strategy. The implementation may either keep one capture stream open and route PCM or explicitly transfer/reopen ownership, but the chosen implementation must be singular, documented, and tested.

### 9.2 Canonicalization before retention

Incoming device audio must be converted to canonical 16 kHz mono PCM before Wake Word retention/inference.

Canonical frame validation occurs before:

- insertion into `PcmRingBuffer`;
- feeding KWS;
- adding live samples to a wake→ASR handoff.

Invalid/non-canonical PCM must not contaminate pre-roll.

### 9.3 Common chronological source

During wake listening, the same canonical chronological PCM source must feed:

1. the bounded two-second ring buffer; and
2. the sherpa KWS engine.

This prevents ring/KWS timeline divergence.

### 9.4 Device failures

Microphone disconnect, reconnect, unavailable-device, permission, and capture errors must cause bounded state transitions. No failure path may busy-spin, multiply streams, silently switch providers, or leave an unowned stream.

---

## 10. `PcmRingBuffer` contract

The existing generic bounded ring-buffer implementation is retained unless a defect is discovered.

Required invariants remain:

- fixed capacity;
- two seconds of canonical Wake Word PCM;
- preallocated storage;
- bounded append path;
- exact chronological snapshot across wraparound;
- oversized writes retain newest samples;
- clear/reset removes retained content;
- no serialization/logging of PCM;
- ownership/threading contract documented.

No Wake Word model types may be introduced into the generic buffer.

---

## 11. Sherpa KWS engine contract

### 11.1 Engine API

The authoritative engine abstraction must support:

- initialization from verified artifact descriptors/paths;
- immutable V1 config;
- feeding canonical streaming PCM;
- bounded wake event output without audio payload;
- reset after accepted recognition;
- explicit shutdown;
- cancellation/interruption where meaningful;
- sanitized errors.

### 11.2 Native adapter

A real `NativeKwsSession`/equivalent implementation must bind to the pinned sherpa native runtime.

The adapter must configure the actual selected model file set and keyword representation. Fake sessions remain test utilities only.

### 11.3 No full transcription

The engine performs KWS only. It must not launch full transcription or make normal inference dependent on network access.

### 11.4 Error behavior

Missing/corrupt artifacts, architecture mismatch, native load failure, invalid config, and inference errors must be mapped to bounded sanitized errors. Public diagnostics must not expose credentials, raw PCM, or unnecessary absolute paths.

---

## 12. Wake runtime state machine

### 12.1 Authoritative states

At minimum:

- `Disabled`;
- `Loading`;
- `Listening`;
- `Suspended`;
- `Triggered` / command handoff;
- `Error`;
- `Stopping`.

Additional substates are acceptable if externally observable semantics remain clear.

### 12.2 Required transitions

Typical transitions:

```text
Disabled
  -> Loading
  -> Listening
  -> Triggered
  -> Suspended/CommandHandoff
  -> Suspended while ASR/Thinking/Talking
  -> Listening after interaction completion

Listening
  -> Suspended
  -> Listening

Any active state
  -> Stopping
  -> Disabled

Recoverable failure
  -> Error
  -> Disabled or Loading/Listening according to explicit recovery policy
```

### 12.3 Invariants

The manager must:

- be the only owner of the KWS session lifecycle;
- coordinate the ring buffer and handoff;
- reject duplicate concurrent starts;
- allow one accepted wake event to cause at most one command activation;
- suppress repeated positive KWS frames from the same phrase;
- allow a later phrase after reset/resume to cause a new interaction;
- be cancellable during load/listen/handoff where practical;
- release native/audio resources on stop/shutdown;
- preserve manual interaction after recoverable Wake Word errors;
- never silently fall back to full-time ASR/cloud wake detection.

---

## 13. Wake→ASR handoff

### 13.1 Trigger snapshot

On accepted wake detection:

1. atomically capture a chronological ring-buffer snapshot;
2. begin preserving subsequent live canonical PCM;
3. start/activate the existing normal command ASR path;
4. replay pre-roll then live samples exactly once and in order.

### 13.2 Continuity

The handoff must prove:

- no sample-order inversion;
- no intentional gap;
- no duplicated range;
- complete wake phrase retention where present in pre-roll;
- immediate command words following `Hey, Moose` are retained;
- V1 does not acoustically strip `Hey, Moose`.

### 13.3 Failure recovery

If command ASR fails to initialize or the interaction is cancelled:

- stale handoff/pre-roll must be cleared;
- runtime must return to a valid recoverable state;
- microphone ownership must be deterministic;
- no duplicate command activation may survive.

---

## 14. Conversation and TTS lifecycle integration

### 14.1 Idle activation

If Wake Word is enabled and application lifecycle permits listening, idle state must resolve to Wake Word listening.

### 14.2 Command activation

One accepted wake event starts exactly one normal command-listening interaction through the same downstream conversation path used by manual activation.

Wake Word must not introduce a second conversation pipeline.

### 14.3 Active interaction

A second wake activation must not start while command ASR, Thinking, or Talking is active unless a later explicit product policy changes that behavior.

### 14.4 Talking suspension

Before or on entry to Talking:

- wake activation is suspended;
- stale ring-buffer content is cleared.

Wake activation remains suspended throughout TTS playback.

### 14.5 Resume

Wake listening must resume, if still enabled, after:

- successful TTS completion;
- TTS cancellation;
- recoverable TTS failure;
- recoverable command interaction failure.

Before listening resumes, stale pre-roll is cleared and KWS state is reset as required.

No error path may leave Wake Word permanently suspended when it should be active.

### 14.6 No barge-in

Wake detections during Talking are not accepted in V1.

---

## 15. Trigger/debounce policy

The core invariant is:

> One physical/corpus wake phrase event may produce at most one command activation.

The runtime must:

- ignore repeated positive KWS frames after the event has been accepted;
- reset the KWS stream at the correct lifecycle boundary;
- avoid a time-based cooldown unless measured real behavior demonstrates it is needed;
- never use cooldown to hide state-machine defects;
- record trigger count and monotonic last-trigger age/timing without audio.

---

## 16. Diagnostics and privacy

### 16.1 Required diagnostics

Wake Word diagnostics should expose, where available:

- enabled/disabled setting;
- authoritative runtime state;
- model identity;
- runtime identity/version;
- platform/architecture;
- inference thread count;
- canonical sample rate/channels;
- ring-buffer duration/capacity;
- threshold/score;
- trigger count;
- last-trigger age/timestamp in a privacy-safe form;
- initialization duration;
- Talking suspension indicator;
- sanitized last error;
- optional measured CPU/memory/inference/handoff metrics.

### 16.2 Forbidden diagnostic content

Diagnostics/logs must never contain:

- raw PCM;
- ring-buffer sample contents;
- credentials/tokens;
- arbitrary secret-like strings;
- unnecessary absolute filesystem paths;
- full utterance audio.

Transcript logging remains governed by existing ASR/conversation policy and is not introduced by Wake Word.

---

## 17. Performance contract

Before final closeout, record representative measurements for:

- idle-listening CPU utilization;
- Wake Word runtime memory overhead;
- KWS inference timing/real-time factor or equivalent latency metric;
- wake detection → command ASR activation latency;
- pre-roll replay/startup timing;
- repeated cycle stability.

V1 remains one KWS inference thread unless measured evidence justifies a change.

The final evidence must show Wake Word KWS is materially lighter than continuously running the project's full ASR path for idle wake detection.

---

## 18. Deterministic acceptance corpus

A reproducible acceptance corpus must include:

### Positive

- multiple speakers where legally/reproducibly available;
- varied volume;
- varied/simulated microphone distance/gain;
- natural `Hey Moose` pronunciation variation;
- phrase immediately followed by command speech;
- background-noise variants.

### Negative and near-miss

- ordinary speech without wake phrase;
- `Moose` alone;
- `Hey Bruce`;
- `Hey Moosey`;
- phonetically similar phrases;
- sentences containing `moose` without the full wake phrase;
- reproducible background speech/TV/podcast-like material where licensing permits.

### Harness

The harness must record:

- fixture provenance;
- artifact/runtime identity;
- threshold/score;
- positive detection / false reject counts;
- negative false accept counts;
- explicit versioned pass/fail criteria.

No accuracy claim may exceed the tested conditions.

---

## 19. Real platform acceptance

### 19.1 Linux x86_64

Real CPU-only sherpa KWS acceptance must verify:

- exact hashes before inference;
- native architecture;
- one-thread policy;
- positive trigger;
- negative no-trigger;
- no network requirement during inference after preparation;
- privacy-safe diagnostics.

### 19.2 macOS arm64

The same real acceptance is required on macOS arm64 before claiming support.

### 19.3 Support claims

A platform must not appear in release/user documentation as supported Wake Word V1 until its real acceptance passes on the exact pinned artifacts.

---

## 20. Lifecycle stability acceptance

A bounded repeated-cycle test must exercise:

`wake -> command ASR -> Thinking -> Talking -> wake resume`

and verify:

- no native session growth;
- no microphone stream multiplication;
- bounded ring-buffer memory;
- stable disable/enable cycles;
- wake resumes after TTS success/cancellation/failure;
- clean shutdown while listening;
- clean shutdown during handoff;
- no stuck runtime state.

A bounded soak/false-trigger run should be added where practical.

---

## 21. CI contract

### 21.1 Ordinary CI

Wake source changes must continue to pass:

- formatting;
- Clippy/lint;
- complete relevant Rust tests;
- generated settings/frontend contract checks.

### 21.2 Artifact CI

Wake artifact CI must watch and test all artifact tooling, including:

- preparation script;
- verification script;
- model identity freezer;
- runtime identity freezer;
- all corresponding Python tests;
- manifest schema/consistency checks.

### 21.3 Specialized Wake gates

Final feature qualification must have explicit gates for:

- deterministic corpus;
- Linux x86_64 real KWS;
- macOS arm64 real KWS;
- native/package architecture verification;
- repeated lifecycle stability;
- performance evidence.

If some gates require specialized hardware/runners, their absence must prevent final closure rather than silently skip the requirement.

---

## 22. Documentation contract

User and developer documentation must accurately state:

- Wake Word is local/offline KWS;
- default phrase is `Hey, Moose`;
- feature is disabled by default;
- enabling it keeps the microphone locally active during wake listening;
- downstream command ASR may receive `Hey, Moose` plus the command;
- listening is suspended while Moose speaks;
- no barge-in in V1;
- pre-roll is memory-only;
- model/runtime provenance and licenses;
- supported platforms based on real acceptance;
- troubleshooting and diagnostics.

Documentation must not describe unimplemented behavior as current functionality.

---

## 23. Migration strategy from current master

The remediation should proceed in this order:

1. freeze baseline and preserve current green behavior;
2. fix the live settings validation defect;
3. consolidate duplicate Wake Word runtime/engine abstractions;
4. preserve/rehome existing good tests and `PcmRingBuffer`;
5. finish artifact identities, licenses, and package paths;
6. implement the real sherpa native session;
7. wire authoritative microphone routing;
8. wire wake→ASR handoff;
9. wire conversation/TTS lifecycle;
10. implement Settings UI/runtime control;
11. expose diagnostics;
12. add corpus, real-platform, performance, and stability acceptance;
13. reconcile docs and original TODO;
14. perform exact-head qualification and guarded merge;
15. rerun required exact-master gates.

No later step should be used to mask a failure in an earlier architectural invariant.

---

## 24. Required regression fixes from the review

The following specific regressions must receive direct tests:

### 24.1 Live invalid phrase persistence

Test through the same command/persistence path used by the application:

- begin with valid persisted settings;
- submit an invalid Wake Word phrase;
- update fails;
- no invalid value is persisted;
- existing settings remain usable;
- restart/load succeeds.

### 24.2 PCM validation ordering

Test that an invalid sample-rate frame:

- is rejected;
- is not appended to pre-roll;
- is not fed to KWS;
- does not alter the previous valid snapshot.

### 24.3 Single authoritative manager

A repository/source structural test should fail if both legacy Wake Word manager stacks are reintroduced or exported.

### 24.4 Artifact contract consistency

Tests must prove Rust config/loader, JSON manifest, artifact scripts, and docs agree on the exact consumed model files and frozen KWS constants.

### 24.5 CI freezer coverage

A CI test must demonstrate that modifying either identity freezer or its tests triggers the artifact workflow.

---

## 25. Error and recovery policy

Wake Word failures are classified as:

- configuration/persistence;
- artifact/provenance;
- native runtime load;
- audio device/capture;
- inference;
- handoff/ASR activation;
- lifecycle/cancellation;
- internal invariant violation.

Recoverable failures should preserve manual interaction.

Fail-closed means:

- do not silently switch to cloud KWS;
- do not silently run continuous full ASR;
- do not accept unverified artifacts;
- do not pretend runtime is Listening after initialization failed;
- do not persist invalid configuration.

---

## 26. Definition of done

Wake Word V1 remediation is complete only when all of the following are true:

1. exactly one production Wake Word runtime/engine stack exists;
2. live and startup settings use the same validation boundary;
3. the real pinned sherpa model/runtime is loadable offline on each claimed platform;
4. production microphone PCM feeds the same chronological ring and KWS path;
5. invalid PCM cannot contaminate pre-roll;
6. one wake event produces exactly one normal command interaction;
7. pre-roll + live audio reaches the existing ASR path continuously;
8. wake is suspended throughout Talking and resumes after completion/cancellation/recoverable failure;
9. Settings UI can enable/disable Wake Word and truthfully show state/privacy behavior;
10. diagnostics remain privacy-safe;
11. deterministic corpus acceptance passes;
12. real Linux x86_64 KWS acceptance passes;
13. real macOS arm64 KWS acceptance passes;
14. lifecycle stability acceptance passes;
15. performance evidence is recorded;
16. artifact/provenance/license records are complete and internally consistent;
17. user/developer docs describe only implemented behavior;
18. original `WAKE_WORD_V1_TODO_2026-09-14.md` is reconciled to final evidence;
19. final exact PR head passes all required gates;
20. the exact merged `master` passes required post-merge gates.

Ordinary CI alone is not sufficient evidence for Wake Word V1 completion.

---

## 27. Evidence policy

Each completed remediation TODO item should point to one or more of:

- source path and line/function;
- deterministic unit/integration test;
- CI run ID;
- artifact manifest identity;
- platform acceptance report;
- performance report;
- documentation path;
- exact commit/PR head.

A checkbox must not be marked complete based solely on an intention document.

---

## 28. Relationship to the original TODO

`docs/WAKE_WORD_V1_TODO_2026-09-14.md` remains the original feature contract and historical task record.

The remediation TODO is the executable plan for reaching that contract from the reviewed 2026-09-17 state.

At final closeout:

- completed original requirements should be checked only when supported by final evidence;
- requirements intentionally superseded by this remediation spec should be annotated rather than silently rewritten;
- obsolete duplicate implementation artifacts should be removed;
- evidence-only churn must not create an infinite closeout loop.
