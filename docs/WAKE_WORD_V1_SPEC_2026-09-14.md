# AI Talking Moose — Wake Word V1 Specification

**Date:** 2026-09-14
**Companion TODO:** `docs/WAKE_WORD_V1_TODO_2026-09-14.md`
**Baseline:** `e638daa706e63f8bbb36b514322679c1c39e19a6` (`master`)
**Status:** Approved design for implementation

## 1. Purpose

Add a lightweight, local, always-listening wake-word feature so AI Talking Moose can begin a normal voice interaction when the user says **“Hey, Moose”** without continuously running the full ASR stack.

The wake-word subsystem is not a replacement for ASR. It is a small keyword-spotting front end whose only job is to decide when to activate the existing speech-recognition path.

## 2. Frozen V1 product decisions

The following decisions are in scope and should be treated as the V1 contract unless the owner explicitly changes them:

- Wake-word engine: **sherpa-onnx keyword spotting (KWS)**.
- Default wake phrase: **“Hey, Moose”**.
- Internal canonical keyword text may be normalized to `HEY MOOSE` or equivalent tokenizer form required by sherpa-onnx.
- Wake-word processing must remain **local/offline**.
- Wake-word mode must have a persisted **enable/disable** setting.
- Wake-word mode should be **disabled by default** for new/default settings because it changes microphone behavior to continuous local monitoring.
- Existing manual-listen behavior must remain unchanged when wake-word mode is disabled.
- While Moose is speaking, wake-word detection and microphone command activation are suspended for V1.
- V1 does **not** support barge-in while TTS is active.
- Use a **2-second PCM ring buffer** so ASR can receive pre-roll and not clip the user’s first command word.
- V1 does **not** acoustically trim the wake phrase from the buffered audio.
- It is acceptable for downstream ASR and the LLM to receive text such as `Hey Moose, tell me the weather`.
- The PCM ring buffer is memory-only and must never be persisted or logged.
- Wake-word failure must fail closed and must not silently enable cloud ASR, alter provider choice, or introduce Local↔cloud fallback.

## 3. High-level architecture

The preferred V1 pipeline is:

```text
Microphone capture
    ↓
Canonical PCM normalization/resampling
    ↓
PcmRingBuffer (2 seconds, memory-only)
    ↓
WakeWordRuntimeManager
    ↓
SherpaKwsEngine
    ↓
“Hey, Moose” detected
    ↓
Normal ASR activation
    ↓
Replay buffered PCM + continue with live PCM
    ↓
Existing intent / LLM path
    ↓
TTS response
    ↓
Wake-word listening resumes after TTS completes
```

The ring buffer must be generic audio infrastructure, not embedded inside sherpa-specific code.

## 4. Runtime ownership

### 4.1 `WakeWordRuntimeManager`

Create one authoritative owner for the wake-word runtime. It should own or coordinate:

- sherpa-onnx KWS model/runtime lifecycle;
- keyword configuration for `Hey, Moose`;
- inference thread policy;
- trigger threshold / sensitivity mapping;
- start/stop/suspend/resume state;
- diagnostics and safe error reporting;
- cancellation and shutdown;
- interaction with microphone ownership and normal ASR activation.

The runtime manager must not independently create a competing microphone capture path if the application already has an authoritative microphone owner. Prefer one capture stream with explicit routing between wake mode and command-ASR mode.

### 4.2 `PcmRingBuffer`

Implement a reusable fixed-capacity circular PCM buffer independent of sherpa-onnx.

V1 contract:

- nominal pre-roll capacity: **2 seconds**;
- use the same canonical PCM representation consumed by the wake detector where practical;
- prefer **16 kHz mono** if that is compatible with the selected sherpa KWS model and existing ASR handoff;
- preallocate fixed storage;
- avoid unbounded allocations from the real-time audio callback;
- support chronological snapshot/replay of the current buffered samples;
- continue accepting live samples during wake-trigger-to-ASR startup;
- clear on disable, shutdown, error reset, transition into Talking, and before wake listening resumes after Talking;
- never serialize or log buffered samples.

For 16 kHz, 16-bit mono, two seconds is approximately 64 KiB and is therefore negligible compared with model/runtime memory.

## 5. State-machine behavior

Wake-word mode should integrate with the existing application/conversation lifecycle instead of creating a parallel conversation state machine.

Conceptual V1 flow:

```text
Wake disabled
    Existing behavior unchanged

Wake enabled + idle
    WakeListening
        ↓ “Hey, Moose”
    WakeTriggered / CommandListening
        ↓ ASR completes
    Thinking
        ↓ response ready
    Talking
        ↓ TTS completes
    WakeListening
```

Required semantics:

- Enabling wake-word mode while idle initializes or starts the wake runtime and begins local keyword listening.
- Disabling wake-word mode immediately stops keyword activation, releases any wake-specific runtime resources that should not remain resident, and clears buffered PCM.
- A wake trigger must activate exactly one command-listening interaction.
- Duplicate detections during the same interaction must be ignored/debounced.
- During `Thinking`, wake-word activation should not start another simultaneous request unless the existing application explicitly supports that concurrency.
- During `Talking`, microphone wake activation is suspended.
- On TTS completion or cancellation, stale PCM is cleared before wake listening resumes.
- On application shutdown, wake runtime and microphone routing must stop cleanly.

## 6. Wake phrase handling

### 6.1 Default phrase

User-visible default:

> Hey, Moose

Internal sherpa keyword representation may remove punctuation/case according to tokenizer requirements.

### 6.2 V1 phrase editing

The settings/data model should not prevent future custom wake phrases, but arbitrary phrase editing is **not required for V1** unless implementation proves trivial and safe.

For V1 it is acceptable to expose the phrase as fixed/read-only UI text while persisting an engine-independent wake-word configuration structure that can evolve later.

### 6.3 Downstream ASR text

Do not attempt sample-perfect acoustic trimming of the wake phrase in V1.

On trigger:

1. retain/replay sufficient pre-roll to include the entire wake phrase;
2. preserve any command words spoken immediately after it;
3. continue seamlessly with live PCM;
4. allow ASR to produce text such as `Hey Moose, what time is it?`;
5. allow the LLM/intent path to receive that text unchanged unless a later text-normalization feature is intentionally added.

This favors reliability over cosmetically cleaner transcripts.

## 7. Settings and UX

Add a Wake Word section in Settings.

Required V1 setting:

- **Enable wake word** — persisted boolean, default `false`.

Recommended V1 controls if they can be implemented without destabilizing the initial release:

- Wake phrase display: `Hey, Moose`.
- Sensitivity: a user-friendly control mapped to sherpa KWS threshold/boost parameters.
- Runtime status/diagnostics: disabled, loading, listening, suspended, triggered, error.

Settings behavior must distinguish wake-word configuration from:

- Google standalone TTS voice;
- Local KittenTTS voice;
- Gemini Live voice;
- ASR provider/model choice.

Changing wake-word settings must not silently change any speech provider.

## 8. Sherpa-onnx integration

### 8.1 Model/runtime selection

Select a sherpa-onnx KWS model suitable for English keyword spotting and continuous CPU inference.

Before implementation is considered complete:

- freeze the exact model artifact identity;
- freeze runtime/native artifact identities for each supported platform;
- record source URL/revision, byte size, SHA-256, architecture, and license/provenance;
- document tokenizer/keyword-preparation requirements;
- document any model-specific sample rate and feature-extractor expectations.

Do not rely on mutable “latest” artifact URLs for production acceptance.

### 8.2 Native runtime packaging

Integrate sherpa-onnx consistently with the existing Rust/Tauri/native-runtime packaging approach.

Requirements:

- no Python sidecar for V1;
- no runtime download from arbitrary mutable locations after installation unless the existing project policy explicitly permits it;
- architecture validation for packaged native libraries;
- clear third-party notices and license attribution;
- deterministic CI preparation where native artifacts are not checked directly into the repository.

### 8.3 Thread policy

Start with **1 KWS inference thread** unless measured behavior proves insufficient.

The thread count must be explicit, observable in diagnostics, and separately tunable from Local TTS inference threads.

## 9. Audio ownership and handoff

Microphone ownership is a high-risk integration point.

The implementation must define one authoritative capture/routing policy that prevents:

- two competing capture streams opening the same device;
- wake KWS and normal ASR reading divergent copies with inconsistent timing;
- device lock/reopen races at the wake→ASR transition;
- missing the start of the command while ASR initializes.

Preferred behavior:

- a continuous normalized PCM source feeds the ring buffer and KWS while wake listening is active;
- when triggered, KWS activation is suspended and the command-ASR consumer receives a chronological pre-roll snapshot followed by live samples;
- once the command interaction ends, command-ASR ownership is released and wake listening is re-established cleanly.

If the existing ASR architecture requires device reopen instead of stream routing, the ring buffer must cover the measured transition latency and acceptance tests must prove first-word retention.

## 10. Talking-state microphone suppression

V1 deliberately disables wake activation while Moose is speaking.

Required behavior:

- on entry to Talking, suspend wake detection and prevent new command activation;
- clear the ring buffer so playback audio cannot become future pre-roll;
- do not attempt acoustic echo cancellation in V1;
- do not attempt barge-in in V1;
- on successful TTS completion, cancellation, or recoverable TTS failure, clear the buffer again and resume wake listening if enabled;
- ensure no error path leaves wake listening permanently suspended.

## 11. Trigger semantics and debouncing

A wake detector can emit repeated positive frames around one spoken phrase. V1 needs explicit debouncing.

Requirements:

- one spoken wake phrase should produce at most one command activation;
- ignore additional KWS detections while a command interaction is in progress;
- reset sherpa stream state appropriately after a recognized keyword;
- apply a bounded cooldown only where needed; do not use cooldown as a substitute for correct state ownership;
- expose trigger count and last-trigger timing in diagnostics without logging raw audio.

## 12. Error handling

Wake-word errors must be isolated and safe.

Examples:

- model/runtime artifact missing or corrupt;
- unsupported architecture;
- microphone unavailable;
- device disconnected;
- KWS initialization failure;
- inference failure;
- ASR handoff failure;
- cancellation during wake→ASR transition.

Required policy:

- report a sanitized wake-word-specific error;
- disable or suspend wake functionality as appropriate;
- preserve manual interaction paths where possible;
- do not silently switch to cloud services;
- do not leak filesystem paths, raw audio, transcripts beyond existing allowed logging policy, credentials, or artifact secrets.

## 13. Privacy

Wake-word mode changes microphone behavior and therefore needs an explicit privacy boundary.

When enabled and idle:

- microphone audio is processed locally for KWS;
- the ring buffer remains in memory only;
- audio is continuously overwritten/discarded when not needed;
- no wake-listening audio is written to disk;
- no wake-listening audio is sent to the network merely for KWS;
- raw PCM must not be included in logs, metrics, crash reports, or diagnostics.

After wake detection, the existing ASR provider policy applies. If the user has selected a network/cloud ASR provider, command audio may then be processed according to that provider’s existing behavior; the wake detector itself remains local.

## 14. Diagnostics and observability

Add bounded diagnostics sufficient to troubleshoot the feature without exposing content.

Useful fields:

- feature enabled/disabled;
- runtime state;
- model/runtime identity/version;
- platform/architecture;
- KWS inference thread count;
- canonical audio sample rate/channels;
- ring-buffer configured duration/capacity;
- sensitivity/threshold/boost values;
- trigger count;
- last trigger timestamp/relative age;
- runtime load/init duration;
- sanitized last error;
- whether wake listening is currently suspended because Moose is Talking.

Do not log:

- raw PCM;
- ring-buffer contents;
- full captured utterances;
- credentials;
- secret/token material;
- unnecessary absolute artifact paths.

## 15. Performance targets

KWS must be materially lighter than continuously running full ASR.

Qualification should record:

- idle KWS CPU usage;
- wake runtime RSS/memory overhead;
- inference latency / real-time behavior;
- wake→ASR activation latency;
- ring-buffer replay behavior;
- repeated long-duration stability.

V1 should begin with 1 inference thread and only increase it if measured recall/latency requires it.

Do not freeze a universal CPU-percentage threshold until measured on representative CI/test hardware, but record the measurements and establish a regression baseline before closeout.

## 16. Accuracy and false-trigger qualification

A passing implementation is not merely one that recognizes one clean `Hey, Moose` sample.

Build or curate a deterministic acceptance corpus covering:

### Positive cases

- multiple speakers;
- different speaking volumes;
- different microphone distances;
- natural `Hey Moose` and `Hey, Moose` phrasing;
- `Hey Moose` immediately followed by a command;
- modest background noise.

### Negative / near-miss cases

- ordinary speech without the wake phrase;
- `Moose` alone;
- `Hey Bruce`;
- `Hey Moosey`;
- phonetically similar phrases;
- conversation containing the word `moose` without the full wake phrase;
- TV/podcast/music/background speech where practical.

Record:

- positive recall;
- false accepts;
- false rejects;
- threshold/sensitivity configuration;
- repeated-run stability.

Do not claim production-quality far-field performance unless it has actually been measured.

## 17. Testing strategy

### 17.1 Pure/unit tests

Cover at minimum:

- `PcmRingBuffer` wraparound;
- chronological snapshot order;
- exact capacity behavior;
- clear/reset behavior;
- no unbounded growth;
- settings default/persistence;
- wake state transitions;
- debounce behavior;
- Talking suspension/resume;
- wake-disabled behavior;
- sanitized error mapping.

### 17.2 Integration tests

Cover:

- deterministic audio → KWS trigger;
- non-trigger near-miss audio;
- wake trigger activates exactly one ASR interaction;
- buffered pre-roll reaches command ASR in order;
- immediate post-wake command words are not clipped;
- wake phrase may remain in downstream transcript by design;
- disabling wake mode stops activation;
- Talking suspends wake activation;
- TTS completion/cancellation resumes wake listening;
- shutdown/cancellation does not leak capture/runtime ownership.

### 17.3 Real runtime acceptance

Where supported, run exact-artifact real sherpa KWS inference on the same target classes expected for release, at minimum Linux x86_64 and macOS arm64 if those are current supported desktop acceptance platforms.

If Windows is a release target, add corresponding native/package acceptance before declaring universal support.

## 18. CI strategy

Avoid making every ordinary source edit pay the full expensive KWS real-runtime matrix if path scoping can preserve confidence.

Recommended layers:

1. ordinary CI: formatting/lint/unit/integration tests with deterministic fixtures;
2. KWS native acceptance: exact pinned runtime/model on supported platforms;
3. wake audio corpus acceptance: positives + near misses;
4. packaging/native architecture checks;
5. optional/manual long-duration false-trigger soak where CI cost is too high for every PR.

All consequential source/runtime changes must be qualified on the exact PR head before guarded merge.

## 19. Compatibility and non-goals

V1 must preserve:

- existing manual listening when wake is disabled;
- existing ASR provider selection;
- existing Local/Google/Gemini TTS ownership and fallback policy;
- Local TTS thread policy;
- existing privacy/logging restrictions.

Explicit V1 non-goals:

- full-time ASR used as wake detection;
- barge-in while Moose speaks;
- acoustic echo cancellation;
- arbitrary user-trained wake phrases;
- acoustic trimming/removal of `Hey, Moose` before ASR;
- cloud-based wake-word detection;
- silent provider fallback;
- persistence of wake-listening audio.

## 20. Completion definition

Wake Word V1 is complete only when:

- sherpa-onnx KWS artifacts/runtime are pinned and provenance documented;
- `Hey, Moose` triggers deterministic local wake detection;
- wake mode can be enabled/disabled from Settings and persists correctly;
- wake disabled preserves current application behavior;
- the 2-second generic PCM ring buffer is implemented and tested;
- wake→ASR handoff preserves the wake phrase and first command words without clipping;
- only one command activation occurs per wake event;
- wake activation is suspended while Moose is Talking;
- wake listening reliably resumes after TTS completion/cancellation;
- raw wake-listening PCM remains memory-only and absent from logs;
- native/runtime packaging is deterministic and licensed correctly;
- unit/integration/native/corpus acceptance passes on the exact merge candidate;
- diagnostics are sufficient to debug state/performance without exposing audio content;
- final TODO/evidence is reconciled and merged to `master`.
