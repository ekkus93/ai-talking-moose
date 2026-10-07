# Whisper.cpp Local ASR Post-Review Remediation TODO

**Date:** 2026-10-07  
**Status:** Open  
**Review baseline:** `master` at `b2525ef58e840e7ebc8d7beb64f8de211ffc9955`  
**Spec:** `docs/WHISPER_CPP_LOCAL_ASR_POST_REVIEW_REMEDIATION_SPEC_2026-10-07.md`  
**Original TODO to reconcile at closeout:** `docs/WHISPER_CPP_LOCAL_ASR_TODO.md`

This checklist is authoritative for the 2026-10-07 Whisper.cpp post-review remediation. A checkbox may be marked complete only when the current production source and test/evidence state satisfy the requirement. Existing checkmarks in the original Whisper TODO are not completion evidence for this remediation.

## WPR-100 — Canonical whisper.cpp source provenance

- [ ] Decide and record the one canonical whisper.cpp source revision that production is intended to build.
- [ ] Make `third_party/whisper.cpp` resolve to that exact revision.
- [ ] Make the Whisper manifest/runtime identity report the same exact revision.
- [ ] Make installation/runtime metadata that records Whisper source identity use the same revision.
- [ ] Update `docs/WHISPER_MODEL_LICENSES.md` to the actual source revision.
- [ ] Update `docs/THIRD_PARTY_NOTICES.md` to the actual source revision.
- [ ] Update any current handoff/pipeline/acceptance documentation that reports the source revision.
- [ ] Add a deterministic check that fails when the expected Whisper source revision and tracked/built native source differ.
- [ ] Add a regression fixture/test proving the mismatch check fails on a deliberately wrong revision.
- [ ] Ensure the real-CPU workflow invokes the provenance check before compilation/acceptance.

**Acceptance:** one independently verifiable revision appears everywhere, and a mismatched gitlink/source tree cannot qualify.

## WPR-110 — License and provenance truthfulness

- [ ] Reconcile the `ggml-small.bin` license state across all current repository documents.
- [ ] Reconcile the whisper.cpp source license/attribution state across all current repository documents.
- [ ] Record the exact model URL/revision, expected byte size, and SHA-256 in the authoritative provenance record.
- [ ] Ensure any required redistributable license/notice text is included in the repository/package path used for release.
- [ ] Remove contradictory “verified” versus “pending verification” statements for the same artifact.
- [ ] Add/extend a documentation/provenance check if one can prevent these contradictions from recurring.

**Acceptance:** current release-facing docs agree on source identity, model identity, and license state.

## WPR-200 — Clean-profile install and canonical model layout

- [ ] Make the production Whisper installer create the required install root/parents itself.
- [ ] Perform root creation before staging-directory creation.
- [ ] Perform root creation before any disk-space probe that requires the path to exist.
- [ ] Define one canonical on-disk model directory.
- [ ] Prefer/implement `<app-data>/models/whisper/whisper-small/`, or explicitly update the governing spec if a different layout is chosen.
- [ ] Keep model artifact and model-specific marker/metadata inside the canonical per-model directory where practical.
- [ ] Define migration/compatibility behavior for an existing older `<app-data>/models/whisper/` layout if deployed profiles can contain it.
- [ ] Add a test starting with no Whisper directory at all.
- [ ] Verify first install succeeds without test/workflow code pre-creating the Whisper model root.
- [ ] Verify delete/reinstall remains correct after the layout change.

**Acceptance:** a clean application profile can install, verify, use, delete, and reinstall Whisper through production code alone.

## WPR-210 — Bounded-memory, nonblocking installed-model verification

- [ ] Replace whole-file `fs::read()` verification for Whisper model integrity with streaming reads.
- [ ] Compute SHA-256 incrementally with a bounded buffer.
- [ ] Validate exact byte count while streaming.
- [ ] Preserve any required magic/header validation without loading the entire artifact.
- [ ] Move full-file verification reached from `get_asr_models()` behind `spawn_blocking` or equivalent blocking isolation.
- [ ] Ensure descriptor retrieval does not block a Tokio/Tauri async worker while hashing the model.
- [ ] Add a small-fixture success test for streaming verification.
- [ ] Add short/truncated artifact coverage.
- [ ] Add oversized/wrong-size artifact coverage.
- [ ] Add wrong-SHA/corrupt artifact coverage.
- [ ] Verify memory use is bounded by the verification buffer rather than model size.

**Acceptance:** verification remains fail-closed without allocating approximately the entire model or blocking the async command executor.

## WPR-220 — Correct Whisper error taxonomy

- [ ] Define one explicit internal-to-`AsrErrorKind` mapping for Whisper install/verify/load/infer/lifecycle failures.
- [ ] Map missing model to `ModelNotInstalled`.
- [ ] Map size/SHA/magic/integrity failures to `ModelCorrupt`.
- [ ] Map native-not-linked/unsupported-runtime conditions to `RuntimeUnavailable`.
- [ ] Map verified-model native load failure to `ModelLoadFailed`.
- [ ] Map malformed/invalid PCM input to `AudioInput`.
- [ ] Map native transcription/segment-extraction failure to `Inference`.
- [ ] Map lifecycle misuse to `InvalidState`.
- [ ] Map explicit cancellation to `Cancelled`.
- [ ] Map unexpected invariant/worker failures to `Internal`.
- [ ] Remove dead/unused error-mapping helpers or route production code through them.
- [ ] Add focused tests for every public error kind still claimed as production-reachable.
- [ ] If a kind is intentionally not reachable from Whisper, correct the original TODO/docs instead of manufacturing an artificial path.

**Acceptance:** public diagnostics/errors distinguish corrupt artifacts, unavailable runtime, native load failure, inference failure, and cancellation truthfully.

## WPR-300 — Whisper utterance state and partial/final semantics

- [ ] Replace “every ~300 ms batch is final” behavior with an explicit utterance state machine.
- [ ] Give each active utterance a stable identity across partial updates.
- [ ] Maintain a bounded utterance/current-window PCM buffer.
- [ ] Define a configurable partial-inference cadence.
- [ ] Emit `StreamingTranscriptUpdate::Partial` for nonterminal Whisper results.
- [ ] Ensure a partial update cannot create a provider user turn.
- [ ] Implement a deterministic local endpoint/finalization signal.
- [ ] Define endpoint silence threshold/hangover as named/testable constants or configuration.
- [ ] Define a maximum utterance duration and deterministic forced-finalization/reset behavior.
- [ ] Emit `Final` only for endpoint, explicit finalization, or another documented terminal condition.
- [ ] Reset utterance state only after finalization is delivered.
- [ ] Ensure one ordinary spoken sentence can produce multiple partials but exactly one final user utterance.
- [ ] Preserve bounded memory and bounded compute per utterance.
- [ ] Add tests proving a 300 ms inference cadence does not itself finalize the utterance.
- [ ] Add tests proving partial text can evolve/correct before finalization without duplicate provider commits.

**Acceptance:** Whisper transcript events have truthful streaming semantics and one logical user utterance is not fragmented into short final turns.

## WPR-310 — Deliver final updates produced during stop

- [ ] Change the engine/pipeline stop/finalize contract so final transcript updates can be returned/emitted.
- [ ] Forward stop-time Whisper updates through the normal transcript event path.
- [ ] Preserve ordering: queued PCM -> final inference -> final event -> worker termination.
- [ ] Make stop/finalize idempotent.
- [ ] Ensure repeated stop cannot duplicate the final transcript.
- [ ] Add a regression where the final utterance is shorter than the normal partial threshold/cadence.
- [ ] Assert that the sub-threshold utterance is delivered exactly once on stop.
- [ ] Add an empty/whitespace stop-flush case and ensure it creates no user turn.

**Acceptance:** no valid final transcript generated by normal stop is silently discarded.

## WPR-320 — Drain accepted PCM before normal shutdown

- [ ] Separate normal graceful stop from immediate abort semantics.
- [ ] Stop microphone capture/producer input before draining the local-ASR queue.
- [ ] Signal no-more-input to the worker without immediately discarding accepted chunks.
- [ ] Drain all PCM chunks already accepted into the bounded queue during normal stop.
- [ ] Feed drained chunks into the active Whisper utterance before finalization.
- [ ] Finalize and deliver the utterance before worker exit.
- [ ] Join/retire the worker only after drain/finalization is complete or a bounded terminal failure is recorded.
- [ ] Release the model lease after worker termination/finalization.
- [ ] If an emergency abort path is retained, document that it may discard audio and keep it distinct from normal conversation stop.
- [ ] Add deterministic queue-drain tests using channels/barriers rather than sleep-only timing.
- [ ] Test shutdown with multiple queued ~100 ms chunks pending.
- [ ] Test stop racing with the worker after at least one chunk is accepted.

**Acceptance:** normal conversation stop does not lose microphone chunks that the local-ASR pipeline already accepted.

## WPR-330 — Conversation-layer finality contract

- [ ] Document the local-ASR partial/final contract at the conversation/provider boundary.
- [ ] Assert partial transcripts never commit provider user turns.
- [ ] Assert one final transcript commits at most one provider user turn.
- [ ] Assert multiple partials collapse into one final utterance.
- [ ] Assert empty/whitespace final text commits no turn.
- [ ] Assert stop-time finalization commits at most one turn.
- [ ] Add an integration-level fake/local engine regression that feeds short batch updates and fails if each batch becomes an independent user turn.
- [ ] Preserve privacy: no new raw PCM/provider payload logging in these tests or production paths.

**Acceptance:** the conversation layer is protected against recurrence of the 300 ms-finalization class of bug even if an engine regresses later.

## WPR-400 — Remove unrelated Moonshine dependency from Whisper startup

- [ ] Refactor `prepare_local_asr()` so mode-specific installers/runtime dependencies are resolved inside the selected mode branch.
- [ ] Ensure Whisper startup requires only Whisper plus shared local-ASR dependencies.
- [ ] Ensure Moonshine startup remains unchanged in behavior.
- [ ] Add a regression with Whisper dependencies available and Moonshine-specific installer state unavailable.
- [ ] Assert Whisper preparation succeeds or fails only for Whisper/shared reasons in that scenario.

**Acceptance:** selecting Whisper cannot fail with a Moonshine-installer-unavailable error when Whisper's own dependencies are valid.

## WPR-410 — Tighten Whisper FFI safety contract

- [ ] Re-evaluate `unsafe impl Send for WhisperModel` against actual worker ownership.
- [ ] Remove `unsafe impl Sync for WhisperModel` unless concurrent shared access is truly required and upstream-supported.
- [ ] If `Sync` remains, document the exact whisper.cpp guarantee and application synchronization that makes it sound.
- [ ] Keep raw whisper.cpp/C types private to the FFI module.
- [ ] Re-audit all Whisper `unsafe` blocks for lifetime, ownership, null, UTF-8/string, and thread assumptions.
- [ ] Add/update safety comments to state the invariant each `unsafe` block relies on.
- [ ] Run the repository's Rust/static safety checks after the change.

**Acceptance:** unsafe trait promises and FFI invariants are no broader than the actual production threading model.

## WPR-500 — Complete native rebuild invalidation

- [ ] Enumerate all native Whisper/ggml source and header roots that affect the linked library.
- [ ] Make `build.rs` emit `rerun-if-changed` coverage for relevant whisper headers.
- [ ] Cover relevant whisper sources.
- [ ] Cover relevant ggml headers.
- [ ] Cover relevant ggml sources.
- [ ] Cover relevant CMake/native build configuration.
- [ ] Prefer deterministic recursive enumeration or a complete explicit manifest over a small handpicked subset.
- [ ] Add a focused build-policy test/check if practical to detect omitted native source roots.

**Acceptance:** changing any native source/header used by the build causes Cargo to rerun the native build/link configuration.

## WPR-510 — Portable CPU parallelism detection

- [ ] Remove `/proc/nproc` CPU-count probing.
- [ ] Use `std::thread::available_parallelism()` or an equivalent portable API.
- [ ] Preserve a safe nonzero fallback.
- [ ] Preserve explicit operator/build override behavior if present.
- [ ] Add a focused unit/helper test where practical.

**Acceptance:** native build parallelism no longer depends on a nonexistent Linux pseudo-file and does not silently default to 4 on normal Linux hosts.

## WPR-600 — Frontend correctness and wording

- [ ] Keep Whisper Small visible as a local ASR option.
- [ ] Keep install state visible.
- [ ] Keep expected and installed byte counts visible.
- [ ] Keep actual runtime/source revision visible.
- [ ] Keep install/delete actions disabled when the active conversation/model lease makes mutation unsafe.
- [ ] Update partial-transcript disclosure to match the remediated WPR-300 behavior exactly.
- [ ] Remove generic Whisper wording that claims CRC32C verification when Whisper only uses SHA-256.
- [ ] Use model-specific or algorithm-neutral verification text.
- [ ] Preserve explicit disclosure that Whisper microphone audio remains local.
- [ ] Preserve explicit disclosure that model download is user initiated.
- [ ] Add/update frontend tests for state and wording.

**Acceptance:** the settings UI neither understates nor invents Whisper behavior, provenance, or verification algorithms.

## WPR-610 — Current documentation reconciliation

- [ ] Update `docs/WHISPER_CPP_LOCAL_ASR_SPEC.md` where the governing design changed.
- [ ] Update `docs/LOCAL_ASR_WHISPER_HANDOFF_2026-10-03.md` or add a current successor that clearly supersedes stale implementation claims.
- [ ] Reconcile `docs/PRIVACY.md` with final local partial/final behavior.
- [ ] Reconcile `docs/WHISPER_MODEL_LICENSES.md`.
- [ ] Reconcile `docs/THIRD_PARTY_NOTICES.md`.
- [ ] Reconcile `README.md` where Whisper/local-ASR behavior is described.
- [ ] Add/update `docs/WHISPER_CPP_LOCAL_ASR_PIPELINE.md` if needed to document utterance/window/endpoint/shutdown behavior.
- [ ] Add/update `docs/WHISPER_CPP_CPU_BENCHMARK.md` or equivalent final performance evidence document.
- [ ] Ensure current docs identify exact source/model provenance without rewriting historical evidence as if it came from the final SHA.
- [ ] Run documentation consistency checks applicable to the repository.

**Acceptance:** a reader can determine current Whisper source identity, model identity, privacy behavior, transcript semantics, install layout, and qualification state without contradictory documents.

## WPR-700 — Repair real-CPU acceptance workflow

- [ ] Fix the malformed indentation in `.github/workflows/whisper-real-cpu-acceptance.yml` embedded Python validation.
- [ ] Add an ordinary-CI/static check that extracts/parses/compiles the embedded acceptance validation code without downloading the real model.
- [ ] Ensure the workflow remains manually/explicitly invoked and separate from ordinary CI.
- [ ] Ensure the workflow does not depend on harness-created Whisper model directories that production code should create itself.
- [ ] Keep real model artifacts out of the repository.
- [ ] Keep ordinary CI free of real Whisper model downloads.
- [ ] Verify workflow failure paths produce useful bounded diagnostics.

**Acceptance:** the acceptance workflow can reach and execute its evidence-validation stage on a clean supported runner.

## WPR-710 — Bind acceptance evidence to actual native source and model

- [ ] Record the exact repository commit SHA under test.
- [ ] Read/record the actual `third_party/whisper.cpp` gitlink/native source revision independently of the manifest.
- [ ] Read/record the canonical expected Whisper source revision.
- [ ] Fail when actual and expected native source revisions differ.
- [ ] Record the exact downloaded model SHA-256.
- [ ] Record the exact downloaded model byte count.
- [ ] Fail when model SHA/size differ from the canonical manifest.
- [ ] Record the test-audio identity/hash where practical.
- [ ] Emit machine-readable acceptance evidence containing all of the above.
- [ ] Upload the evidence as a workflow artifact with immutable run/job identity.
- [ ] Ensure human-readable summaries are generated from verified evidence rather than unverified manifest claims.

**Acceptance:** a reviewer can prove which repository source, whisper.cpp source, model, and test audio produced the acceptance result.

## WPR-720 — Real CPU/performance evidence and tuning

- [ ] Run acceptance on the exact final production source SHA on supported Linux CPU hardware.
- [ ] Record first partial latency.
- [ ] Record final transcript latency from endpoint/finalization.
- [ ] Record real-time factor.
- [ ] Record CPU utilization using a documented sampling method.
- [ ] Record true/high-water process RSS using an OS-appropriate metric rather than sparse snapshots labeled as peak.
- [ ] Record dropped chunks under nominal load.
- [ ] Run a deliberate overload scenario and record dropped-chunk behavior.
- [ ] Record the chosen partial interval/cadence.
- [ ] Record the chosen endpoint silence threshold/hangover.
- [ ] Record maximum utterance duration/forced-finalization behavior.
- [ ] Record and justify queue capacity; change capacity only if evidence shows the current 8 x ~100 ms budget is insufficient.
- [ ] Store machine-readable raw metrics with the workflow artifact.
- [ ] Summarize measurements and any acceptance bounds in the benchmark/evidence document.
- [ ] Do not declare P5 complete until CPU and RSS evidence are present.

**Acceptance:** P5 performance/behavior claims are supported by exact-source real-CPU evidence, not workflow structure alone.

## WPR-800 — Regression coverage audit

- [ ] Source-revision mismatch regression exists and passes.
- [ ] Clean-profile install-root regression exists and passes.
- [ ] Streaming verification success/corruption regressions exist and pass.
- [ ] `ModelCorrupt` mapping regression exists and passes.
- [ ] `RuntimeUnavailable` mapping regression exists and passes.
- [ ] Stable utterance-ID/partial-update regression exists and passes.
- [ ] “300 ms cadence is not finality” regression exists and passes.
- [ ] Multiple partials -> one final regression exists and passes.
- [ ] Sub-threshold stop flush regression exists and passes.
- [ ] Accepted-queue drain on stop regression exists and passes.
- [ ] Repeated-stop/no-duplicate-final regression exists and passes.
- [ ] Empty final/no-provider-turn regression exists and passes.
- [ ] Whisper-without-Moonshine-installer regression exists and passes.
- [ ] Active model lease still blocks unsafe deletion/replacement.
- [ ] Native rebuild/provenance checks pass.
- [ ] Acceptance embedded-script syntax check runs in ordinary CI.
- [ ] Worker/lifecycle tests use deterministic synchronization where races are under test.

**Acceptance:** each reviewed failure class has a falsifiable regression that would fail if the old behavior returned.

## WPR-900 — Re-audit and reconcile the original Whisper TODO

Re-read final-source code before changing any checkbox in `docs/WHISPER_CPP_LOCAL_ASR_TODO.md`.

### P0 reconciliation

- [ ] Re-verify exact whisper.cpp source pin against actual tracked/built source.
- [ ] Re-verify model SHA/bytes/license.
- [ ] Re-verify target matrix claims against actual qualification evidence.
- [ ] Re-verify privacy/handoff documentation.

### P1 reconciliation

- [ ] Re-verify manifest source revision.
- [ ] Re-verify installer clean-profile behavior.
- [ ] Re-verify canonical model directory layout.
- [ ] Re-verify native source-change rebuild behavior.
- [ ] Re-verify FFI safety/threading claims.

### P2 reconciliation

- [ ] Re-verify utterance state tracking.
- [ ] Re-verify actual partial event emission.
- [ ] Re-verify final event semantics.
- [ ] Re-verify bounded recent/current window behavior.
- [ ] Re-verify endpoint finalization.
- [ ] Re-verify stop-time final flush delivery.
- [ ] Re-verify all documented error mappings.
- [ ] Re-verify no provider/cloud fallback.

### P3 reconciliation

- [ ] Re-verify fail-closed model preparation before microphone capture.
- [ ] Re-verify normal shutdown drains accepted PCM before worker termination.
- [ ] Re-verify wake-word mode support/fail-closed behavior.
- [ ] Re-verify model commands and progress events.
- [ ] Re-verify diagnostics report truthful source/model/runtime identity.
- [ ] Re-run/reconcile generated frontend/backend contracts if shapes changed.

### P4 reconciliation

- [ ] Mark already-implemented frontend items only after final-source re-verification.
- [ ] Verify install state/byte count/revision display.
- [ ] Verify active-conversation mutation disabling.
- [ ] Verify local/privacy/download disclosure.
- [ ] Verify partial transcript disclosure matches actual WPR-300 behavior.
- [ ] Verify README/pipeline/benchmark docs are current.

### P5 reconciliation

- [ ] Re-verify workflow syntax/execution.
- [ ] Re-verify actual native source provenance evidence.
- [ ] Re-verify model provenance evidence.
- [ ] Re-verify transcript output evidence.
- [ ] Re-verify first-partial/final latency evidence.
- [ ] Re-verify CPU evidence.
- [ ] Re-verify true/high-water RSS evidence.
- [ ] Re-verify RTF evidence.
- [ ] Re-verify dropped-chunk overload evidence.
- [ ] Re-verify window/endpoint/max-utterance/queue tuning evidence.

**Acceptance:** no checkbox in the original TODO remains checked solely because an earlier implementation note or commit message claimed completion.

## WPR-950 — Exact-head final qualification

**Exact qualification head:** _TBD_

- [ ] Reload current `master` immediately before qualification and record the exact SHA.
- [ ] Confirm all production-source remediation is present at that SHA.
- [ ] Run `npm run check:frontend`.
- [ ] Run `npm run check:rust`.
- [ ] Run `npm run check:all`.
- [ ] Run `npm run check:generated-trees`.
- [ ] Run `npm run check:generated-backend-contract`.
- [ ] Run `npm run check:tauri-command-contract`.
- [ ] Run `npm run check:frontend-contract-shapes`.
- [ ] Run `python3 scripts/check_local_llm_packaging_policy.py` when required by repository policy/changed paths.
- [ ] Run all Whisper-specific focused tests/checkers added by this remediation.
- [ ] Run the repaired real-CPU Whisper acceptance workflow on the exact qualification SHA.
- [ ] Verify all required ordinary CI runs on the exact qualification SHA are terminal and successful.
- [ ] Verify the real-CPU acceptance run is terminal and successful.
- [ ] Verify acceptance evidence records exact repository SHA, native Whisper source revision, model SHA/bytes, and test-audio identity.
- [ ] Verify CPU/RSS/latency/RTF/overload evidence is present.
- [ ] Record exact workflow/run/job/artifact identities in a final evidence document under `docs/evidence/`.
- [ ] Complete WPR-900 original-TODO reconciliation using the exact qualification source/evidence.
- [ ] Mark every mandatory WPR task/subtask complete only after evidence exists.

## WPR-960 — Exact-master closeout

**Final master SHA:** _TBD_

- [ ] Re-read current `master` after evidence/TODO reconciliation.
- [ ] Verify whether any commit after the exact qualification head changed production source.
- [ ] If production source changed, repeat WPR-950 on the new exact source SHA.
- [ ] If only docs/evidence/checklist files changed, record that fact explicitly.
- [ ] Run the repository's applicable documentation-scope/exact-master CI on final `master`.
- [ ] Verify final `master` contains the qualified production source unchanged.
- [ ] Verify `docs/WHISPER_CPP_LOCAL_ASR_TODO.md` is reconciled with final-source evidence.
- [ ] Verify this remediation TODO is reconciled with final-source evidence.
- [ ] Verify no current documentation contradicts source/model/license/runtime identity.
- [ ] Verify no mandatory finding from the 2026-10-07 Whisper review remains open.

## Final acceptance

- [ ] Built whisper.cpp source revision and reported source revision are identical and independently checked.
- [ ] Whisper model/source provenance and licenses are internally consistent.
- [ ] Clean-profile installation succeeds using production code only.
- [ ] Installed-model verification is streaming/bounded-memory and blocking-isolated.
- [ ] Error taxonomy truthfully distinguishes missing, corrupt, unavailable, load, input, inference, state, cancellation, and internal failures as applicable.
- [ ] Whisper emits partials and exactly one final per logical utterance.
- [ ] A 300 ms inference cadence does not create 300 ms provider turns.
- [ ] Normal stop drains accepted PCM and delivers final transcript output exactly once.
- [ ] Whisper startup has no dependency on Moonshine-only installer state.
- [ ] FFI unsafe trait guarantees match actual ownership/threading.
- [ ] Native source/header changes reliably trigger rebuilds.
- [ ] Build CPU parallelism detection is portable.
- [ ] Frontend and docs describe actual Whisper behavior and verification.
- [ ] Real-CPU acceptance executes successfully and verifies actual native/model provenance.
- [ ] First-partial latency, final latency, RTF, CPU, true/high-water RSS, and overload/drop evidence are recorded for the exact final source SHA.
- [ ] Every task/subtask in the original Whisper TODO has been re-audited and reconciled.
- [ ] All required exact-head and exact-master gates pass.
- [ ] Whisper local ASR can be declared production-complete with no unresolved mandatory review finding.