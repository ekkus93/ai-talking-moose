# AI Talking Moose — Wake Word V1 Remediation TODO

**Date:** 2026-09-17
**Status:** Implementation queue
**Baseline:** `master` at `9f5d90b4b15f16c1ef2d473e574537392c310f0b`
**Specification:** `docs/WAKE_WORD_V1_REMEDIATION_SPEC_2026-09-17.md`
**Original TODO:** `docs/WAKE_WORD_V1_TODO_2026-09-14.md`
**Review baseline:** `ai-talking-moose-wake-word-v1-code-review-2026-09-17.md`

Task IDs use the `WWR-###` prefix (**Wake Word Remediation**).

The ordering is intentional. Do not implement later integration around unresolved duplicate runtime ownership or unverified artifact identities.

---

## WWR-000 — Freeze remediation baseline and preserve known-good behavior

- [x] Confirm current implementation base is `9f5d90b4b15f16c1ef2d473e574537392c310f0b` or a later verified descendant.
- [x] Record exact starting master SHA and ordinary CI run.
- [x] Preserve manual-listen behavior while Wake Word remains disabled.
- [x] Preserve ASR provider selection/fallback policy.
- [x] Preserve Local/Google/Gemini TTS separation.
- [x] Preserve Local TTS production thread policy.
- [x] Preserve no-barge-in V1 policy.
- [x] Preserve no acoustic wake-phrase trimming V1 policy.
- [x] Preserve `PcmRingBuffer` behavior and tests.
- [x] Preserve fail-closed artifact behavior until real identities are populated.
- [x] Add/update a remediation evidence file recording baseline and scope.

**Acceptance**

- [x] No remediation change silently changes non-Wake ASR/TTS/manual behavior.
- [x] Baseline is reproducible from repository evidence.

**Evidence:** `docs/evidence/WWR-000_REMEDIATION_BASELINE_2026-09-17.md`

---

## WWR-010 — Fix live Wake Word settings validation defect

- [x] Identify the one canonical Wake Word settings validation/normalization function.
- [x] Route persisted-load validation through that function.
- [x] Route live `update_settings` validation through the same function.
- [x] Ensure `wake_word_phrase` normalizes case/whitespace to exact `Hey, Moose`.
- [x] Reject any other Wake Word phrase before state mutation or persistence.
- [x] Ensure invalid type/value does not partially persist Wake Word state.
- [x] Preserve unrelated ASR/TTS fields on successful Wake Word updates.
- [x] Preserve previous valid persisted settings on failed update.

**Tests**

- [x] Live update accepts canonical `Hey, Moose`.
- [x] Live update normalizes `  hey, moose  `.
- [x] Live update rejects `Hey Bruce`.
- [x] Rejected update does not persist invalid JSON/settings.
- [x] Restart/load after rejected update still succeeds.
- [x] Missing Wake Word fields still migrate to disabled + canonical phrase.
- [x] Existing unrelated ASR/TTS settings remain unchanged.

**Acceptance**

- [x] There is no settings path that can persist a value startup validation later rejects.
- [x] Command-level regression test covers the previously identified defect.

**Evidence:** implementation merged in PR #169 at `cdf5ec65cf1a2d0d726c6e96459ba4264d7aeadc`; exact PR-head ordinary CI `35254726268` passed on `a85158d62084fd8506004b6d486530af32bf37a4`. Canonical validation is `WakeWordSettings::from_app_settings_fields`; both persisted-load projection and live `update_settings` use it, with live normalization/rejection regression tests in `src-tauri/src/commands/settings.rs`.

---

## WWR-020 — Consolidate duplicate Wake Word module architecture

- [x] Choose a single canonical module boundary, preferably `src-tauri/src/wake_word/`.
- [x] Inventory all functionality in `app/wake_word_*`.
- [x] Inventory all functionality in `asr/wake_word_*`.
- [x] Select one authoritative KWS config representation.
- [x] Select one authoritative KWS engine/session abstraction.
- [x] Select one authoritative `WakeWordRuntimeManager`.
- [x] Select one authoritative handoff implementation.
- [x] Select one authoritative diagnostics representation.
- [x] Move/rehome reusable tests to the canonical module.
- [x] Remove duplicate/obsolete public exports.
- [x] Remove duplicate/obsolete source files after migration.
- [x] Ensure production code cannot instantiate two independent Wake Word managers.
- [x] Ensure Wake Word remains distinct from command ASR provider implementations.
- [x] Add structural/source regression coverage against duplicate manager reintroduction.

**Acceptance**

- [x] Exactly one production `WakeWordRuntimeManager` exists.
- [x] Exactly one production V1 KWS config/engine policy exists.
- [x] No tests depend on the removed duplicate implementation.
- [x] Ordinary CI passes after consolidation.

**Evidence:** `docs/evidence/WWR-020_WAKE_WORD_ARCHITECTURE_CONSOLIDATION_2026-09-17.md`; implementation merged in PR #172 at `70b813b757789cf993ff4c1e8d2d4c9825f6313b`. Exact PR-head ordinary CI `35268110856` passed on `39c95ddba9628cdd531c08f3bff4451fc6bad785`.

---

## WWR-030 — Freeze one authoritative V1 KWS policy

- [x] Define canonical constants in one source location:
  - [x] sample rate `16_000 Hz`;
  - [x] channels `1`;
  - [x] feature dimension `80`;
  - [x] inference threads `1`;
  - [x] keyword source `HEY MOOSE`;
  - [x] score `1.0`;
  - [x] threshold `0.25`;
  - [x] pre-roll `2 seconds`.
- [x] Make settings/docs/diagnostics/tests consume or verify the same constants.
- [x] Remove permissive parallel config validation that merely accepts any positive threshold/score.
- [x] Make config drift fail deterministically.
- [x] Ensure model artifact schema matches engine config schema.

**Tests**

- [x] Any threshold other than `0.25` fails V1 config validation.
- [x] Any score other than `1.0` fails V1 config validation.
- [x] Any inference thread count other than `1` fails V1 validation.
- [x] Non-16-kHz or non-mono KWS input fails before retention/inference.
- [x] Artifact contract test verifies encoder/decoder/joiner/tokens/BPE expectations.

**Acceptance**

- [x] No production module can configure a contradictory Wake Word V1 policy.

**Evidence:** `docs/evidence/WWR-030_CANONICAL_KWS_POLICY_2026-09-17.md`; implementation merged in PR #174 at `4a9c3d585bf1458418ab87aa3d5b818add845505`. Exact PR-head ordinary CI `35281663768` and KittenTTS CPU acceptance `35281663889` passed on `93ea1f9cbd41178975293abf0ce1c0f5f9665153`.

---

## WWR-100 — Complete model artifact identities and provenance

- [x] Obtain the exact selected GigaSpeech KWS model archive through an independently verifiable source.
- [x] Run deterministic identity freezing.
- [x] Record model archive byte size.
- [x] Record model archive SHA-256.
- [x] Record exact encoder filename/path, byte size, SHA-256.
- [x] Record exact decoder filename/path, byte size, SHA-256.
- [x] Record exact joiner filename/path, byte size, SHA-256.
- [x] Record exact `tokens.txt` byte size and SHA-256.
- [x] Record exact `bpe.model` byte size and SHA-256.
- [x] Deterministically generate/freeze the exact `HEY MOOSE` keyword representation required by the native API.
- [x] Record keyword artifact bytes/hash if represented as a file.
- [x] Populate `wake-word-artifacts.json` with production model entries.
- [x] Populate/replace Rust manifest placeholders with real identities or generate Rust data from the authoritative manifest.
- [x] Ensure Rust and JSON manifests cannot drift silently.
- [x] Remove zero-byte/empty-hash production placeholders.

**Provenance/license**

- [x] Independently verify model provenance.
- [x] Independently verify model license.
- [x] Remove any unverified hard-coded license assertion.
- [x] Add required model attribution/notices.
- [x] Make docs distinguish runtime license from model license.

**Acceptance**

- [x] Every consumed production model/tokenizer/keyword input has immutable identity.
- [x] Repository provenance/license information is internally consistent.
- [x] Any byte mismatch fails closed.


**Evidence:** `docs/evidence/WWR-100_MODEL_IDENTITY_2026-09-17.md`; implementation merged in PR #178 at `2319947000fd629f0f1308389fe40dd074ac1198`. Exact final PR head `18d5c798c06a38c17c93418e2cec0a297d4305ca` passed ordinary CI `35289861261`, Wake Artifact Verification `35289861279`, and Wake Word model identity freeze `35289861291`.
---

## WWR-110 — Complete sherpa native runtime identities and packaging

- [x] Verify the frozen sherpa runtime version used for V1.
- [x] Obtain exact Linux x86_64 runtime archive.
- [x] Freeze Linux archive byte size/SHA-256.
- [x] Freeze all actually loaded Linux native library byte sizes/SHA-256 values.
- [x] Verify ELF x86_64 architecture before use.
- [x] Obtain exact macOS arm64 runtime archive.
- [x] Freeze macOS archive byte size/SHA-256.
- [x] Freeze all actually loaded macOS native library byte sizes/SHA-256 values.
- [x] Verify Mach-O arm64 architecture before use.
- [x] Define deterministic installed runtime directory layout.
- [x] Define deterministic package/bundle locations.
- [x] Ensure cached artifacts still undergo hash verification.
- [x] Produce sanitized unsupported-platform errors.
- [x] Add complete sherpa runtime Apache-2.0 attribution/notices.
- [x] Do not add additional supported architectures without real acceptance.

**Tests**

- [x] Wrong Linux architecture fails.
- [x] Wrong macOS architecture fails.
- [x] Corrupt archive fails.
- [x] Corrupt consumed library fails.
- [x] Missing required library fails.
- [x] Unsupported platform fails with sanitized explicit error.
- [x] Offline prepared runtime can be located deterministically.

**Acceptance**

- [x] Claimed packages can locate the exact pinned runtime offline.
- [x] No wrong-architecture/corrupt runtime can reach inference.


**Evidence:** `docs/evidence/WWR-110_SHERPA_RUNTIME_IDENTITY_2026-09-17.md`; implementation on PR #184. Frozen identities were independently reproduced by runtime-identity workflow `35294590820` on master `4955d1f1872c310f872219b812f5d95a3887e97e`. Exact PR-head `50c562f6421b73e11f7049fbe87764048730f9be` passed ordinary CI `35296408130`, Wake Artifact Verification `35296408236`, Wake Word model identity freeze `35296408238`, and Wake Word runtime identity freeze `35296408308`. Runtime preparation re-verifies cached archives and installed libraries, rejects unsafe archive members, enforces ELF x86_64 / Mach-O arm64 architecture, supports deterministic offline preparation from an explicit archive, and reports unsupported platforms with sanitized errors.
---

## WWR-120 — Expand artifact CI coverage

- [x] Update Wake artifact workflow path filters to include:
  - [x] model identity freezer;
  - [x] runtime identity freezer;
  - [x] their test files;
  - [x] production manifest(s);
  - [x] preparation script;
  - [x] verification script.
- [x] Run all four Wake artifact Python test suites in CI.
- [x] Add manifest consistency/schema validation.
- [x] Add a check that production-required identities are non-placeholder when production mode is enabled.
- [x] Add test coverage for safe extraction/path traversal rejection.
- [x] Add test coverage proving cache cannot bypass identity verification.

**Acceptance**

- [x] A change to either freezer script triggers and exercises the Wake artifact workflow.
- [x] Artifact tooling regression cannot merge behind a skipped path filter.


**Evidence:** `docs/evidence/WWR-120_ARTIFACT_CI_COVERAGE_2026-09-17.md`; implementation head `761f94352d3ca7b385a6a3eca1a7f8ec61197288` passed ordinary CI `35298008399`, Wake Artifact Verification `35298008347`, Wake Word model identity freeze `35298008322`, and Wake Word runtime identity freeze `35298008340`. The artifact workflow runs the four pre-existing Wake artifact suites plus focused current-freezer/runtime tests, validates the production manifest and non-placeholder identities, rejects malicious extraction paths, and proves cached runtime state cannot bypass identity verification.
---

## WWR-200 — Implement the real native sherpa KWS session

- [x] Implement the real native `NativeKwsSession`/equivalent adapter.
- [x] Load the exact verified sherpa native runtime.
- [x] Load exact verified encoder.
- [x] Load exact verified decoder.
- [x] Load exact verified joiner.
- [x] Load exact verified `tokens.txt`.
- [x] Load exact verified `bpe.model`.
- [x] Configure deterministic `HEY MOOSE` keyword representation.
- [x] Configure one inference thread.
- [x] Configure score `1.0`.
- [x] Configure threshold `0.25`.
- [x] Feed streaming 16-kHz mono PCM.
- [x] Emit bounded Wake Word detection event with no raw audio.
- [x] Reset stream state after accepted recognition.
- [x] Implement idempotent shutdown.
- [x] Add cancellation/interrupt behavior where native API permits.
- [x] Map native errors to sanitized Wake Word errors.
- [x] Ensure normal inference has no network dependency.
- [x] Ensure engine never performs full transcription.

**Tests**

- [x] Missing verified artifact fails before native session creation.
- [x] Corrupt verified artifact fails before native session creation.
- [x] Wrong architecture fails before native load.
- [x] Native load failure is sanitized.
- [x] Thread/threshold/score policy is observable.
- [x] Fake-session tests remain for deterministic unit coverage.
- [ ] Real positive fixture triggers.
- [ ] Real negative fixture does not trigger.

**Acceptance**

- [ ] Production engine can execute real pinned sherpa KWS offline.
- [ ] Fake session is not used by production composition.

---

**Evidence (implementation):** `src-tauri/src/app/wake_word_engine.rs` now contains the real verified native sherpa C-API session, fail-closed model/runtime identity and architecture checks, frozen V1 policy, bounded detection, reset/shutdown behavior, and deterministic fake-session/unit coverage. Real positive/negative fixture acceptance and production composition remain open and must not be inferred from component tests.

---

## WWR-210 — Fix PCM validation ordering

- [x] Validate/canonicalize PCM before ring-buffer append.
- [x] Validate/canonicalize PCM before KWS feed.
- [x] Validate/canonicalize PCM before live handoff append.
- [x] Ensure invalid frames do not mutate Wake Word retained state.
- [x] Ensure prior valid pre-roll remains unchanged after invalid input.

**Tests**

- [x] 48-kHz frame is rejected before append.
- [x] Empty frame is rejected before append.
- [x] Invalid frame is not fed to KWS.
- [x] Invalid frame does not alter ring snapshot.
- [x] Valid canonical frame is appended/fed exactly once.

**Acceptance**

- [x] Non-canonical PCM cannot contaminate pre-roll.

**Evidence:** implementation completed across PR #187 and PR #189. PR #187 merged at `5be1539d0ba03b9a8e072a035752d19df3b872c1` with exact PR-head ordinary CI `35303924318`, adding validation before live wake→ASR handoff retention in `src-tauri/src/asr/wake_word_handoff.rs`. PR #189 merged at `c8b43524b01719454543517ccd21e2fe9a457504` with exact PR-head ordinary CI `35317569091` on `1d9a4bd891aff735caad2aee96f795e79d3e39d6`, adding validation before ring-buffer retention in `WakeWordRuntimeManager::append_listening_pcm_frame` and regression coverage proving invalid 48-kHz/empty input leaves retained pre-roll unchanged; it also adds KWS-feed coverage proving invalid frames fail before engine state mutation.

---

## WWR-300 — Integrate one authoritative microphone routing path

- [x] Re-audit current `AudioCapture` ownership on the post-consolidation source.
- [x] Select and document the final one-stream/routing strategy.
- [ ] Wire Wake Word manager into production application state/composition.
- [x] Ensure Wake Word does not open a competing continuous microphone stream.
- [x] Canonicalize/resample microphone PCM once where practical.
- [x] Feed ring buffer and KWS from the same chronological canonical stream.
- [x] Preserve live samples immediately after trigger while command ASR initializes.
- [x] Implement deterministic ownership transfer to command ASR.
- [x] Implement deterministic ownership return to wake listening.
- [x] Handle device disconnect.
- [x] Handle reconnect.
- [x] Handle unavailable device.
- [x] Handle permission/capture error without spin/deadlock.
- [x] Ensure cancellation does not orphan or multiply streams.

**Tests**

- [x] Repeated wake→ASR→wake cycles do not increase capture-stream count.
- [x] Wake disable tears down/suspends capture according to final policy.
- [x] Manual listen still works with Wake Word disabled.
- [x] Device error leaves deterministic ownership.
- [x] Cancellation leaves deterministic ownership.

**Acceptance**

- [ ] Exactly one authoritative microphone ownership model exists in production.
- [x] No simultaneous competing capture opens occur.

---

**Evidence (audit/strategy):** `docs/evidence/WWR-300_AUDIO_CAPTURE_OWNERSHIP_AUDIT_2026-09-19.md`; audit merged in PR #223 at `f182f688d94e384780fc28024ca3ae3f5680829f`. The remaining WWR-300 items are production wiring and acceptance, not documentation.

**Evidence (routing/composition):** `docs/evidence/WWR-300_APP_STATE_CAPTURE_COMPOSITION_2026-09-22.md`; implementation merged in PR #370 at `ae2fb1e50a97768b697052c557d65609e89921bd`. Exact merged-master ordinary CI `35743357172`, Wake Word source security audit `35743357280`, and Wake Word lifecycle stability `35743357095` passed. The shared `AuthoritativeWakeCaptureOwner` uses the existing `AppState::audio_capture`, deterministic transfer/return/cancellation methods stop or resume that same owner, and `CanonicalWakePcmRouter` validates one canonical 16-kHz mono timeline before retaining the same chunks for pre-roll and feeding KWS. Lifecycle start wiring, real device disconnect/reconnect acceptance, and repeated wake→ASR→wake soak remain open.

**Evidence (capture recovery/cycles):** `docs/evidence/WWR-300_CAPTURE_RECOVERY_CYCLES_2026-09-22.md`. Post-trigger canonical PCM is retained in the single handoff path while command ASR starts. Runtime capture inactivity is detected even when the PCM queue remains open, fails Wake closed, and is recoverable only through the serialized restart path on the same `AppState::audio_capture`. A 100-cycle wake→command→wake regression verifies repeated ownership transfer/return reuses that exact shared capture owner. Real hardware disconnect/reconnect acceptance remains a later integration/acceptance concern; production lifecycle startup wiring is still open.

---

## WWR-310 — Complete production wake→ASR pre-roll/live handoff

- [x] On accepted trigger, snapshot ring chronologically.
- [x] Start preserving subsequent live canonical samples.
- [ ] Activate the existing normal command ASR path exactly once.
- [x] Replay pre-roll into command ASR.
- [x] Continue with live PCM.
- [x] Prevent gaps at snapshot/live boundary.
- [x] Prevent duplicated sample ranges.
- [x] Preserve complete `Hey, Moose` phrase when present in the two-second window.
- [x] Preserve immediate first command word.
- [x] Do not acoustically trim Wake Word in V1.
- [x] Clear stale handoff/pre-roll after success.
- [x] Clear stale handoff/pre-roll after ASR startup failure.
- [x] Clear stale handoff/pre-roll after cancellation.
- [x] Return runtime to a valid recoverable state after handoff failure.

**Tests**

- [x] Synthetic exact sample-order boundary test.
- [ ] Real/reproducible `Hey Moose, tell me the time` audio acceptance.
- [ ] First command word is present in downstream ASR acceptance.
- [x] No duplicate range is observed.
- [x] No inversion is observed.
- [x] ASR startup failure returns to recoverable state.

**Acceptance**

- [ ] Existing command ASR receives one continuous wake phrase + command utterance.
- [ ] No first-word clipping occurs in deterministic acceptance.

**Evidence:** `docs/evidence/WWR-310_HANDOFF_BOUNDARY_2026-09-22.md`; implementation/evidence merged in PR #377 at `efe91096135956d3e4454f1957f8623a1fa264e7`. Exact merged-master ordinary CI `35759517089` passed. The merged evidence covers deterministic pre-roll snapshot, post-trigger live retention, one-shot handoff payload transfer, contiguous wake phrase plus first command word in provider-neutral command-ASR audio, no duplicate/inverted sample ranges, stale handoff clearing for failure/cancellation paths, and recoverable return to Listening. Full normal command-ASR activation and real/reproducible audio acceptance remain open.

---

## WWR-400 — Integrate Wake Word with application lifecycle

- [x] Add one Wake Word runtime owner to `AppState` or equivalent authoritative application composition.
- [x] Initialize runtime from persisted settings.
- [x] Keep runtime disabled when setting is false.
- [ ] Start/load/listen when enabled and lifecycle permits.
- [ ] Ensure one wake trigger starts exactly one normal command interaction.
- [ ] Prevent wake activation while command ASR is active.
- [ ] Prevent wake activation while Thinking is active.
- [ ] Suspend wake activation on/before Talking.
- [ ] Clear ring buffer on entry to Talking.
- [ ] Keep Wake Word suspended throughout TTS playback.
- [ ] Resume after successful TTS completion.
- [ ] Resume after TTS cancellation.
- [ ] Resume after recoverable TTS failure.
- [ ] Resume after recoverable command interaction failure.
- [ ] Clear stale pre-roll before resuming.
- [ ] Reset KWS state before resuming where required.
- [ ] Ensure disabling Wake Word during an interaction results in Disabled rather than an unintended resume.
- [ ] Preserve manual interaction after Wake Word errors.
- [ ] Do not implement barge-in.

**Tests**

- [ ] Idle + enabled -> Listening.
- [ ] Idle + disabled -> Disabled/manual behavior.
- [ ] Trigger -> one command activation.
- [ ] Repeated positive frames -> no duplicate activation.
- [ ] Talking -> Suspended.
- [ ] TTS success -> Listening when enabled.
- [ ] TTS cancellation -> Listening when enabled.
- [ ] TTS recoverable failure -> Listening when enabled.
- [ ] Disabled during Talking -> Disabled after TTS.
- [ ] Wake error does not break manual listen.
- [ ] No path leaves runtime permanently suspended unintentionally.

**Acceptance**

- [ ] Moose cannot wake itself from its own TTS in V1.
- [ ] One integrated lifecycle controls Wake Word end-to-end.

---

## WWR-410 — Finalize debounce/trigger semantics

- [x] Preserve one wake event → one command activation invariant.
- [x] Ignore repeated positive KWS frames after acceptance.
- [x] Reset KWS stream at the verified lifecycle point.
- [x] Permit a later phrase after return to Listening.
- [x] Avoid cooldown unless real acceptance shows it is required.

- [x] If cooldown is added, make it bounded/configured and document measured justification. — Not applicable in V1: no cooldown is implemented.
- [x] Keep trigger count privacy-safe.

- [x] Keep last-trigger timing privacy-safe and monotonic where practical.

**Tests**

- [x] One phrase with repeated positive frames yields one interaction.
- [x] A second phrase after resume yields a second interaction.
- [x] No cooldown is needed for correctness tests.
- [x] Trigger diagnostics do not expose PCM.

**Acceptance**

- [x] Debounce behavior is lifecycle-correct rather than timer-masking a state bug.

**Evidence:** `docs/evidence/WWR-410_DEBOUNCE_TRIGGER_SEMANTICS_2026-09-22.md`.

---

## WWR-500 — Implement Wake Word Settings UI

- [x] Add Wake Word section under Settings.
- [x] Add `Enable wake word` toggle.
- [x] Display fixed phrase `Hey, Moose`.
- [x] Do not expose arbitrary phrase editing.
- [x] Do not expose sensitivity in V1.
- [x] Explain local/offline keyword spotting.
- [x] Explain microphone remains locally active while listening for Wake Word.
- [x] Show useful runtime status: loading/listening/suspended/error.
- [x] Show sanitized failure/help text.
- [x] Apply toggle to runtime without app restart when safe.
- [x] Ensure disabling restores manual behavior immediately/boundedly.
- [x] Ensure UI never implies full-time cloud transcription.
- [x] Add accessibility labels and keyboard behavior consistent with Settings conventions.

**Tests**

- [x] Default UI shows disabled.
- [x] Toggle persists enabled state.
- [x] Toggle updates runtime.
- [x] Toggle off stops/suspends Wake Word according to policy.
- [x] Phrase is displayed but not editable.
- [x] Local/offline and active-mic disclosures render.
- [x] Runtime error state is displayed without raw path/secret leakage.

**Acceptance**

- [x] A user can enable/disable Wake Word entirely through the normal Settings UI.

---

## WWR-510 — Complete privacy-safe diagnostics

- [x] Expose Wake Word enabled state.
- [x] Expose authoritative runtime state.
- [x] Expose exact model identity.
- [x] Expose exact runtime version/identity.
- [x] Expose platform/architecture.
- [x] Expose one-thread policy.
- [x] Expose canonical sample rate/channels.
- [x] Expose ring duration/capacity.
- [x] Expose threshold/score.
- [x] Expose trigger count.
- [x] Expose last-trigger age/timestamp in approved form.
- [x] Expose initialization duration.
- [x] Expose Talking suspension.
- [x] Expose sanitized last error.
- [x] Add optional measured CPU/memory/inference/handoff timing fields as available.
- [x] Ensure raw PCM cannot be represented/serialized.
- [x] Audit errors/logs for credentials.
- [x] Audit errors/logs for unnecessary absolute paths.
- [x] Audit errors/logs for audio content.

**Acceptance**

- [x] Diagnostics can troubleshoot lifecycle/artifact/performance issues without exposing audio or secrets.

**Evidence:** `docs/evidence/WWR-510_MEASURED_DIAGNOSTICS_PLACEHOLDERS_2026-09-22.md`, `docs/evidence/WWR-510_PRIVACY_ERROR_AUDIT_GUARD_2026-09-22.md`, `docs/evidence/WWR-510_WAKE_WORD_ERROR_LOG_PRIVACY_AUDIT_2026-09-21.md`, `docs/evidence/WWR-510_WAKE_PRIVACY_LOG_AUDIT_2026-09-21.md`, and `docs/evidence/WWR-510_TODO_RECONCILIATION_2026-09-22.md`. Optional measured CPU/memory/inference/handoff diagnostics are schema placeholders only; WWR-630 remains open for accepted performance measurements.

---

## WWR-600 — Add deterministic Wake Word corpus and harness

### Positive corpus

- [ ] Add multiple reproducible/licensable speakers where available.
- [ ] Add varied speaking volume.
- [ ] Add varied/simulated distance or gain.
- [ ] Add natural `Hey Moose` pronunciation variants.
- [ ] Add Wake Word immediately followed by command.
- [ ] Add reproducible background-noise variants.

### Negative/near-miss corpus

- [ ] Add ordinary speech without Wake Word.
- [ ] Add `Moose` alone.
- [ ] Add `Hey Bruce`.
- [ ] Add `Hey Moosey`.
- [ ] Add phonetically similar phrases.
- [ ] Add sentences containing `moose` without full phrase.
- [ ] Add reproducible background speech/media-like negatives where licensing permits.

### Harness

- [ ] Record fixture provenance/license.
- [ ] Record exact model/runtime identity.
- [ ] Record score/threshold.
- [ ] Record positive detections.

- [ ] Record false rejects.
- [ ] Record negative false accepts.
- [ ] Version explicit pass/fail criteria.
- [ ] Make harness deterministic and CI/report friendly.
- [ ] Prevent copyrighted/non-redistributable fixture leakage.

**Acceptance**

- [ ] Acceptance measures recall and false-trigger behavior, not a single happy path.
- [ ] Claims are limited to tested conditions.

---

## WWR-610 — Add real Linux x86_64 sherpa KWS acceptance

- [ ] Prepare exact pinned model/runtime.
- [ ] Verify every hash before inference.
- [ ] Verify ELF x86_64 runtime architecture.
- [ ] Verify CPU-only production path.
- [ ] Verify one-thread policy.
- [ ] Run real positive Wake Word fixture.
- [ ] Run at least one real negative fixture.
- [ ] Verify inference succeeds offline after artifact preparation.
- [ ] Capture privacy-safe diagnostics/evidence.
- [ ] Record exact commit, manifest, runner/platform details, and CI/run ID.

**Acceptance**

- [ ] Linux x86_64 support claim is backed by real KWS inference.

---

## WWR-620 — Add real macOS arm64 sherpa KWS acceptance

- [ ] Prepare exact pinned model/runtime.
- [ ] Verify every hash before inference.
- [ ] Verify Mach-O arm64 runtime architecture.
- [ ] Verify CPU-only production path.
- [ ] Verify one-thread policy.
- [ ] Run real positive Wake Word fixture.
- [ ] Run at least one real negative fixture.
- [ ] Verify inference succeeds offline after artifact preparation.
- [ ] Capture privacy-safe diagnostics/evidence.
- [ ] Record exact commit, manifest, runner/platform details, and CI/run ID.

**Acceptance**

- [ ] macOS arm64 support claim is backed by real KWS inference.

---

## WWR-630 — Add performance evidence

- [ ] Measure idle Wake Word CPU utilization on representative Linux acceptance environment.
- [ ] Measure idle Wake Word CPU utilization on representative macOS acceptance environment.
- [ ] Measure Wake Word runtime memory overhead.
- [ ] Measure inference timing/real-time behavior.
- [ ] Measure wake detection → command ASR activation latency.
- [ ] Measure pre-roll replay/startup timing.
- [ ] Measure repeated-cycle resource behavior.
- [ ] Compare idle KWS cost with continuously running full ASR path.
- [ ] Preserve one-thread policy unless measured evidence requires change.
- [ ] If thread policy changes, update spec/config/corpus thresholds and record justification.

**Acceptance**

- [ ] A reproducible performance baseline exists.
- [ ] KWS is demonstrably lighter than continuous full ASR for idle wake detection.

---

## WWR-640 — Add integrated lifecycle stability acceptance

- [ ] Run repeated wake→ASR→Thinking→Talking→wake cycles.
- [ ] Verify no native runtime/session growth.
- [ ] Verify no capture-stream multiplication.
- [ ] Verify ring-buffer memory remains bounded.
- [ ] Verify successful TTS repeatedly resumes Wake Word.
- [ ] Verify cancelled TTS repeatedly resumes Wake Word.
- [ ] Verify recoverable TTS failure resumes Wake Word.
- [ ] Verify repeated disable/enable cycles.
- [ ] Verify shutdown while Listening.
- [ ] Verify shutdown during handoff.
- [ ] Add bounded soak/false-trigger acceptance where practical.
- [ ] Record resource counts/state after each cycle or suitable intervals.

**Acceptance**

- [ ] No leak, duplicate stream, or stuck state is observed under the defined acceptance.

---

## WWR-700 — Correct and complete documentation

- [ ] Update architecture docs to the one authoritative Wake Word subsystem.
- [ ] Remove references that imply duplicate runtime stacks are both authoritative.
- [ ] Document fixed phrase `Hey, Moose`.
- [ ] Document disabled-by-default policy.
- [ ] Document local/offline KWS.
- [ ] Document active local microphone behavior.
- [ ] Document wake phrase + prompt may reach command ASR.
- [ ] Document Talking suspension.
- [ ] Document no-barge-in limitation.
- [ ] Document memory-only ring/pre-roll behavior.
- [ ] Document exact model/runtime provenance/licenses.
- [ ] Document supported platforms based only on real acceptance.
- [ ] Document diagnostics/troubleshooting.
- [ ] Correct any current docs that describe planned behavior as already functional.
- [ ] Update README/user docs when feature becomes usable.
- [ ] Avoid unmeasured subjective accuracy claims.

**Acceptance**

- [ ] User-facing docs match actual production behavior on master.
- [ ] Developer docs match actual authoritative source ownership.

---

## WWR-800 — Add specialized Wake CI gates

- [ ] Define deterministic corpus CI/validation gate.
- [ ] Define Linux real KWS acceptance gate.
- [ ] Define macOS arm64 real KWS acceptance gate.
- [ ] Define native packaging/architecture gate.
- [ ] Define repeated lifecycle stability gate.
- [ ] Define performance evidence gate/report policy.
- [ ] Ensure required gates are exact-head bound.
- [ ] Ensure required gates are not silently treated as passed when skipped.
- [ ] Document which gates require specialized runners/hardware.
- [ ] Ensure final merge eligibility checks required Wake gates in addition to ordinary CI.

**Acceptance**

- [ ] A final feature head cannot qualify using ordinary CI alone.

---

## WWR-900 — Final source/privacy/security audit

- [ ] Audit single Wake runtime ownership.
- [ ] Audit microphone ownership transitions.
- [ ] Audit cancellation/shutdown.
- [ ] Audit ring-buffer clearing.
- [ ] Audit Wake-disabled behavior.
- [ ] Audit Talking suspension/resume.
- [ ] Audit one-trigger/one-command invariant.
- [ ] Audit provider separation/no cloud fallback.
- [ ] Audit exact artifact/runtime loading.
- [ ] Audit native architecture verification.
- [ ] Audit logs/errors/metrics for raw audio.
- [ ] Audit logs/errors/metrics for secrets.
- [ ] Audit logs/errors/metrics for unnecessary paths.
- [ ] Confirm no network dependency during idle KWS inference.
- [ ] Confirm no full-time ASR remains active merely for wake detection.
- [ ] Confirm no user-facing docs overstate acceptance.
- [ ] Confirm all mandatory review findings are closed.

**Acceptance**

- [ ] No mandatory Wake Word V1 defect remains unresolved.

---

## WWR-910 — Reconcile original 314-item TODO

- [ ] Reload `docs/WAKE_WORD_V1_TODO_2026-09-14.md` from the final feature head.
- [ ] Re-evaluate every original checkbox against final source/evidence.
- [ ] Mark completed items only when objective evidence exists.
- [ ] Annotate any requirement superseded by the remediation spec.
- [ ] Do not mark design-only documentation as implementation completion.
- [ ] Record evidence paths/run IDs beside task sections or in a linked evidence matrix.
- [ ] Ensure WW-000 remains traceable.
- [ ] Ensure WW-100/110 reflect real populated artifact identities.
- [ ] Ensure WW-200 remains intact.
- [ ] Ensure WW-300/310 refer to the consolidated authoritative implementation.
- [ ] Ensure WW-400/410 are proven by integrated production tests.
- [ ] Ensure WW-500 live-write bug is represented as fixed evidence.
- [ ] Ensure WW-510 UI is genuinely present.
- [ ] Ensure WW-600/610 are integrated rather than component-only.
- [ ] Ensure WW-700/710 have production evidence.
- [ ] Ensure WW-800/810/820 acceptance actually passes.
- [ ] Ensure WW-900 docs are truthful.
- [ ] Ensure WW-950 final audit is complete.
- [ ] Leave WW-960/970 for exact-head/final merge evidence.

**Acceptance**

- [ ] Original TODO is no longer stale.
- [ ] Reconciliation does not create evidence-only recursion.


---

## WWR-950 — Exact-head final qualification

### Diff/review

- [ ] Reload latest `master`.
- [ ] Review exact final feature branch diff against current `master`.
- [ ] Confirm duplicate Wake Word stacks are gone.
- [ ] Confirm no unrelated ASR/TTS regression is introduced.
- [ ] Record exact final PR head SHA.

### Ordinary gates

- [ ] Rust formatting passes.
- [ ] Clippy/lint passes.
- [ ] Complete relevant Rust tests pass.
- [ ] Frontend tests/checks pass.
- [ ] Generated settings/backend contract is current.
- [ ] Artifact Python tests pass.

### Wake-specific gates

- [ ] Deterministic corpus gate passes.
- [ ] Linux x86_64 real KWS gate passes.
- [ ] macOS arm64 real KWS gate passes.
- [ ] Native package/architecture gate passes.
- [ ] Integrated lifecycle stability gate passes.
- [ ] Performance evidence is recorded and accepted.
- [ ] Privacy/security audit passes.
- [ ] Documentation audit passes.

### Evidence

- [ ] Record exact ordinary CI run ID.
- [ ] Record exact Wake-specific run IDs/reports.
- [ ] Record exact artifact manifest revision.
- [ ] Record exact corpus version.
- [ ] Record exact acceptance platform details.

**Acceptance**

- [ ] No stale or partially qualified head is eligible for merge.

---

## WWR-960 — Guarded merge and exact-master verification

- [ ] Recheck PR mergeability immediately before merge.
- [ ] Recheck exact head SHA immediately before merge.
- [ ] Merge only the exact tested head.
- [ ] Use an allowed guarded merge method.
- [ ] Record exact merged master SHA.
- [ ] Verify ordinary CI on exact merged master.
- [ ] Re-run exact-master Wake gates required by policy/final diff.
- [ ] Verify exact-master artifact manifest is unchanged from qualified head.
- [ ] Verify exact-master corpus version is unchanged from qualified head.
- [ ] Reload and finish original WW-960/WW-970 reconciliation.
- [ ] Confirm final README/docs refer to the merged implementation.
- [ ] Record final closeout evidence without opening an evidence-only implementation loop.

**Acceptance**

- [ ] `master` contains the complete Wake Word V1 implementation.
- [ ] Required exact-master validation evidence passes.
- [ ] Original TODO and remediation TODO are reconciled.

---

## Final remediation checklist

### Architecture

- [x] One authoritative Wake Word subsystem exists.
- [x] One authoritative `WakeWordRuntimeManager` exists.
- [x] One authoritative KWS config/engine policy exists.
- [x] Duplicate legacy Wake Word stacks are removed.

### Settings/UI

- [x] Live settings validation cannot persist an invalid phrase.
- [x] Wake defaults disabled.
- [x] Phrase is fixed to `Hey, Moose`.
- [x] Settings UI can enable/disable Wake Word.
- [x] UI discloses local/offline KWS and active microphone behavior.
- [x] Manual behavior is preserved when Wake Word is disabled.

### Artifacts/engine

- [ ] Model archive/files have immutable identities.
- [ ] Native runtimes have immutable identities.
- [ ] Model/runtime licenses and notices are verified.
- [ ] Real native sherpa session loads exact verified inputs.
- [x] KWS uses 16 kHz mono, one thread, score 1.0, threshold 0.25.
- [ ] KWS is local/offline during idle inference.

### Audio/handoff/lifecycle

- [ ] One authoritative microphone ownership path exists.
- [x] PCM is validated before retention.
- [ ] Ring buffer and KWS share one chronological canonical stream.
- [ ] Wake→ASR pre-roll/live handoff is continuous.
- [ ] First command word is not clipped.
- [ ] One wake event creates one command interaction.
- [ ] Wake is suspended while Moose talks.
- [ ] Wake resumes after TTS success/cancellation/recoverable failure.
- [ ] No V1 barge-in exists.

### Privacy/quality

- [ ] Raw Wake PCM remains memory-only.
- [x] Diagnostics are privacy-safe.
- [ ] No silent cloud/full-ASR fallback exists.
- [ ] Corpus acceptance passes.
- [ ] Linux real KWS acceptance passes.
- [ ] macOS arm64 real KWS acceptance passes.
- [ ] Lifecycle stability passes.
- [ ] Performance baseline is recorded.
- [ ] Documentation is truthful.

### Closeout

- [ ] Artifact/freezer CI coverage is complete.
- [ ] Exact PR-head required gates pass.
- [ ] Guarded merge uses exact tested head.
- [ ] Exact merged-master required gates pass.
- [ ] Original 314-item TODO is reconciled.
- [ ] No mandatory code-review finding remains open.
