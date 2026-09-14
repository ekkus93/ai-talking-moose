# AI Talking Moose — Wake Word V1 TODO

**Date:** 2026-09-14  
**Companion spec:** `docs/WAKE_WORD_V1_SPEC_2026-09-14.md`  
**Baseline:** `e638daa706e63f8bbb36b514322679c1c39e19a6` (`master`)  
**Status:** Planned implementation queue

Task IDs use the `WW-###` prefix (**Wake Word**).

---

## WW-000 — Freeze scope and implementation baseline

- [ ] Confirm implementation begins from baseline `e638daa706e63f8bbb36b514322679c1c39e19a6` or a later verified descendant.
- [ ] Record the exact implementation base SHA before the first source change.
- [ ] Keep V1 wake engine fixed to sherpa-onnx KWS unless a documented blocker requires owner review.
- [ ] Keep default wake phrase fixed to `Hey, Moose`.
- [ ] Keep wake mode disabled by default for new/default settings.
- [ ] Preserve existing manual-listen behavior when wake mode is disabled.
- [ ] Preserve existing ASR provider selection and fallback behavior.
- [ ] Preserve Local/Google/Gemini TTS separation.
- [ ] Preserve Local TTS 2-thread production policy.
- [ ] Keep barge-in and acoustic echo cancellation out of V1.
- [ ] Keep wake-phrase acoustic trimming out of V1.

**Acceptance**

- [ ] Final implementation scope is traceable to the companion spec/TODO.
- [ ] Out-of-scope discoveries are documented separately rather than silently folded into V1.

---

## WW-100 — Select and freeze sherpa-onnx KWS artifacts

- [ ] Select a sherpa-onnx English KWS model suitable for continuous CPU keyword spotting.
- [ ] Confirm required input sample rate/channels/features.
- [ ] Confirm tokenizer/keyword preparation for `Hey, Moose`.
- [ ] Confirm native runtime/API path usable from Rust without a Python sidecar.
- [ ] Pin model source revision/URL.
- [ ] Pin model byte size and SHA-256.
- [ ] Pin tokenizer/support artifact identities as applicable.
- [ ] Pin sherpa native runtime version/artifacts per supported platform.
- [ ] Record native artifact byte sizes and SHA-256 values.
- [ ] Record supported architectures.
- [ ] Record sherpa/model licenses and required notices.
- [ ] Avoid mutable `latest` production dependencies.
- [ ] Add deterministic artifact preparation/verification scripts or equivalent project-native mechanism.

**Acceptance**

- [ ] All production KWS/model/runtime inputs have immutable identities.
- [ ] Artifact mismatch fails closed.
- [ ] Provenance/licensing is reviewable from the repository.

---

## WW-110 — Add sherpa native runtime packaging

- [ ] Define platform-specific runtime layout consistent with existing native artifact policy.
- [ ] Add Linux x86_64 sherpa runtime preparation/verification.
- [ ] Add macOS arm64 sherpa runtime preparation/verification.
- [ ] Add additional release architectures only when real acceptance is available.
- [ ] Validate native library architecture before tests/package use.
- [ ] Ensure package/install paths are deterministic.
- [ ] Add third-party notices/license attribution.
- [ ] Ensure CI caches never substitute for SHA verification.
- [ ] Ensure unsupported architecture produces a sanitized explicit error.

**Acceptance**

- [ ] Supported packages can locate/load the exact pinned sherpa runtime offline.
- [ ] Wrong-architecture or corrupt runtime fails before inference.

---

## WW-200 — Implement generic `PcmRingBuffer`

- [ ] Add a sherpa-independent fixed-capacity PCM ring-buffer component.
- [ ] Configure nominal capacity for 2 seconds of canonical microphone PCM.
- [ ] Preallocate bounded storage.
- [ ] Avoid unbounded allocation/growth on the capture callback path.
- [ ] Support append/write across wraparound.
- [ ] Support chronological snapshot/replay after wraparound.
- [ ] Support clear/reset.
- [ ] Define behavior for writes larger than total capacity.
- [ ] Define thread-safety/ownership contract.
- [ ] Keep implementation independent of wake engine/model types.
- [ ] Ensure buffered PCM is never serialized/logged.

**Tests**

- [ ] Empty buffer snapshot.
- [ ] Partial-fill snapshot.
- [ ] Exact-capacity snapshot.
- [ ] Single wraparound.
- [ ] Repeated wraparound.
- [ ] Oversized write retains the correct newest samples.
- [ ] Clear removes all retained samples.
- [ ] Chronological replay order is exact.
- [ ] Capacity remains bounded under long repeated writes.

**Acceptance**

- [ ] Ring-buffer unit tests are deterministic and ordinary-CI safe.
- [ ] No raw PCM content is emitted in diagnostics on failure.

---

## WW-300 — Implement `SherpaKwsEngine`

- [ ] Add the narrow sherpa-specific KWS engine abstraction.
- [ ] Load exact pinned model/runtime/tokenizer artifacts.
- [ ] Configure keyword for canonical `HEY MOOSE` representation.
- [ ] Start with 1 inference thread.
- [ ] Configure explicit trigger threshold/boost values.
- [ ] Feed canonical streaming PCM frames.
- [ ] Emit a bounded wake-detected event without raw audio payload.
- [ ] Reset sherpa stream state correctly after recognition.
- [ ] Map initialization/inference errors to sanitized wake-word errors.
- [ ] Add explicit shutdown/cancellation behavior.
- [ ] Do not perform full transcription.
- [ ] Do not make network requests during normal KWS inference.

**Tests**

- [ ] Configuration rejects missing/corrupt artifacts.
- [ ] Keyword configuration is deterministic.
- [ ] Thread policy is observable and defaults to 1.
- [ ] Engine cancellation/shutdown is idempotent.
- [ ] Error strings do not leak credentials/raw PCM/unnecessary paths.

**Acceptance**

- [ ] A deterministic positive fixture triggers `Hey, Moose`.
- [ ] A deterministic negative fixture does not trigger.

---

## WW-310 — Implement `WakeWordRuntimeManager`

- [ ] Add one authoritative wake runtime owner.
- [ ] Define states at minimum: disabled, loading, listening, suspended, triggered/command handoff, error, stopping.
- [ ] Own sherpa engine lifecycle.
- [ ] Coordinate ring-buffer lifecycle.
- [ ] Coordinate microphone routing/ownership with existing ASR path.
- [ ] Expose start/stop/suspend/resume operations with truthful state transitions.
- [ ] Prevent duplicate concurrent starts.
- [ ] Prevent duplicate concurrent command activation.
- [ ] Ensure cancellation can interrupt load/listen/handoff where practical.
- [ ] Ensure shutdown frees native/runtime/audio resources.
- [ ] Preserve manual interaction paths after recoverable wake failures.

**Acceptance**

- [ ] There is one authoritative wake runtime lifecycle.
- [ ] No hidden fallback to full-time ASR or cloud services exists.

---

## WW-400 — Integrate authoritative microphone routing

- [ ] Audit current microphone capture ownership and ASR activation path.
- [ ] Document the chosen one-stream/routing or reopen design.
- [ ] Prevent simultaneous competing microphone capture opens.
- [ ] Normalize/resample wake PCM once where practical.
- [ ] Feed ring buffer and KWS from the same chronological audio source.
- [ ] Preserve live samples while ASR initializes after wake detection.
- [ ] Define exact ownership transfer from wake listening to command ASR.
- [ ] Define exact ownership transfer back to wake listening after interaction.
- [ ] Handle microphone disconnect/reconnect safely.
- [ ] Handle device-unavailable errors without deadlock/spin.

**Acceptance**

- [ ] Repeated wake→ASR→wake cycles do not leak or multiply microphone streams.
- [ ] Device ownership remains deterministic after cancellation/errors.

---

## WW-410 — Implement ring-buffer wake→ASR pre-roll handoff

- [ ] On wake detection, capture a chronological ring-buffer snapshot.
- [ ] Preserve sufficient pre-roll to include the complete `Hey, Moose` phrase.
- [ ] Do not acoustically trim the wake phrase in V1.
- [ ] Replay buffered PCM to command ASR in correct order.
- [ ] Continue with live PCM without a gap or duplicate range.
- [ ] Ensure immediate command words following `Hey, Moose` are retained.
- [ ] Define behavior if ASR startup fails after trigger.
- [ ] Clear stale pre-roll after interaction completes/fails.

**Tests**

- [ ] `Hey Moose, tell me the time` style fixture preserves the beginning of the command.
- [ ] Pre-roll/live boundary has no sample-order inversion.
- [ ] No intentional requirement strips `Hey Moose` from ASR transcript.
- [ ] Failed handoff returns to a valid recoverable state.

**Acceptance**

- [ ] ASR can receive wake phrase + prompt as one continuous utterance.
- [ ] First command word is not clipped in deterministic acceptance.

---

## WW-500 — Add persisted Wake Word settings

- [ ] Add persisted `wake_word_enabled` boolean.
- [ ] Default `wake_word_enabled` to `false`.
- [ ] Add engine-independent wake phrase/config representation that can support future expansion.
- [ ] Default phrase to `Hey, Moose`.
- [ ] Add sensitivity/threshold setting only if its mapping is stable enough for V1; otherwise document a fixed V1 threshold.
- [ ] Add settings validation/normalization.
- [ ] Add migration/default-on-missing behavior without forcing unrelated schema changes.
- [ ] Update generated frontend/backend settings contract as required by project conventions.
- [ ] Ensure wake setting changes do not require unrelated conversation restart unless technically necessary and documented.

**Tests**

- [ ] New/default settings have wake disabled.
- [ ] Missing field defaults wake disabled.
- [ ] Persisted enabled state remains enabled.
- [ ] Default wake phrase is `Hey, Moose`.
- [ ] Invalid wake config fails safely or normalizes deterministically.
- [ ] Existing unrelated TTS/ASR settings remain unchanged.

**Acceptance**

- [ ] Wake enable/disable persistence is deterministic across restart/load.

---

## WW-510 — Add Wake Word Settings UI

- [ ] Add a Wake Word section to Settings.
- [ ] Add `Enable wake word` toggle.
- [ ] Display default phrase `Hey, Moose`.
- [ ] If sensitivity is included, provide a user-friendly control and explain its effect.
- [ ] Clearly communicate that wake listening is local/offline.
- [ ] Clearly communicate that enabling wake mode keeps the microphone locally active for keyword detection.
- [ ] Keep arbitrary custom wake-phrase editing out of V1 unless separately qualified.
- [ ] Ensure UI reflects loading/listening/suspended/error state where appropriate.
- [ ] Ensure toggle changes update runtime without requiring application restart when feasible.

**Acceptance**

- [ ] Turning wake mode off restores existing manual behavior immediately.
- [ ] UI does not imply full-time cloud transcription.

---

## WW-600 — Integrate wake state with conversation lifecycle

- [ ] Define wake-listening activation when app is idle and wake mode enabled.
- [ ] A single wake detection starts exactly one normal command-listening interaction.
- [ ] Ignore/debounce repeated detections for the same wake event.
- [ ] Prevent a second wake activation while command ASR/Thinking is active unless existing concurrency policy explicitly allows it.
- [ ] Suspend wake activation on entry to Talking.
- [ ] Clear ring buffer on entry to Talking.
- [ ] Keep wake activation suspended for the full TTS playback interval.
- [ ] Resume wake listening after successful TTS completion.
- [ ] Resume wake listening after TTS cancellation.
- [ ] Resume wake listening after recoverable TTS failure.
- [ ] Clear ring buffer again before resuming wake listening.
- [ ] Do not implement barge-in in V1.

**Acceptance**

- [ ] Moose cannot wake itself from its own TTS playback in V1.
- [ ] No error/cancellation path leaves wake mode permanently suspended when it should be active.

---

## WW-610 — Debounce and trigger policy

- [ ] Define one wake event → one command activation invariant.
- [ ] Ignore repeated positive KWS frames after trigger.
- [ ] Reset KWS stream after accepted trigger at the correct lifecycle point.
- [ ] Add bounded cooldown only if measured behavior requires it.
- [ ] Ensure cooldown does not hide lifecycle bugs.
- [ ] Record trigger count without raw audio.
- [ ] Record last-trigger timing safely.

**Tests**

- [ ] Repeated positive frames from one phrase create one command interaction.
- [ ] A later valid wake phrase after returning to WakeListening creates a new interaction.

---

## WW-700 — Add privacy-safe diagnostics

- [ ] Add wake enabled/disabled diagnostic.
- [ ] Add runtime state diagnostic.
- [ ] Add model/runtime identity/version diagnostic.
- [ ] Add platform/architecture diagnostic.
- [ ] Add inference thread diagnostic.
- [ ] Add canonical sample rate/channel diagnostic.
- [ ] Add ring-buffer duration/capacity diagnostic.
- [ ] Add threshold/boost/sensitivity diagnostic.
- [ ] Add trigger count and last-trigger age/timestamp as appropriate.
- [ ] Add runtime initialization duration.
- [ ] Add Talking-suspension indicator.
- [ ] Add sanitized last-error field.
- [ ] Explicitly prevent raw PCM/ring-buffer contents from diagnostics.
- [ ] Audit logs for utterance/audio/credential/path leakage.

**Acceptance**

- [ ] Diagnostics are sufficient to debug lifecycle/performance without exposing audio content.

---

## WW-710 — Add performance instrumentation

- [ ] Record KWS runtime memory overhead.
- [ ] Record idle-listening CPU utilization on representative acceptance environments.
- [ ] Record inference timing/real-time behavior.
- [ ] Record wake→ASR activation latency.
- [ ] Record ring-buffer replay/startup timing.
- [ ] Record repeated wake-cycle stability.
- [ ] Keep 1-thread KWS policy unless evidence requires a change.
- [ ] If thread policy changes, record measured justification.

**Acceptance**

- [ ] A regression baseline exists before closeout.
- [ ] Wake KWS is demonstrably lighter than continuously running the project’s full ASR path.

---

## WW-800 — Build deterministic wake-word acceptance corpus

### Positive corpus

- [ ] Multiple speakers where fixtures are available/licensable.
- [ ] Different speaking volumes.
- [ ] Different microphone distances or simulated gain conditions.
- [ ] Natural `Hey Moose` pronunciation variations.
- [ ] Wake phrase immediately followed by a command.
- [ ] Background-noise variants.

### Negative / near-miss corpus

- [ ] Ordinary speech without wake phrase.
- [ ] `Moose` alone.
- [ ] `Hey Bruce`.
- [ ] `Hey Moosey`.
- [ ] Phonetically similar phrases.
- [ ] Sentences containing `moose` without full wake phrase.
- [ ] Background speech/TV/podcast-style fixtures where legally/reproducibly available.

### Harness

- [ ] Make corpus source/provenance reproducible.
- [ ] Avoid copyrighted fixture redistribution problems.
- [ ] Record threshold/boost configuration.
- [ ] Record positive detections/false rejects.
- [ ] Record negative false accepts.
- [ ] Make pass/fail criteria explicit and versioned.

**Acceptance**

- [ ] Corpus testing measures recall and false-trigger behavior, not only a single happy path.
- [ ] Claims are limited to the tested conditions.

---

## WW-810 — Add real sherpa KWS platform acceptance

- [ ] Add Linux x86_64 real pinned-model KWS inference acceptance.
- [ ] Add macOS arm64 real pinned-model KWS inference acceptance.
- [ ] Verify CPU-only inference where that is the production path.
- [ ] Verify exact artifact hashes before inference.
- [ ] Verify architecture of native runtime.
- [ ] Verify one-thread policy.
- [ ] Verify positive wake trigger.
- [ ] Verify at least one negative no-trigger fixture.
- [ ] Verify network is unnecessary during inference after artifacts are prepared.
- [ ] Capture privacy-safe diagnostics/evidence.

**Acceptance**

- [ ] All claimed supported acceptance platforms pass real KWS inference.

---

## WW-820 — Add long/repeated lifecycle stability acceptance

- [ ] Run repeated wake→ASR→Thinking→Talking→wake cycles.
- [ ] Verify no native-runtime/session growth across cycles.
- [ ] Verify no microphone-stream multiplication.
- [ ] Verify ring-buffer memory remains bounded.
- [ ] Verify wake resumes after repeated TTS completion/cancellation.
- [ ] Verify disable/enable cycles are stable.
- [ ] Verify shutdown while listening is clean.
- [ ] Verify shutdown during handoff is clean.
- [ ] Add a bounded soak/false-trigger test where practical.

**Acceptance**

- [ ] No lifecycle leak or stuck state is observed under the defined repeated-cycle acceptance.

---

## WW-900 — Documentation and user-facing behavior

- [ ] Document Wake Word architecture.
- [ ] Document default phrase `Hey, Moose`.
- [ ] Document wake disabled-by-default policy.
- [ ] Document local/offline KWS privacy boundary.
- [ ] Document that downstream ASR may receive `Hey, Moose` + prompt.
- [ ] Document that wake activation is suspended while Moose speaks.
- [ ] Document no-barge-in V1 limitation.
- [ ] Document ring-buffer memory-only behavior.
- [ ] Document model/runtime provenance and licenses.
- [ ] Document diagnostics/troubleshooting.
- [ ] Update README/user docs where appropriate.

**Acceptance**

- [ ] Documentation does not imply subjective/accuracy guarantees beyond measured evidence.
- [ ] Documentation accurately distinguishes wake KWS from full ASR.

---

## WW-950 — Final source/privacy/security audit

- [ ] Audit all wake runtime ownership paths.
- [ ] Audit microphone ownership transitions.
- [ ] Audit cancellation/shutdown.
- [ ] Audit ring-buffer lifecycle and clearing.
- [ ] Audit wake-disabled behavior for regressions.
- [ ] Audit Talking suspension/resume.
- [ ] Audit provider separation/no-fallback behavior.
- [ ] Audit artifact/runtime loading boundaries.
- [ ] Audit logs/metrics/errors for raw audio leakage.
- [ ] Audit logs/metrics/errors for credentials/secrets.
- [ ] Audit logs/metrics/errors for unnecessary raw filesystem paths.
- [ ] Confirm no network dependency is introduced into idle KWS inference.
- [ ] Confirm no full-time ASR process remains active merely for wake detection.

**Acceptance**

- [ ] No mandatory defect remains unresolved in V1 scope.

---

## WW-960 — Exact-head qualification

### Always required

- [ ] Review exact final branch diff against current `master`.
- [ ] Pass ordinary exact-head CI.
- [ ] Record exact PR head SHA.
- [ ] Record exact ordinary CI run ID.

### Wake-runtime source changes

- [ ] Formatting passes.
- [ ] Lint/Clippy passes.
- [ ] Complete relevant Rust tests pass.
- [ ] Settings/frontend contract generation checks pass.
- [ ] Deterministic wake corpus gate passes.
- [ ] Linux x86_64 real KWS acceptance passes.
- [ ] macOS arm64 real KWS acceptance passes.
- [ ] Packaging/native architecture checks pass.
- [ ] Repeated lifecycle stability acceptance passes.
- [ ] Performance evidence is recorded.

**Acceptance**

- [ ] No stale or partially qualified head is eligible for merge.

---

## WW-970 — Guarded merge and exact-master verification

- [ ] Recheck mergeability immediately before merge.
- [ ] Merge only the exact tested head SHA.
- [ ] Use an allowed guarded merge method.
- [ ] Record exact merged `master` SHA.
- [ ] Verify ordinary CI on exact merged `master`.
- [ ] Re-run exact-master KWS/native/corpus gates when required by project policy or final diff scope.
- [ ] Reconcile this TODO with final evidence without creating evidence-only recursion.

**Acceptance**

- [ ] `master` contains Wake Word V1 implementation and documentation.
- [ ] Exact merged-master required validation evidence is recorded.

---

## Final checklist

- [ ] sherpa-onnx KWS is the wake engine.
- [ ] Production model/runtime artifacts are pinned and provenance documented.
- [ ] Default wake phrase is `Hey, Moose`.
- [ ] Wake feature can be enabled/disabled in Settings.
- [ ] Wake feature defaults disabled.
- [ ] Wake-disabled behavior preserves current manual interaction behavior.
- [ ] Generic 2-second PCM ring buffer is implemented and tested.
- [ ] Wake→ASR handoff preserves wake phrase + first command words.
- [ ] No V1 acoustic trimming requirement exists.
- [ ] Exactly one command activation occurs per wake event.
- [ ] Wake listening is suspended while Moose is Talking.
- [ ] No barge-in is implemented in V1.
- [ ] Wake listening resumes after TTS completion/cancellation/failure recovery.
- [ ] Raw wake PCM remains memory-only and absent from logs.
- [ ] No silent cloud/provider fallback is introduced.
- [ ] KWS remains local/offline during idle listening.
- [ ] Diagnostics are privacy-safe.
- [ ] Deterministic positive/negative/near-miss corpus acceptance passes.
- [ ] Linux x86_64 real sherpa KWS acceptance passes.
- [ ] macOS arm64 real sherpa KWS acceptance passes.
- [ ] Repeated lifecycle stability passes.
- [ ] Performance baseline is recorded.
- [ ] Documentation is complete and truthful.
- [ ] Exact PR-head required gates pass.
- [ ] Guarded merge uses exact tested head.
- [ ] Exact merged-master required gates pass.
