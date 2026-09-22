# Wake Word V1 — Code-Review Remediation TODO

**Date:** 2026-09-17  
**Repository:** `ekkus93/ai-talking-moose`  
**Baseline reviewed:** `master` at `9f5d90b4b15f16c1ef2d473e574537392c310f0b`  
**Status:** Blocking remediation plan before Wake Word V1 may be considered complete  
**Primary review:** `docs/WAKE_WORD_V1_CODE_REVIEW_2026-09-17.md`  
**Original implementation TODO:** `docs/WAKE_WORD_V1_TODO_2026-09-14.md`

> Reconciliation note (2026-09-21): WWR-800 gate definitions are implemented by `docs/wake-word-required-gates.json`, `scripts/check_wake_word_required_gates.mjs`, the specialized workflows documented in `docs/WAKE_WORD_V1_CI_GATES.md`, and the fail-closed final-readiness check `scripts/check_wake_word_final_qualification_readiness.mjs`. This closes gate-definition/policy work only; WWR-600/610/620/630/640 real fixture, native inference, performance, and integrated lifecycle acceptance remain pending and must not be inferred from policy-gate success.

## 0. Non-negotiable completion rule

Wake Word V1 is **not complete** until every applicable item in this remediation TODO is implemented, objectively qualified, merged, and reconciled back into `docs/WAKE_WORD_V1_TODO_2026-09-14.md`.

A checked box means the requirement is present in the authoritative production path and backed by evidence. A design document, scaffold, mock, placeholder, skipped workflow, or component-only test is **not** completion evidence unless the item explicitly asks for design work only.

When evidence is incomplete, leave the box unchecked.

---

## 1. Review baseline and blocking findings

The 2026-09-17 review found five mandatory defects:

1. **Two competing Wake Word implementations exist.**
   - `src-tauri/src/asr/wake_word_runtime.rs` contains a substantial `WakeWordRuntimeManager` with ring buffer, lifecycle, diagnostics, trigger routing, and handoff behavior.
   - `src-tauri/src/app/wake_word_engine.rs` contains a second substantial runtime/engine path with native sherpa loading, stream handling, and artifact verification.
   - They are not composed into one authoritative production subsystem.

2. **Live Settings writes are invalid.**
   - `src-tauri/src/app/runtime_preferences.rs` rejects any transition where `wake_word_enabled` changes.
   - The Settings panel writes `wake_word_enabled` immediately.
   - Therefore the visible live toggle is functionally broken.

3. **Wake Word is not wired into the real production microphone/ASR/TTS lifecycle.**
   - The authoritative capture path is still owned by `AppState.audio_capture` / `AudioCapture`.
   - No verified production path feeds those PCM frames into the Wake Word runtime.
   - No verified production handoff stops wake inference, starts command ASR with pre-roll, and resumes Wake Word after TTS.

4. **Current tests overstate production readiness.**
   - Many Wake Word tests instantiate isolated runtime/engine objects directly.
   - They do not prove the real `AppState` + `AudioCapture` + command-ASR + TTS path.

5. **Real native acceptance is missing.**
   - No committed deterministic positive/negative audio corpus.
   - No real Linux x86_64 KWS acceptance evidence.
   - No real macOS arm64 KWS acceptance evidence.
   - No repeated integrated lifecycle/resource-stability acceptance.

These are release-blocking defects. The remediation must fix them without regressing the existing ASR/TTS behavior already merged on `master`.

---

## 2. Execution order

Execute in this order unless a dependency requires a small local reorder:

1. `WWR-000` — freeze baseline and ownership map.
2. `WWR-100` — consolidate the Wake Word implementation.
3. `WWR-200` — wire authoritative production audio capture.
4. `WWR-300` — implement command-ASR handoff and pre-roll.
5. `WWR-400` — implement TTS suspension/resume and terminal-path recovery.
6. `WWR-500` — fix live Settings behavior and complete user-facing diagnostics.
7. `WWR-600` — add deterministic corpus/harness.
8. `WWR-610` — real Linux x86_64 KWS acceptance.
9. `WWR-620` — real macOS arm64 KWS acceptance.
10. `WWR-630` — performance evidence.
11. `WWR-640` — integrated lifecycle/resource stability.
12. `WWR-700` — documentation truthfulness/completeness.
13. `WWR-800` — specialized Wake CI gates.
14. `WWR-900` — final source/privacy/security audit.
15. `WWR-910` — reconcile the original 314-item TODO.
16. `WWR-950` — exact-head final qualification.
17. `WWR-960` — guarded merge and exact-master verification.

---

## WWR-000 — Freeze baseline and authoritative ownership

- [x] Record reviewed `master` SHA.
- [x] Record the Wake-related diff range from the last pre-Wake baseline through current `master`.
- [x] Inventory every Wake-related production module.
- [x] Inventory every Wake-related test module.
- [x] Inventory every Wake-related workflow/script/artifact manifest.
- [x] Identify every production microphone owner.
- [x] Identify every command-ASR entry point.
- [x] Identify every TTS completion/cancel/failure path that must resume Wake Word.
- [x] Identify every Settings write path for `wake_word_enabled`.
- [x] Identify every diagnostics/UI read path.
- [x] Declare the intended authoritative Wake Word runtime owner.
- [x] Declare the intended authoritative KWS engine owner.
- [x] Declare the intended authoritative ring/pre-roll owner.
- [x] Declare the intended authoritative lifecycle state machine owner.
- [x] Mark the duplicate/legacy path to remove or reduce to a thin wrapper.

**Acceptance**

- [x] There is one written ownership map that later tasks can be checked against.
- [x] The ownership map matches actual source paths on the reviewed head.

---

## WWR-100 — Consolidate to one authoritative Wake Word implementation

### Runtime ownership

- [x] Keep exactly one production `WakeWordRuntimeManager`.
- [x] Keep exactly one authoritative Wake Word state machine.
- [x] Keep exactly one authoritative diagnostics snapshot source.
- [x] Keep exactly one authoritative ring/pre-roll implementation.
- [x] Keep exactly one authoritative trigger/handoff path.

### Engine ownership

- [x] Keep exactly one production sherpa KWS engine abstraction.
- [x] Keep exactly one native runtime loading path.
- [x] Keep exactly one model/runtime artifact verification path.
- [x] Keep exactly one native architecture verification path.
- [x] Keep exactly one KWS stream lifecycle path.

### Remove/reduce duplicate stack

- [x] Delete or reduce the non-authoritative stack to thin adapters.
- [x] Remove duplicated enums/state machines that can drift.
- [x] Remove duplicated ring-buffer logic.
- [x] Remove duplicated diagnostics logic.
- [x] Remove duplicated trigger acceptance logic.
- [x] Remove duplicated lifecycle transitions.
- [x] Ensure tests import the authoritative production modules rather than test-only duplicates.

### Composition

- [x] Compose the authoritative runtime manager with the authoritative sherpa engine.
- [x] Define explicit ownership/lifetime boundaries between manager and engine.
- [x] Define one API for PCM ingestion.
- [x] Define one API for trigger acceptance.
- [x] Define one API for command-ASR handoff.
- [x] Define one API for Talking suspension.
- [x] Define one API for resume/re-arm.
- [x] Define one API for disable/shutdown.

**Acceptance**

- [x] Production has one Wake Word subsystem, not two parallel implementations.
- [x] No test-only duplicate is required to explain runtime behavior.

---

## WWR-200 — Wire the real production microphone path

### Authoritative capture

- [x] Feed PCM from the real `AudioCapture` path into the Wake Word subsystem.
- [x] Do not create a second competing always-on microphone stream for Wake Word.
- [x] Keep sample format/channel/rate conversion explicit and testable.
- [x] Verify Wake Word input is 16 kHz mono PCM16 at the KWS boundary.
- [x] Define resampling/downmix behavior if authoritative capture differs.
- [x] Ensure Wake Word sees only local PCM required for inference/pre-roll.

### Capture ownership transitions

- [x] Define who owns capture while `Disabled`.
- [x] Define who owns capture while `Listening`.
- [x] Define who owns capture during `WakeTriggered`.
- [x] Define who owns capture during command ASR.
- [x] Define who owns capture during `Thinking`.
- [x] Define who owns capture during `Talking`.
- [x] Define who owns capture during shutdown.
- [x] Prevent overlapping capture owners during transitions.

### Failure behavior

- [x] Capture-start failure must fail closed.
- [x] Capture-stop failure must not create a second stream.
- [x] Device loss must not silently leave UI/runtime in a false `Listening` state.
- [x] Device restoration behavior must be deterministic.
- [x] Shutdown must terminate capture exactly once.

**Acceptance**

- [x] Real production PCM reaches the authoritative Wake Word manager.
- [x] There is one active microphone stream in steady-state Wake listening.
- [x] Capture ownership is explicit across all lifecycle states.

---

## WWR-300 — Implement wake trigger → command ASR handoff

### Trigger semantics

- [x] Accept one trigger only while armed/listening.
- [x] Transition to `WakeTriggered` exactly once per accepted wake.
- [x] Disarm/stop KWS immediately after trigger acceptance.
- [x] Reject duplicate callbacks for the same trigger.
- [x] Keep trigger callback work bounded and non-blocking.

### Pre-roll semantics

- [x] Snapshot pre-roll exactly once per accepted trigger.
- [x] Preserve bounded in-memory PCM only.
- [x] Clear/reset the ring at the defined boundary after handoff.
- [x] Do not write pre-roll audio to disk.
- [x] Do not log/transcribe pre-roll separately.
- [x] Define whether the Wake phrase is included in the ASR input.

### Command ASR handoff

- [x] Stop/disarm Wake inference before command ASR owns the handoff.
- [x] Deliver pre-roll to the command-ASR path in the defined order.
- [x] Start live command-ASR capture exactly once.
- [x] Prevent Wake Word from re-triggering during command ASR.
- [x] Prevent a second microphone stream from being created for command ASR.
- [x] Ensure command ASR receives audio after the wake trigger without an unbounded gap.

### Failure/cancel

- [x] If command ASR fails to start, recover to a defined state.
- [x] If command ASR is cancelled, recover to a defined state.
- [x] If handoff fails after Wake is disarmed, do not remain stuck indefinitely.

**Acceptance**

- [x] One accepted wake causes one command-ASR session.
- [x] Pre-roll is consumed once and remains memory-only.
- [x] No duplicate capture stream exists during handoff.

---

## WWR-400 — Integrate Talking/TTS suspension and resume

### Suspension

- [x] Suspend/disarm Wake Word before Moose audio playback begins.
- [x] Do not detect Wake Word from Moose's own TTS output.
- [x] Do not create a second capture stream while suspended.

### Resume after terminal TTS paths

- [x] Resume/re-arm after successful TTS completion.
- [x] Resume/re-arm after TTS cancellation.
- [x] Resume/re-arm after recoverable TTS failure.
- [x] Respect latest `wake_word_enabled` value when deciding whether to resume.
- [x] If disabled while Talking, remain disabled after TTS ends.
- [x] If enabled while Talking, resume according to defined policy.

### Lifecycle correctness

- [x] Keep lifecycle transitions idempotent.
- [x] Prevent duplicate re-arm calls from creating duplicate native sessions.
- [x] Prevent stale callbacks from re-arming after disable/shutdown.
- [x] Ensure shutdown wins over late TTS callbacks.

**Acceptance**

- [x] Wake Word is suspended during Moose speech.
- [x] Wake Word resumes correctly after every terminal TTS path.
- [x] No stale callback can resurrect Wake Word after disable/shutdown.

---

## WWR-500 — Fix live Settings behavior and diagnostics

### Live enable/disable

- [x] Make the Settings toggle semantically valid.
- [x] Enabling Wake Word applies to the authoritative runtime immediately or via an explicitly documented restart boundary.
- [x] Disabling Wake Word stops/disarms the authoritative runtime immediately or via an explicitly documented restart boundary.
- [x] Roll back UI state if runtime application fails.
- [x] Persist only a state that matches the runtime contract.

### Settings UI

- [x] Show fixed phrase `Hey, Moose`.
- [x] Show disabled-by-default behavior.
- [x] Show local/offline keyword-spotting disclosure.
- [x] Show active-microphone disclosure.
- [x] Show cloud-boundary disclosure: wake detection itself is not full-time cloud transcription.
- [x] Show no-barge-in limitation.
- [x] Show current runtime status.
- [x] Show actionable last error when present.
- [x] Keep advanced tuning hidden from V1 user settings.

### Diagnostics

- [x] Expose lifecycle state.
- [x] Expose model/runtime identity.
- [x] Expose trigger count.
- [x] Expose last trigger timestamp or elapsed age.
- [x] Expose last sanitized error.
- [x] Expose native session/stream counts if available.
- [x] Expose ring occupancy/capacity if available.
- [ ] Add optional measured CPU/memory/inference/handoff timing fields as available.
- [x] Ensure raw PCM cannot be represented/serialized.
- [ ] Audit errors/logs for credentials.
- [ ] Audit errors/logs for unnecessary absolute paths.
- [ ] Audit errors/logs for audio content.

**Acceptance**

- [x] Diagnostics can troubleshoot lifecycle/artifact/performance issues without exposing audio or secrets.

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

- [x] Define deterministic corpus CI/validation gate.
- [x] Define Linux real KWS acceptance gate.
- [x] Define macOS arm64 real KWS acceptance gate.
- [x] Define native packaging/architecture gate.
- [x] Define repeated lifecycle stability gate.
- [x] Define performance evidence gate/report policy.
- [x] Ensure required gates are exact-head bound.
- [x] Ensure required gates are not silently treated as passed when skipped.
- [x] Document which gates require specialized runners/hardware.
- [x] Ensure final merge eligibility checks required Wake gates in addition to ordinary CI.

**Acceptance**

- [x] A final feature head cannot qualify using ordinary CI alone.

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

## 3. Required evidence discipline

For every implementation PR:

- record exact branch/head SHA;
- record exact CI run IDs;
- record exact artifact manifest revision when artifacts are involved;
- record exact corpus revision when corpus/acceptance is involved;
- distinguish hosted component tests from real native acceptance;
- distinguish schema validation from real inference;
- distinguish source inspection from integrated runtime evidence;
- do not infer macOS arm64 acceptance from Linux;
- do not infer production acceptance from test-only constructors;
- do not infer real audio acceptance from synthetic PCM unless the requirement explicitly allows it;
- do not treat a skipped workflow as passing evidence;
- do not mark original TODO items complete merely because remediation scaffolding exists.

## 4. Definition of done

Wake Word V1 is done only when:

1. there is one authoritative production Wake Word subsystem;
2. real production PCM reaches it;
3. one accepted wake starts one command-ASR session with bounded pre-roll;
4. Wake Word is suspended during Moose speech and resumes on every terminal TTS path;
5. the live Settings toggle is truthful and functional;
6. deterministic corpus tests cover positive and negative behavior;
7. Linux x86_64 and macOS arm64 real native KWS acceptance pass;
8. performance and lifecycle stability evidence are recorded;
9. documentation matches reality;
10. privacy/security review passes;
11. original TODO is reconciled against final evidence;
12. exact-head and exact-master qualification pass.
