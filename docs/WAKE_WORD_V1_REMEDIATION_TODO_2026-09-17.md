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

- [ ] Confirm current implementation base is `9f5d90b4b15f16c1ef2d473e574537392c310f0b` or a later verified descendant.
- [ ] Record exact starting master SHA and ordinary CI run.
- [ ] Preserve manual-listen behavior while Wake Word remains disabled.
- [ ] Preserve ASR provider selection/fallback policy.
- [ ] Preserve Local/Google/Gemini TTS separation.
- [ ] Preserve Local TTS production thread policy.
- [ ] Preserve no-barge-in V1 policy.
- [ ] Preserve no acoustic wake-phrase trimming V1 policy.
- [ ] Preserve `PcmRingBuffer` behavior and tests.
- [ ] Preserve fail-closed artifact behavior until real identities are populated.
- [ ] Add/update a remediation evidence file recording baseline and scope.

**Acceptance**

- [ ] No remediation change silently changes non-Wake ASR/TTS/manual behavior.
- [ ] Baseline is reproducible from repository evidence.

---

## WWR-010 — Fix live Wake Word settings validation defect

- [ ] Identify the one canonical Wake Word settings validation/normalization function.
- [ ] Route persisted-load validation through that function.
- [ ] Route live `update_settings` validation through the same function.
- [ ] Ensure `wake_word_phrase` normalizes case/whitespace to exact `Hey, Moose`.
- [ ] Reject any other Wake Word phrase before state mutation or persistence.
- [ ] Ensure invalid type/value does not partially persist Wake Word state.
- [ ] Preserve unrelated ASR/TTS fields on successful Wake Word updates.
- [ ] Preserve previous valid persisted settings on failed update.

**Tests**

- [ ] Live update accepts canonical `Hey, Moose`.
- [ ] Live update normalizes `  hey, moose  `.
- [ ] Live update rejects `Hey Bruce`.
- [ ] Rejected update does not persist invalid JSON/settings.
- [ ] Restart/load after rejected update still succeeds.
- [ ] Missing Wake Word fields still migrate to disabled + canonical phrase.
- [ ] Existing unrelated ASR/TTS settings remain unchanged.

**Acceptance**

- [ ] There is no settings path that can persist a value startup validation later rejects.
- [ ] Command-level regression test covers the previously identified defect.

---

## WWR-020 — Consolidate duplicate Wake Word module architecture

- [ ] Choose a single canonical module boundary, preferably `src-tauri/src/wake_word/`.
- [ ] Inventory all functionality in `app/wake_word_*`.
- [ ] Inventory all functionality in `asr/wake_word_*`.
- [ ] Select one authoritative KWS config representation.
- [ ] Select one authoritative KWS engine/session abstraction.
- [ ] Select one authoritative `WakeWordRuntimeManager`.
- [ ] Select one authoritative handoff implementation.
- [ ] Select one authoritative diagnostics representation.
- [ ] Move/rehome reusable tests to the canonical module.
- [ ] Remove duplicate/obsolete public exports.
- [ ] Remove duplicate/obsolete source files after migration.
- [ ] Ensure production code cannot instantiate two independent Wake Word managers.
- [ ] Ensure Wake Word remains distinct from command ASR provider implementations.
- [ ] Add structural/source regression coverage against duplicate manager reintroduction.

**Acceptance**

- [ ] Exactly one production `WakeWordRuntimeManager` exists.
- [ ] Exactly one production V1 KWS config/engine policy exists.
- [ ] No tests depend on the removed duplicate implementation.
- [ ] Ordinary CI passes after consolidation.

---

## WWR-030 — Freeze one authoritative V1 KWS policy

- [ ] Define canonical constants in one source location:
  - [ ] sample rate `16_000 Hz`;
  - [ ] channels `1`;
  - [ ] feature dimension `80`;
  - [ ] inference threads `1`;
  - [ ] keyword source `HEY MOOSE`;
  - [ ] score `1.0`;
  - [ ] threshold `0.25`;
  - [ ] pre-roll `2 seconds`.
- [ ] Make settings/docs/diagnostics/tests consume or verify the same constants.
- [ ] Remove permissive parallel config validation that merely accepts any positive threshold/score.
- [ ] Make config drift fail deterministically.
- [ ] Ensure model artifact schema matches engine config schema.

**Tests**

- [ ] Any threshold other than `0.25` fails V1 config validation.
- [ ] Any score other than `1.0` fails V1 config validation.
- [ ] Any inference thread count other than `1` fails V1 validation.
- [ ] Non-16-kHz or non-mono KWS input fails before retention/inference.
- [ ] Artifact contract test verifies encoder/decoder/joiner/tokens/BPE expectations.

**Acceptance**

- [ ] No production module can configure a contradictory Wake Word V1 policy.

---

## WWR-100 — Complete model artifact identities and provenance

- [ ] Obtain the exact selected GigaSpeech KWS model archive through an independently verifiable source.
- [ ] Run deterministic identity freezing.
- [ ] Record model archive byte size.
- [ ] Record model archive SHA-256.
- [ ] Record exact encoder filename/path, byte size, SHA-256.
- [ ] Record exact decoder filename/path, byte size, SHA-256.
- [ ] Record exact joiner filename/path, byte size, SHA-256.
- [ ] Record exact `tokens.txt` byte size and SHA-256.
- [ ] Record exact `bpe.model` byte size and SHA-256.
- [ ] Deterministically generate/freeze the exact `HEY MOOSE` keyword representation required by the native API.
- [ ] Record keyword artifact bytes/hash if represented as a file.
- [ ] Populate `wake-word-artifacts.json` with production model entries.
- [ ] Populate/replace Rust manifest placeholders with real identities or generate Rust data from the authoritative manifest.
- [ ] Ensure Rust and JSON manifests cannot drift silently.
- [ ] Remove zero-byte/empty-hash production placeholders.

**Provenance/license**

- [ ] Independently verify model provenance.
- [ ] Independently verify model license.
- [ ] Remove any unverified hard-coded license assertion.
- [ ] Add required model attribution/notices.
- [ ] Make docs distinguish runtime license from model license.

**Acceptance**

- [ ] Every consumed production model/tokenizer/keyword input has immutable identity.
- [ ] Repository provenance/license information is internally consistent.
- [ ] Any byte mismatch fails closed.

---

## WWR-110 — Complete sherpa native runtime identities and packaging

- [ ] Verify the frozen sherpa runtime version used for V1.
- [ ] Obtain exact Linux x86_64 runtime archive.
- [ ] Freeze Linux archive byte size/SHA-256.
- [ ] Freeze all actually loaded Linux native library byte sizes/SHA-256 values.
- [ ] Verify ELF x86_64 architecture before use.
- [ ] Obtain exact macOS arm64 runtime archive.
- [ ] Freeze macOS archive byte size/SHA-256.
- [ ] Freeze all actually loaded macOS native library byte sizes/SHA-256 values.
- [ ] Verify Mach-O arm64 architecture before use.
- [ ] Define deterministic installed runtime directory layout.
- [ ] Define deterministic package/bundle locations.
- [ ] Ensure cached artifacts still undergo hash verification.
- [ ] Produce sanitized unsupported-platform errors.
- [ ] Add complete sherpa runtime Apache-2.0 attribution/notices.
- [ ] Do not add additional supported architectures without real acceptance.

**Tests**

- [ ] Wrong Linux architecture fails.
- [ ] Wrong macOS architecture fails.
- [ ] Corrupt archive fails.
- [ ] Corrupt consumed library fails.
- [ ] Missing required library fails.
- [ ] Unsupported platform fails with sanitized explicit error.
- [ ] Offline prepared runtime can be located deterministically.

**Acceptance**

- [ ] Claimed packages can locate the exact pinned runtime offline.
- [ ] No wrong-architecture/corrupt runtime can reach inference.

---

## WWR-120 — Expand artifact CI coverage

- [ ] Update Wake artifact workflow path filters to include:
  - [ ] model identity freezer;
  - [ ] runtime identity freezer;
  - [ ] their test files;
  - [ ] production manifest(s);
  - [ ] preparation script;
  - [ ] verification script.
- [ ] Run all four Wake artifact Python test suites in CI.
- [ ] Add manifest consistency/schema validation.
- [ ] Add a check that production-required identities are non-placeholder when production mode is enabled.
- [ ] Add test coverage for safe extraction/path traversal rejection.
- [ ] Add test coverage proving cache cannot bypass identity verification.

**Acceptance**

- [ ] A change to either freezer script triggers and exercises the Wake artifact workflow.
- [ ] Artifact tooling regression cannot merge behind a skipped path filter.

---

## WWR-200 — Implement the real native sherpa KWS session

- [ ] Implement the real native `NativeKwsSession`/equivalent adapter.
- [ ] Load the exact verified sherpa native runtime.
- [ ] Load exact verified encoder.
- [ ] Load exact verified decoder.
- [ ] Load exact verified joiner.
- [ ] Load exact verified `tokens.txt`.
- [ ] Load exact verified `bpe.model`.
- [ ] Configure deterministic `HEY MOOSE` keyword representation.
- [ ] Configure one inference thread.
- [ ] Configure score `1.0`.
- [ ] Configure threshold `0.25`.
- [ ] Feed streaming 16-kHz mono PCM.
- [ ] Emit bounded Wake Word detection event with no raw audio.
- [ ] Reset stream state after accepted recognition.
- [ ] Implement idempotent shutdown.
- [ ] Add cancellation/interrupt behavior where native API permits.
- [ ] Map native errors to sanitized Wake Word errors.
- [ ] Ensure normal inference has no network dependency.
- [ ] Ensure engine never performs full transcription.

**Tests**

- [ ] Missing verified artifact fails before native session creation.
- [ ] Corrupt verified artifact fails before native session creation.
- [ ] Wrong architecture fails before native load.
- [ ] Native load failure is sanitized.
- [ ] Thread/threshold/score policy is observable.
- [ ] Fake-session tests remain for deterministic unit coverage.
- [ ] Real positive fixture triggers.
- [ ] Real negative fixture does not trigger.

**Acceptance**

- [ ] Production engine can execute real pinned sherpa KWS offline.
- [ ] Fake session is not used by production composition.

---

## WWR-210 — Fix PCM validation ordering

- [ ] Validate/canonicalize PCM before ring-buffer append.
- [ ] Validate/canonicalize PCM before KWS feed.
- [ ] Validate/canonicalize PCM before live handoff append.
- [ ] Ensure invalid frames do not mutate Wake Word retained state.
- [ ] Ensure prior valid pre-roll remains unchanged after invalid input.

**Tests**

- [ ] 48-kHz frame is rejected before append.
- [ ] Empty frame is rejected before append.
- [ ] Invalid frame is not fed to KWS.
- [ ] Invalid frame does not alter ring snapshot.
- [ ] Valid canonical frame is appended/fed exactly once.

**Acceptance**

- [ ] Non-canonical PCM cannot contaminate pre-roll.

---

## WWR-300 — Integrate one authoritative microphone routing path

- [ ] Re-audit current `AudioCapture` ownership on the post-consolidation source.
- [ ] Select and document the final one-stream/routing strategy.
- [ ] Wire Wake Word manager into production application state/composition.
- [ ] Ensure Wake Word does not open a competing continuous microphone stream.
- [ ] Canonicalize/resample microphone PCM once where practical.
- [ ] Feed ring buffer and KWS from the same chronological canonical stream.
- [ ] Preserve live samples immediately after trigger while command ASR initializes.
- [ ] Implement deterministic ownership transfer to command ASR.
- [ ] Implement deterministic ownership return to wake listening.
- [ ] Handle device disconnect.
- [ ] Handle reconnect.
- [ ] Handle unavailable device.
- [ ] Handle permission/capture error without spin/deadlock.
- [ ] Ensure cancellation does not orphan or multiply streams.

**Tests**

- [ ] Repeated wake→ASR→wake cycles do not increase capture-stream count.
- [ ] Wake disable tears down/suspends capture according to final policy.
- [ ] Manual listen still works with Wake Word disabled.
- [ ] Device error leaves deterministic ownership.
- [ ] Cancellation leaves deterministic ownership.

**Acceptance**

- [ ] Exactly one authoritative microphone ownership model exists in production.
- [ ] No simultaneous competing capture opens occur.

---

## WWR-310 — Complete production wake→ASR pre-roll/live handoff

- [ ] On accepted trigger, snapshot ring chronologically.
- [ ] Start preserving subsequent live canonical samples.
- [ ] Activate the existing normal command ASR path exactly once.
- [ ] Replay pre-roll into command ASR.
- [ ] Continue with live PCM.
- [ ] Prevent gaps at snapshot/live boundary.
- [ ] Prevent duplicated sample ranges.
- [ ] Preserve complete `Hey, Moose` phrase when present in the two-second window.
- [ ] Preserve immediate first command word.
- [ ] Do not acoustically trim Wake Word in V1.
- [ ] Clear stale handoff/pre-roll after success.
- [ ] Clear stale handoff/pre-roll after ASR startup failure.
- [ ] Clear stale handoff/pre-roll after cancellation.
- [ ] Return runtime to a valid recoverable state after handoff failure.

**Tests**

- [ ] Synthetic exact sample-order boundary test.
- [ ] Real/reproducible `Hey Moose, tell me the time` audio acceptance.
- [ ] First command word is present in downstream ASR acceptance.
- [ ] No duplicate range is observed.
- [ ] No inversion is observed.
- [ ] ASR startup failure returns to recoverable state.

**Acceptance**

- [ ] Existing command ASR receives one continuous wake phrase + command utterance.
- [ ] No first-word clipping occurs in deterministic acceptance.

---

## WWR-400 — Integrate Wake Word with application lifecycle

- [ ] Add one Wake Word runtime owner to `AppState` or equivalent authoritative application composition.
- [ ] Initialize runtime from persisted settings.
- [ ] Keep runtime disabled when setting is false.
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

- [ ] Preserve one wake event → one command activation invariant.
- [ ] Ignore repeated positive KWS frames after acceptance.
- [ ] Reset KWS stream at the verified lifecycle point.
- [ ] Permit a later phrase after return to Listening.
- [ ] Avoid cooldown unless real acceptance shows it is required.
- [ ] If cooldown is added, make it bounded/configured and document measured justification.
- [ ] Keep trigger count privacy-safe.
- [ ] Keep last-trigger timing privacy-safe and monotonic where practical.

**Tests**

- [ ] One phrase with repeated positive frames yields one interaction.
- [ ] A second phrase after resume yields a second interaction.
- [ ] No cooldown is needed for correctness tests.
- [ ] Trigger diagnostics do not expose PCM.

**Acceptance**

- [ ] Debounce behavior is lifecycle-correct rather than timer-masking a state bug.

---

## WWR-500 — Implement Wake Word Settings UI

- [ ] Add Wake Word section under Settings.
- [ ] Add `Enable wake word` toggle.
- [ ] Display fixed phrase `Hey, Moose`.
- [ ] Do not expose arbitrary phrase editing.
- [ ] Do not expose sensitivity in V1.
- [ ] Explain local/offline keyword spotting.
- [ ] Explain microphone remains locally active while listening for Wake Word.
- [ ] Show useful runtime status: loading/listening/suspended/error.
- [ ] Show sanitized failure/help text.
- [ ] Apply toggle to runtime without app restart when safe.
- [ ] Ensure disabling restores manual behavior immediately/boundedly.
- [ ] Ensure UI never implies full-time cloud transcription.
- [ ] Add accessibility labels and keyboard behavior consistent with Settings conventions.

**Tests**

- [ ] Default UI shows disabled.
- [ ] Toggle persists enabled state.
- [ ] Toggle updates runtime.
- [ ] Toggle off stops/suspends Wake Word according to policy.
- [ ] Phrase is displayed but not editable.
- [ ] Local/offline and active-mic disclosures render.
- [ ] Runtime error state is displayed without raw path/secret leakage.

**Acceptance**

- [ ] A user can enable/disable Wake Word entirely through the normal Settings UI.

---

## WWR-510 — Complete privacy-safe diagnostics

- [ ] Expose Wake Word enabled state.
- [ ] Expose authoritative runtime state.
- [ ] Expose exact model identity.
- [ ] Expose exact runtime version/identity.
- [ ] Expose platform/architecture.
- [ ] Expose one-thread policy.
- [ ] Expose canonical sample rate/channels.
- [ ] Expose ring duration/capacity.
- [ ] Expose threshold/score.
- [ ] Expose trigger count.
- [ ] Expose last-trigger age/timestamp in approved form.
- [ ] Expose initialization duration.
- [ ] Expose Talking suspension.
- [ ] Expose sanitized last error.
- [ ] Add optional measured CPU/memory/inference/handoff timing fields as available.
- [ ] Ensure raw PCM cannot be represented/serialized.
- [ ] Audit errors/logs for credentials.
- [ ] Audit errors/logs for unnecessary absolute paths.
- [ ] Audit errors/logs for audio content.

**Acceptance**

- [ ] Diagnostics can troubleshoot lifecycle/artifact/performance issues without exposing audio or secrets.

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

- [ ] One authoritative Wake Word subsystem exists.
- [ ] One authoritative `WakeWordRuntimeManager` exists.
- [ ] One authoritative KWS config/engine policy exists.
- [ ] Duplicate legacy Wake Word stacks are removed.

### Settings/UI

- [ ] Live settings validation cannot persist an invalid phrase.
- [ ] Wake defaults disabled.
- [ ] Phrase is fixed to `Hey, Moose`.
- [ ] Settings UI can enable/disable Wake Word.
- [ ] UI discloses local/offline KWS and active microphone behavior.
- [ ] Manual behavior is preserved when Wake Word is disabled.

### Artifacts/engine

- [ ] Model archive/files have immutable identities.
- [ ] Native runtimes have immutable identities.
- [ ] Model/runtime licenses and notices are verified.
- [ ] Real native sherpa session loads exact verified inputs.
- [ ] KWS uses 16 kHz mono, one thread, score 1.0, threshold 0.25.
- [ ] KWS is local/offline during idle inference.

### Audio/handoff/lifecycle

- [ ] One authoritative microphone ownership path exists.
- [ ] PCM is validated before retention.
- [ ] Ring buffer and KWS share one chronological canonical stream.
- [ ] Wake→ASR pre-roll/live handoff is continuous.
- [ ] First command word is not clipped.
- [ ] One wake event creates one command interaction.
- [ ] Wake is suspended while Moose talks.
- [ ] Wake resumes after TTS success/cancellation/recoverable failure.
- [ ] No V1 barge-in exists.

### Privacy/quality

- [ ] Raw Wake PCM remains memory-only.
- [ ] Diagnostics are privacy-safe.
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
