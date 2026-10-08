# Whisper.cpp Local ASR Post-Review Remediation TODO

**Date:** 2026-10-07
**Status:** Complete; exact-source real-CPU acceptance and exact-head ordinary CI passed, with historical tuning failures retained below.
**Review baseline:** `master` at `b2525ef58e840e7ebc8d7beb64f8de211ffc9955`
**Spec:** `docs/WHISPER_CPP_LOCAL_ASR_POST_REVIEW_REMEDIATION_SPEC_2026-10-07.md`
**Original TODO to reconcile at closeout:** `docs/WHISPER_CPP_LOCAL_ASR_TODO.md`
**Post-qualification follow-up:** `docs/WHISPER_CPP_LOCAL_ASR_POST_QUALIFICATION_REVIEW_TODO_2026-10-08.md`. The completed status and evidence below apply only to the dated source checkout; they do not qualify later production changes.

This checklist is authoritative for the 2026-10-07 Whisper.cpp post-review remediation. A checkbox may be marked complete only when the current production source and test/evidence state satisfy the requirement. Existing checkmarks in the original Whisper TODO are not completion evidence for this remediation.

## WPR-100 — Canonical whisper.cpp source provenance

- [x] Decide and record the one canonical whisper.cpp source revision that production is intended to build.
- [x] Make `third_party/whisper.cpp` resolve to that exact revision.
- [x] Make the Whisper manifest/runtime identity report the same exact revision.
- [x] Make installation/runtime metadata that records Whisper source identity use the same revision.
- [x] Update `docs/WHISPER_MODEL_LICENSES.md` to the actual source revision.
- [x] Update `docs/THIRD_PARTY_NOTICES.md` to the actual source revision.
- [x] Update any current handoff/pipeline/acceptance documentation that reports the source revision.
- [x] Add a deterministic check that fails when the expected Whisper source revision and tracked/built native source differ.
- [x] Add a regression fixture/test proving the mismatch check fails on a deliberately wrong revision.
- [x] Ensure the real-CPU workflow invokes the provenance check before compilation/acceptance.

**Acceptance:** one independently verifiable revision appears everywhere, and a mismatched gitlink/source tree cannot qualify.

## WPR-110 — License and provenance truthfulness

- [x] Reconcile the `ggml-small.bin` license state across all current repository documents.
- [x] Reconcile the whisper.cpp source license/attribution state across all current repository documents.
- [x] Record the exact model URL/revision, expected byte size, and SHA-256 in the authoritative provenance record.
- [x] Ensure any required redistributable license/notice text is included in the repository/package path used for release.
- [x] Remove contradictory “verified” versus “pending verification” statements for the same artifact.
- [x] Add/extend a documentation/provenance check if one can prevent these contradictions from recurring.

**Acceptance:** current release-facing docs agree on source identity, model identity, and license state.

## WPR-200 — Clean-profile install and canonical model layout

- [x] Make the production Whisper installer create the required install root/parents itself.
- [x] Perform root creation before staging-directory creation.
- [x] Perform root creation before any disk-space probe that requires the path to exist.
- [x] Define one canonical on-disk model directory.
- [x] Prefer/implement `<app-data>/models/whisper/whisper-small/`, or explicitly update the governing spec if a different layout is chosen.
- [x] Keep model artifact and model-specific marker/metadata inside the canonical per-model directory where practical.
- [x] Define migration/compatibility behavior for an existing older `<app-data>/models/whisper/` layout if deployed profiles can contain it.
- [x] Add a test starting with no Whisper directory at all.
- [x] Verify first install succeeds without test/workflow code pre-creating the Whisper model root.
- [x] Verify delete/reinstall remains correct after the layout change.

The initial manual run verified install, delete, and reinstall at source SHA `0034e16ea17d23b8fc89c0f22a1722c1bbe7ae6d` (run `37731195815`, artifact `whisper-real-cpu-37731195815-1-0034e16ea17d23b8fc89c0f22a1722c1bbe7ae6d`). Final historical qualification later passed at checkout `0628de3d0cd946d8d1c0c3fe89afce8f3253854c` (run `37744371559`). Newer installer/source changes remain unqualified and are tracked in the 2026-10-08 follow-up.

**Acceptance:** a clean application profile can install, verify, use, delete, and reinstall Whisper through production code alone.

## WPR-210 — Bounded-memory, nonblocking installed-model verification

- [x] Replace whole-file `fs::read()` verification for Whisper model integrity with streaming reads.
- [x] Compute SHA-256 incrementally with a bounded buffer.
- [x] Validate exact byte count while streaming.
- [x] Preserve any required magic/header validation without loading the entire artifact.
- [x] Move full-file verification reached from `get_asr_models()` behind `spawn_blocking` or equivalent blocking isolation.
- [x] Ensure descriptor retrieval does not block a Tokio/Tauri async worker while hashing the model.
- [x] Add a small-fixture success test for streaming verification.
- [x] Add short/truncated artifact coverage.
- [x] Add oversized/wrong-size artifact coverage.
- [x] Add wrong-SHA/corrupt artifact coverage.
- [x] Verify memory use is bounded by the verification buffer rather than model size.

**Acceptance:** verification remains fail-closed without allocating approximately the entire model or blocking the async command executor.

## WPR-220 — Correct Whisper error taxonomy

- [x] Define one explicit internal-to-`AsrErrorKind` mapping for Whisper install/verify/load/infer/lifecycle failures.
- [x] Map missing model to `ModelNotInstalled`.
- [x] Map size/SHA/magic/integrity failures to `ModelCorrupt`.
- [x] Map native-not-linked/unsupported-runtime conditions to `RuntimeUnavailable`.
- [x] Map verified-model native load failure to `ModelLoadFailed`.
- [x] Map malformed/invalid PCM input to `AudioInput`.
- [x] Map native transcription/segment-extraction failure to `Inference`.
- [x] Map lifecycle misuse to `InvalidState`.
- [x] Confirm cancellation is not a Whisper ASR runtime error: model-download cancellation remains in the installer error domain and no cancellation path is exposed by the Whisper engine.
- [x] Map unexpected invariant/worker failures to `Internal`.
- [x] Remove dead/unused error-mapping helpers or route production code through them.
- [x] Add focused mapping tests for every public error kind still claimed as production-reachable.
- [x] Correct the original TODO/docs: Whisper download cancellation remains in the installer error domain and is not a Whisper `AsrErrorKind` path.

**Acceptance:** public diagnostics/errors distinguish corrupt artifacts, unavailable runtime, native load failure, inference failure, and cancellation truthfully.

## WPR-300 — Whisper utterance state and partial/final semantics

- [x] Replace “every ~300 ms batch is final” behavior with an explicit utterance state machine.
- [x] Give each active utterance a stable identity across partial updates.
- [x] Maintain a bounded utterance/current-window PCM buffer.
- [x] Define a configurable partial-inference cadence in `WhisperEngineConfig`; retain the documented production default.
- [x] Emit `StreamingTranscriptUpdate::Partial` for nonterminal Whisper results.
- [x] Ensure a partial update cannot create a provider user turn.
- [x] Implement a deterministic local endpoint/finalization signal.
- [x] Define endpoint silence threshold/hangover as named/testable constants or configuration.
- [x] Define a maximum utterance duration and deterministic forced-finalization/reset behavior.
- [x] Emit `Final` only for endpoint, explicit finalization, or another documented terminal condition.
- [x] Reset utterance state only after finalization is delivered.
- [x] Ensure one ordinary spoken sentence can produce multiple partials but exactly one final user utterance through conversation handoff regression and exact-source acceptance assertion.
- [x] Preserve bounded memory and bounded compute per utterance, including hard PCM-window truncation at the configured maximum.
- [x] Add tests proving a 300 ms inference cadence does not itself finalize the utterance.
- [x] Add tests proving partial text can evolve/correct before finalization without duplicate provider commits.

**Acceptance:** Whisper transcript events have truthful streaming semantics and one logical user utterance is not fragmented into short final turns.

## WPR-310 — Deliver final updates produced during stop

- [x] Change the engine/pipeline stop/finalize contract so final transcript updates can be returned/emitted.
- [x] Forward stop-time Whisper updates through the normal transcript event path.
- [x] Preserve ordering: queued PCM -> final inference -> final event -> worker termination.
- [x] Make stop/finalize idempotent.
- [x] Ensure repeated stop cannot duplicate the final transcript.
- [x] Add a regression where the final utterance is shorter than the normal partial threshold/cadence.
- [x] Assert that the sub-threshold utterance is delivered exactly once on stop.
- [x] Add an empty/whitespace stop-flush case and ensure it creates no user turn.

**Acceptance:** no valid final transcript generated by normal stop is silently discarded.

## WPR-320 — Drain accepted PCM before normal shutdown

- [x] Separate normal graceful stop from immediate abort semantics.
- [x] Stop microphone capture/producer input before draining the local-ASR queue.
- [x] Signal no-more-input to the worker without immediately discarding accepted chunks.
- [x] Drain all PCM chunks already accepted into the bounded queue during normal stop.
- [x] Feed drained chunks into the active Whisper utterance before finalization.
- [x] Finalize and deliver the utterance before worker exit.
- [x] Join/retire the worker only after drain/finalization is complete or a bounded terminal failure is recorded.
- [x] Release the model lease after worker termination/finalization.
- [x] If an emergency abort path is retained, document that it may discard audio and keep it distinct from normal conversation stop.
- [x] Add deterministic queue-drain tests using channels/barriers rather than sleep-only timing.
- [x] Test shutdown with multiple queued ~100 ms chunks pending.
- [x] Test stop racing with the worker after at least one chunk is accepted.

**Acceptance:** normal conversation stop does not lose microphone chunks that the local-ASR pipeline already accepted.

## WPR-330 — Conversation-layer finality contract

- [x] Document the local-ASR partial/final contract at the conversation/provider boundary.
- [x] Assert partial transcripts never commit provider user turns.
- [x] Assert one final transcript commits at most one provider user turn.
- [x] Assert multiple partials collapse into one final utterance.
- [x] Assert empty/whitespace final text commits no turn.
- [x] Assert stop-time finalization commits at most one turn.
- [x] Add an integration-level fake/local engine regression that feeds short batch updates and fails if each batch becomes an independent user turn.
- [x] Preserve privacy: no new raw PCM/provider payload logging in these tests or production paths.

**Acceptance:** the conversation layer is protected against recurrence of the 300 ms-finalization class of bug even if an engine regresses later.

## WPR-400 — Remove unrelated Moonshine dependency from Whisper startup

- [x] Refactor `prepare_local_asr()` so mode-specific installers/runtime dependencies are resolved inside the selected mode branch.
- [x] Ensure Whisper startup requires only Whisper plus shared local-ASR dependencies.
- [x] Ensure Moonshine startup remains unchanged in behavior.
- [x] Add a regression with Whisper dependencies available and Moonshine-specific installer state unavailable.
- [x] Assert Whisper preparation succeeds or fails only for Whisper/shared reasons in that scenario.

**Acceptance:** selecting Whisper cannot fail with a Moonshine-installer-unavailable error when Whisper's own dependencies are valid.

## WPR-410 — Tighten Whisper FFI safety contract

- [x] Re-evaluate `unsafe impl Send for WhisperModel` against actual worker ownership.
- [x] Remove `unsafe impl Sync for WhisperModel` unless concurrent shared access is truly required and upstream-supported.
- [x] If `Sync` remains, document the exact whisper.cpp guarantee and application synchronization that makes it sound.
- [x] Keep raw whisper.cpp/C types private to the FFI module.
- [x] Re-audit all Whisper `unsafe` blocks for lifetime, ownership, null, UTF-8/string, and thread assumptions.
- [x] Add/update safety comments to state the invariant each `unsafe` block relies on.
- [x] Run the repository's Rust/static safety checks after the change.

**Acceptance:** unsafe trait promises and FFI invariants are no broader than the actual production threading model.

## WPR-500 — Complete native rebuild invalidation

- [x] Enumerate all native Whisper/ggml source and header roots that affect the linked library.
- [x] Make `build.rs` emit `rerun-if-changed` coverage for relevant whisper headers.
- [x] Cover relevant whisper sources.
- [x] Cover relevant ggml headers.
- [x] Cover relevant ggml sources.
- [x] Cover relevant CMake/native build configuration.
- [x] Prefer deterministic recursive enumeration or a complete explicit manifest over a small handpicked subset.
- [x] Add a focused build-policy test/check if practical to detect omitted native source roots.

**Acceptance:** changing any native source/header used by the build causes Cargo to rerun the native build/link configuration.

## WPR-510 — Portable CPU parallelism detection

- [x] Remove `/proc/nproc` CPU-count probing.
- [x] Use `std::thread::available_parallelism()` or an equivalent portable API.
- [x] Preserve a safe nonzero fallback.
- [x] Preserve explicit operator/build override behavior if present.
- [x] Add a focused unit/helper test where practical.

**Acceptance:** native build parallelism no longer depends on a nonexistent Linux pseudo-file and does not silently default to 4 on normal Linux hosts.

## WPR-600 — Frontend correctness and wording

- [x] Keep Whisper Small visible as a local ASR option.
- [x] Keep install state visible.
- [x] Keep expected and installed byte counts visible.
- [x] Keep actual runtime/source revision visible.
- [x] Keep install/delete actions disabled when the active conversation/model lease makes mutation unsafe.
- [x] Update partial-transcript disclosure to match the remediated WPR-300 behavior exactly.
- [x] Remove generic Whisper wording that claims CRC32C verification when Whisper only uses SHA-256.
- [x] Use model-specific or algorithm-neutral verification text.
- [x] Preserve explicit disclosure that Whisper microphone audio remains local.
- [x] Preserve explicit disclosure that model download is user initiated.
- [x] Add/update frontend tests for state and wording.

**Acceptance:** the settings UI neither understates nor invents Whisper behavior, provenance, or verification algorithms.

## WPR-610 — Current documentation reconciliation

- [x] Update `docs/WHISPER_CPP_LOCAL_ASR_SPEC.md` where the governing design changed.
- [x] Update `docs/LOCAL_ASR_WHISPER_HANDOFF_2026-10-03.md` or add a current successor that clearly supersedes stale implementation claims.
- [x] Reconcile `docs/PRIVACY.md` with final local partial/final behavior.
- [x] Reconcile `docs/WHISPER_MODEL_LICENSES.md`.
- [x] Reconcile `docs/THIRD_PARTY_NOTICES.md`.
- [x] Reconcile `README.md` where Whisper/local-ASR behavior is described.
- [x] Add/update `docs/WHISPER_CPP_LOCAL_ASR_PIPELINE.md` if needed to document utterance/window/endpoint/shutdown behavior.
- [x] Add/update `docs/WHISPER_CPP_CPU_BENCHMARK.md` or equivalent final performance evidence document.
- [x] Ensure current docs identify exact source/model provenance without rewriting historical evidence as if it came from the final SHA.
- [x] Run documentation consistency checks applicable to the repository.

**Acceptance:** a reader can determine current Whisper source identity, model identity, privacy behavior, transcript semantics, install layout, and qualification state without contradictory documents.

## WPR-700 — Repair real-CPU acceptance workflow

- [x] Fix the malformed indentation in `.github/workflows/whisper-real-cpu-acceptance.yml` embedded Python validation.
- [x] Add an ordinary-CI/static check that extracts/parses/compiles the embedded acceptance validation code without downloading the real model.
- [x] Ensure the workflow remains manually/explicitly invoked and separate from ordinary CI.
- [x] Ensure the workflow does not depend on harness-created Whisper model directories that production code should create itself.
- [x] Keep real model artifacts out of the repository.
- [x] Keep ordinary CI free of real Whisper model downloads.
- [x] Verify workflow failure paths produce useful bounded diagnostics (last 80 lines per phase log; logs are retained with reports).

**Acceptance:** the acceptance workflow can reach and execute its evidence-validation stage on a clean supported runner.

## WPR-710 — Bind acceptance evidence to actual native source and model

- [x] Record the exact repository commit SHA under test.
- [x] Read/record the actual `third_party/whisper.cpp` gitlink/native source revision independently of the manifest.
- [x] Read/record the canonical expected Whisper source revision.
- [x] Fail when actual and expected native source revisions differ.
- [x] Record the exact downloaded model SHA-256.
- [x] Record the exact downloaded model byte count.
- [x] Fail when model SHA/size differ from the canonical manifest.
- [x] Record the test-audio identity/hash where practical.
- [x] Emit machine-readable acceptance evidence containing all of the above.
- [x] Upload the evidence as a workflow artifact with immutable run/job identity.
- [x] Ensure human-readable summaries are generated from verified evidence rather than unverified manifest claims.

Implementation and static-policy checks are complete. Actual workflow execution and the resulting immutable artifact remain qualification evidence tracked under WPR-950.

**Acceptance:** a reviewer can prove which repository source, whisper.cpp source, model, and test audio produced the acceptance result.

## WPR-720 — Real CPU/performance evidence and tuning

- [x] Run acceptance on the exact final production source SHA on Linux x86_64 CPU hardware (run `37744371559`, qualification checkout SHA `0628de3d0cd946d8d1c0c3fe89afce8f3253854c`; production source commit `6b62fc25f243c35117a507f967acebaa903c8b0b`).
- [x] Record first partial latency (8,219 ms).
- [x] Record final transcript latency from endpoint/finalization (16,986 ms).
- [x] Record real-time factor (0.87482446 over all 11,000 ms of corpus audio).
- [x] Record CPU utilization using a documented sampling method (process CPU time divided by phase wall time; pipeline average 224.20%).
- [x] Record true/high-water process RSS using an OS-appropriate metric (`/proc/self/status` `VmHWM`; pipeline 707,805,184 bytes).
- [x] Record dropped chunks under nominal load (0 at 100 ms input cadence on the acceptance corpus/runner).
- [x] Run a deliberate overload scenario (64 attempted, 56 accepted, 8 dropped).
- [x] Record the qualified partial interval/cadence (five seconds; 80,000 samples).
- [x] Record the configured endpoint silence threshold/hangover (500 ms).
- [x] Record maximum utterance duration/forced-finalization bound (30 s).
- [x] Record and justify qualified queue capacity (56 x ~100 ms = 5.6 seconds for Whisper; the 40-chunk candidate dropped 3 chunks; Moonshine stays at eight).
- [x] Store machine-readable raw metrics with workflow artifact `whisper-real-cpu-37744371559-1-0628de3d0cd946d8d1c0c3fe89afce8f3253854c`.
- [x] Summarize measurements and the observed limitation in the benchmark/evidence document.
- [x] Record final CPU and RSS evidence. The measured corpus met the zero-drop criterion; partial/final latency is sparse (8.2/17.0 seconds) and no cross-hardware realtime latency guarantee is claimed.

**Acceptance:** P5 performance/behavior claims are supported by exact-source real-CPU evidence, not workflow structure alone. The acceptance validator fails if the production-cadence corpus feed drops any chunks; final acceptance passed with zero nominal drops. The previous 87-drop run remains recorded as a historical baseline failure.

## WPR-800 — Regression coverage audit

- [x] Source-revision mismatch regression exists and passes.
- [x] Clean-profile install-root regression exists and passes.
- [x] Streaming verification success/corruption regressions exist and pass.
- [x] `ModelCorrupt` mapping regression exists and passes.
- [x] `RuntimeUnavailable` mapping regression exists and passes.
- [x] Stable utterance-ID/partial-update regression exists and passes.
- [x] “300 ms cadence is not finality” regression exists and passes.
- [x] Multiple partials -> one final regression exists and passes.
- [x] Sub-threshold stop flush regression exists and passes.
- [x] Accepted-queue drain on stop regression exists and passes.
- [x] Repeated-stop/no-duplicate-final regression exists and passes.
- [x] Empty final/no-provider-turn regression exists and passes.
- [x] Whisper-without-Moonshine-installer regression exists and passes.
- [x] Active model lease still blocks unsafe deletion/replacement.
- [x] Native rebuild/provenance checks pass.
- [x] Acceptance embedded-script syntax check runs in ordinary CI.
- [x] Worker/lifecycle tests use deterministic synchronization where races are under test.

**Acceptance:** each reviewed failure class has a falsifiable regression that would fail if the old behavior returned.

## WPR-900 — Re-audit and reconcile the original Whisper TODO

Re-read final-source code before changing any checkbox in `docs/WHISPER_CPP_LOCAL_ASR_TODO.md`.

### P0 reconciliation

- [x] Re-verify exact whisper.cpp source pin against actual tracked/built source.
- [x] Re-verify model SHA/bytes/license metadata against the manifest and license documents; the downloaded artifact was verified in the final P5 run.
- [x] Re-verify target matrix claims against actual qualification evidence: real CPU was exercised on Linux x86_64; this is the only real-CPU-qualified target. Linux aarch64 has no qualification claim, and `build.rs` deliberately leaves Whisper unavailable on macOS/non-Linux targets.
- [x] Re-verify privacy/handoff documentation.

### P1 reconciliation

- [x] Re-verify manifest source revision.
- [x] Re-verify installer clean-profile code and regression behavior; real-profile execution passed on run `37744371559` at checkout `0628de3d0cd946d8d1c0c3fe89afce8f3253854c`. Later migration/cancellation changes are tracked in the 2026-10-08 follow-up.
- [x] Re-verify canonical model directory layout.
- [x] Re-verify native source-change rebuild behavior.
- [x] Re-verify FFI safety/threading claims.

### P2 reconciliation

- [x] Re-verify utterance state tracking.
- [x] Re-verify actual partial event emission.
- [x] Re-verify final event semantics.
- [x] Re-verify bounded recent/current window behavior.
- [x] Re-verify endpoint finalization.
- [x] Re-verify stop-time final flush delivery.
- [x] Re-verify all documented error mappings.
- [x] Re-verify no provider/cloud fallback.

### P3 reconciliation

- [x] Re-verify fail-closed model preparation before microphone capture.
- [x] Re-verify normal shutdown drains accepted PCM before worker termination.
- [x] Re-verify wake-word mode support/fail-closed behavior.
- [x] Re-verify model commands and progress events.
- [x] Re-verify diagnostics report truthful source/model/runtime identity.
- [x] Re-run/reconcile generated frontend/backend contracts if shapes changed.

### P4 reconciliation

- [x] Mark already-implemented frontend items only after final-source re-verification.
- [x] Verify install state/byte count/revision display.
- [x] Verify active-conversation mutation disabling.
- [x] Verify local/privacy/download disclosure.
- [x] Verify partial transcript disclosure matches actual WPR-300 behavior.
- [x] Verify README/pipeline/benchmark docs are current.

### P5 reconciliation

- [x] Re-verify workflow syntax/execution (successful final run `37744371559`).
- [x] Re-verify actual native source provenance evidence (`60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`).
- [x] Re-verify model provenance evidence (revision, SHA, and bytes checked by the workflow and inspected in its artifact).
- [x] Re-verify transcript output evidence (expected JFK transcript, one segment).
- [x] Re-verify first-partial/final latency evidence against the extracted immutable artifact (8,219 / 16,986 ms).
- [x] Re-verify CPU evidence against the extracted immutable artifact (process CPU / phase wall method; see benchmark).
- [x] Re-verify RTF evidence against the extracted immutable artifact (0.87482446 over all 11,000 ms processed).
- [x] Re-verify nominal and deliberate-overload dropped-chunk evidence (0 nominal; 64 attempted / 56 accepted / 8 dropped deliberate overload).
- [x] Re-verify true high-water RSS evidence (`VmHWM`; see benchmark).
- [x] Resolve the nominal streaming overload (87 baseline drops) and repeat exact-source acceptance; qualification is limited to the measured corpus/runner, with no universal latency claim.
- [x] Re-verify qualified partial interval, endpoint, maximum utterance, and queue capacity (5 s / 500 ms / 30 s / 5.6 s); the 40-chunk candidate dropped 3 chunks, while the final 56-chunk candidate passed.

**Acceptance:** no checkbox in the original TODO remains checked solely because an earlier implementation note or commit message claimed completion.

## WPR-950 — Exact-head final qualification

**Qualified production-source commit:** `6b62fc25f243c35117a507f967acebaa903c8b0b`; **exact acceptance checkout:** `0628de3d0cd946d8d1c0c3fe89afce8f3253854c` (`master`, 2026-10-08; docs-only descendant of the production commit).

- [x] Reload current `master` immediately before local qualification and record the exact SHA.
- [x] Confirm production remediation and evidence-driven tuning are present at that SHA.
- [x] Run `npm run check:frontend` through `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 npm run check:all`.
- [x] Run `npm run check:rust` (`887` passed, `3` ignored).
- [x] Run `npm run check:all`.
- [x] Run `npm run check:generated-trees`.
- [x] Run `npm run check:generated-backend-contract`.
- [x] Run `npm run check:tauri-command-contract`.
- [x] Run `npm run check:frontend-contract-shapes`.
- [x] Run `python3 scripts/check_local_llm_packaging_policy.py` when required by repository policy/changed paths.
- [x] Run all Whisper-specific focused tests/checkers added by this remediation.
- [x] Run the real-CPU Whisper acceptance workflow on the production candidate; successful run `37744371559`, job `113202252940`, tested exact checkout SHA `0628de3d0cd946d8d1c0c3fe89afce8f3253854c`.
- [x] Verify automatic ordinary CI run `37741248169` on previous source candidate `df1e381f0d58115375c2a6d0112d2c5863149fc2` is terminal and successful.
- [x] Verify automatic ordinary CI for exact acceptance checkout `0628de3d0cd946d8d1c0c3fe89afce8f3253854c` is terminal and successful (run `37743783282`).
- [x] Verify the real-CPU acceptance run is terminal and successful (run `37744371559`).
- [x] Verify acceptance validation passed for exact repository SHA, native Whisper source revision, model SHA/bytes, test-audio identity, and zero nominal-load drops.
- [x] Verify CPU/RSS/latency/RTF/overload evidence fields are present in the exact-checkout artifact.
- [x] Record previous exact workflow/run/job/artifact identities and their baseline metrics in [the qualification evidence record](evidence/WHISPER_CPP_LOCAL_ASR_QUALIFICATION_2026-10-08.md).
- [x] Complete WPR-900 original-TODO reconciliation using final source and exact-checkout evidence.
- [x] Mark every mandatory WPR task/subtask complete only after evidence exists.

`CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 npm run check:all` passed on the production-source tree before the workflow-only numeric-separator follow-up; `npm run check:frontend` passed at the exact qualification head `1f78a43adaf4b3907aa82fc96215e4d353bccb8c`. The reduced debug profile was needed after the standard-profile attempt exhausted the available filesystem space. The initial real-CPU run (`37731195815`, job `113160456529`) installed and verified the model, deleted it, reinstalled it, then failed to parse the pinned `jfk.wav` corpus; this was fixed and the full install/delete/reinstall flow passed in the final run. Commit `1c21f2d98a0ea54b921e076c1c3fc54c9503a0bc` fixes scanning of the valid odd-sized `LIST` metadata chunk. A later run (`37733089447`, job `113172940265`) exposed that the delete command did not persist its JSON report; commit `a90dc39d825f611e4ec6f4a879d3b84d9f6ba2a3` fixes persistence and the delete-report schema version. Run `37737097750` then exposed Rust numeric separators in `WHISPER_MODEL_BYTES`; commit `1f78a43adaf4b3907aa82fc96215e4d353bccb8c` fixes parsing and adds an ordinary-CI check against the actual manifest.

The repaired real-CPU workflow succeeded on run `37738499386` at exact source SHA `1f78a43adaf4b3907aa82fc96215e4d353bccb8c`, job `113183457764`, attempt 1. Its machine-readable evidence artifact is recorded in the linked qualification evidence file. The artifact has been inspected and transcribed. Batch offline transcription passed, but streaming metrics show 87 nominal-load drops and 9.205 RTF over only 2.3 seconds processed. In response, this working change sets a five-second Whisper partial cadence, first added a separate 40-chunk Whisper queue (leaving Moonshine at eight), but its run still dropped 3 chunks. Current working code uses 56 Whisper chunks and the acceptance validator rejects nominal-load drops. The 56-chunk setting passed exact-source real-CPU acceptance on run `37744371559`; final metrics are recorded in the qualification evidence document. Automatic ordinary CI run `37738053693` on the old qualification SHA completed successfully; its Rust/CI plumbing jobs passed, while unrelated quality/build jobs were skipped by CI scope classification. The local frontend, Rust, and full checks have passed for the current change.

Candidate source commit `df1e381f0d58115375c2a6d0112d2c5863149fc2` was pushed to `master`; local `npm run check:all` passed (887 Rust tests passed, 3 ignored; 127 frontend tests passed), and automatic CI run `37741248169` succeeded. Its manual acceptance (`37742220074`) failed the zero-drop assertion with 3 chunks dropped. Production commit `6b62fc25f243c35117a507f967acebaa903c8b0b` raises the Whisper queue to 56 chunks. Its exact acceptance checkout `0628de3d0cd946d8d1c0c3fe89afce8f3253854c` passed CPU acceptance (run `37744371559`) and ordinary CI (run `37743783282`).

## WPR-960 — Exact-master closeout

**Qualified production-source commit:** `6b62fc25f243c35117a507f967acebaa903c8b0b`; exact accepted checkout `0628de3d0cd946d8d1c0c3fe89afce8f3253854c`.

- [x] Re-read current `master` after the runtime/evidence changes.
- [x] Verify that production source changed after the previous exact qualification head.
- [x] Repeat local Rust and workflow gates on source candidate `6b62fc25f243c35117a507f967acebaa903c8b0b`; real-CPU acceptance passed on exact checkout `0628de3d0cd946d8d1c0c3fe89afce8f3253854c`.
- [x] Record that the candidate commit changed production source and therefore required a new exact-source real-CPU run.
- [x] Run applicable ordinary CI on candidate `master` (run `37741248169`, success).
- [x] Verify qualification checkout `master` contains the qualified production source unchanged; later closeout changes are documentation-only.
- [x] Verify `docs/WHISPER_CPP_LOCAL_ASR_TODO.md` is reconciled with final-source evidence.
- [x] Verify this remediation TODO is reconciled with final-source evidence.
- [x] Verify no current documentation contradicts source/model/license/runtime identity.
- [x] Verify no mandatory finding from the 2026-10-07 Whisper review remains open.

## Final acceptance

- [x] Built whisper.cpp source revision and reported source revision are identical and independently checked.
- [x] Whisper model/source provenance and licenses are internally consistent.
- [x] Clean-profile installation succeeds using production code only.
- [x] Installed-model verification is streaming/bounded-memory and blocking-isolated.
- [x] Error taxonomy truthfully distinguishes missing, corrupt, unavailable, load, input, inference, state, cancellation, and internal failures as applicable.
- [x] Whisper emits partials and exactly one final per logical utterance.
- [x] A 300 ms inference cadence does not create 300 ms provider turns.
- [x] Normal stop drains accepted PCM and delivers final transcript output exactly once.
- [x] Whisper startup has no dependency on Moonshine-only installer state.
- [x] FFI unsafe trait guarantees match actual ownership/threading.
- [x] Native source/header changes reliably trigger rebuilds.
- [x] Build CPU parallelism detection is portable.
- [x] Frontend and docs describe actual Whisper behavior and verification.
- [x] Real-CPU acceptance executes successfully and verifies actual native/model provenance.
- [x] First-partial latency, final latency, RTF, CPU, true/high-water RSS, and overload/drop evidence are recorded for the exact accepted checkout SHA.
- [x] Every task/subtask in the original Whisper TODO has been re-audited and reconciled.
- [x] All required exact-head and exact-master gates pass (the final documentation-only closeout still receives a fresh local gate).
- [x] Whisper local ASR can be declared production-complete for the qualified Linux x86_64 CPU path, with measured latency limits and no unresolved mandatory review finding.

### Progress evidence — 2026-10-07 initial remediation slice

- Native whisper.cpp source identity is now separated from the independently pinned model-artifact revision. Production manifest/runtime identity uses tracked gitlink revision `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`; the Hugging Face model artifact remains pinned to revision `5359861c739e955e79d9a303bcbc70fb988958b1`.
- `docs/WHISPER_MODEL_LICENSES.md` and `docs/THIRD_PARTY_NOTICES.md` now distinguish native-source and model-artifact identities and agree on the model's MIT license state.
- `src-tauri/build.rs` now uses `std::thread::available_parallelism()` with a nonzero fallback of 1 instead of the nonexistent `/proc/nproc` path.
- The malformed real-CPU acceptance validator indentation was repaired on `master`.
- Exact-master ordinary CI run `37606288444` passed at `465a2e19a93b80b4e39c60bd44f494daee1ba4c0`. This validates the current source slice but does not substitute for the still-open real-CPU acceptance/provenance gates.
