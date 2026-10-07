# Whisper.cpp Local ASR Post-Review Remediation Spec

**Date:** 2026-10-07
**Status:** Draft remediation specification
**Review baseline:** `master` at `b2525ef58e840e7ebc8d7beb64f8de211ffc9955`
**Review source:** comprehensive static review plus targeted executable checks of the 2026-10-07 master snapshot
**Original implementation spec:** `docs/WHISPER_CPP_LOCAL_ASR_SPEC.md`
**Original implementation TODO:** `docs/WHISPER_CPP_LOCAL_ASR_TODO.md`
**Companion remediation TODO:** `docs/WHISPER_CPP_LOCAL_ASR_POST_REVIEW_REMEDIATION_TODO_2026-10-07.md`

## 1. Purpose

This specification defines the remediation required for issues found during the 2026-10-07 review of the Whisper.cpp local-ASR implementation on current `master`.

The implementation has a sound overall architecture: Whisper is isolated behind the local-ASR abstraction, microphone PCM remains local, model installation is explicit, local inference uses a bounded worker queue, verified model leases protect active artifacts, and the conversation layer does not silently fall back to Gemini Live, Moonshine, or a fake provider. Those strengths must be preserved.

The review nevertheless found release-blocking defects in source provenance, utterance/finalization semantics, shutdown draining, clean-profile installation, and the real-CPU acceptance workflow. It also found error-taxonomy, verification-performance, build invalidation, FFI-contract, documentation, and TODO-reconciliation defects. The existing `docs/WHISPER_CPP_LOCAL_ASR_TODO.md` therefore must not be treated as authoritative evidence of completion until this remediation is implemented and the original checklist is reconciled against final-source evidence.

## 2. Goals

The remediation must achieve all of the following:

1. Make the whisper.cpp source revision reported by manifests, diagnostics, documentation, CI, and runtime exactly match the source actually built.
2. Make Whisper transcription operate on coherent utterances rather than treating every approximately 300 ms batch as an independent final user turn.
3. Emit truthful partial and final transcript events with stable utterance identity.
4. Guarantee that normal stop/shutdown drains already-accepted PCM and delivers the final transcript instead of discarding it.
5. Make first-time model installation succeed from a clean application-data directory and make the on-disk model layout match the documented contract.
6. Preserve model-integrity verification while avoiding whole-model memory reads and async-runtime blocking.
7. Make Whisper-specific failures map to the documented `AsrErrorKind` taxonomy.
8. Remove unnecessary concurrency promises from the Whisper FFI wrapper and keep unsafe boundaries minimal.
9. Make native rebuild invalidation and host CPU-count detection correct.
10. Repair the real-CPU acceptance workflow so it is executable, provenance-bound, and capable of producing trustworthy performance evidence.
11. Make frontend wording and repository documentation describe actual behavior and actual provenance.
12. Reconcile every checkbox in the original Whisper TODO against final implementation and immutable evidence.
13. Finish with exact-head and exact-master qualification; documentation-only closeout commits must not silently invalidate source qualification.

## 3. Non-goals

- Do not replace whisper.cpp with a different ASR engine as part of this remediation.
- Do not weaken local-ASR privacy boundaries or add a network fallback.
- Do not commit `ggml-small.bin` or other real Whisper model artifacts into the repository.
- Do not add real-model downloads to ordinary CI.
- Do not redesign Moonshine unless a shared local-ASR abstraction must change to preserve correct semantics for both engines.
- Do not hide model or source mismatch behind documentation changes; runtime/build provenance must be correct first.
- Do not mark partial-transcript requirements complete by renaming final events or by emitting duplicate final text as “partial.”
- Do not count a workflow file as acceptance evidence unless that workflow successfully executes on the exact qualifying source SHA.
- Do not create branches/PRs solely to subdivide checklist items when execution is explicitly being performed directly on `master`.

## 4. Severity and release gates

The following findings are release blockers for the Whisper local-ASR path:

- **WPR-100:** whisper.cpp source revision mismatch.
- **WPR-300:** 300 ms batches are emitted as final transcript turns instead of coherent utterances.
- **WPR-310:** final stop-time transcript updates are discarded.
- **WPR-320:** queued PCM can be abandoned during shutdown.
- **WPR-200:** fresh model installation can fail because the install root is not guaranteed to exist.
- **WPR-700:** real-CPU acceptance validation contains invalid Python indentation and cannot currently complete its evidence-validation step.

Whisper local ASR must not be declared production-complete until all release blockers and all mandatory findings in the companion TODO are closed.

## 5. Preserved invariants

Every implementation choice made under this remediation must preserve these existing guarantees:

- Whisper ASR remains local; raw microphone PCM is not sent to Google merely because Whisper is selected.
- `AsrMode::WhisperSmall` remains a local-ASR mode.
- Model installation remains explicit and user initiated.
- The model is verified before it can be leased for inference.
- Active model leases prevent unsafe deletion/replacement while in use.
- Audio capture callbacks remain nonblocking.
- The local-ASR producer/consumer queue remains bounded and exposes dropped-chunk diagnostics.
- Failures remain fail-closed. No automatic fallback to Gemini Live, Moonshine, or a fake provider is permitted.
- Unsafe whisper.cpp interaction remains contained behind a Rust wrapper; raw C types do not spread through application code.
- Ordinary CI remains real-model-free.
- Logs and diagnostics must not expose microphone PCM, provider secrets, access tokens, or private transcript payloads beyond existing privacy policy.

## 6. Findings and required design

### WPR-100 — Canonical whisper.cpp source provenance

At the review baseline, `src-tauri/src/asr/whisper/manifest.rs` reports whisper.cpp source commit:

`5359861c739e955e79d9a303bcbc70fb988958b1`

but the tracked `third_party/whisper.cpp` gitlink resolves to:

`60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`

The build consumes the tracked submodule while runtime diagnostics, installation metadata, documentation, and acceptance evidence can report the manifest constant. This makes provenance claims false even if the native binary itself builds and runs.

#### Required remediation

Establish exactly one intended whisper.cpp revision and make all representations agree with it.

The build and CI must independently verify that the tracked gitlink/source tree corresponds to the canonical expected revision. A mismatch must fail visibly before qualification. Do not merely change a displayed constant without validating the source that is actually compiled.

The canonical revision must be reflected consistently in:

- the tracked `third_party/whisper.cpp` gitlink or other approved vendoring mechanism;
- `manifest.rs` or its replacement;
- runtime diagnostics;
- model/runtime installation markers where the runtime source revision is recorded;
- `docs/WHISPER_MODEL_LICENSES.md`;
- `docs/THIRD_PARTY_NOTICES.md`;
- the real-CPU acceptance report;
- any handoff or pipeline document that states the revision.

Add a deterministic repository/build check that intentionally fails when the manifest/expected revision and the checked-out native source differ.

### WPR-110 — Model/source license and provenance documentation truthfulness

The review found contradictory repository documentation: Whisper model documentation records MIT while `docs/THIRD_PARTY_NOTICES.md` still describes the model license as pending verification. Source documentation also cites the wrong whisper.cpp revision because of WPR-100.

#### Required remediation

Create one consistent provenance record for both:

- vendored/built whisper.cpp source; and
- `ggml-small.bin` model artifact.

The record must include the exact source/revision/URL identity used by the build or installer, license identity, expected model bytes, and SHA-256 where applicable. If license text or attribution must be redistributed, ensure the repository/package path satisfies that obligation.

Repository documents must not simultaneously claim both verified and pending state for the same artifact.

### WPR-200 — Clean-profile install root and model layout

The reviewed installer creates a staging directory under the configured Whisper install root but does not reliably create the install root itself first. The dedicated CI workflow creates the root externally, masking the behavior users encounter on a clean profile.

The original TODO also requires `<app-data>/models/whisper/whisper-small/`, while current production path construction stores the model directly under `<app-data>/models/whisper/`.

#### Required remediation

The production installer itself must create every required parent directory using safe `create_dir_all`-style behavior before disk-space probing or staging creation.

Choose and document one canonical per-model layout. The preferred contract is:

`<app-data>/models/whisper/whisper-small/`

with model artifact and verification/marker metadata contained within that model-specific directory. If a different layout is deliberately selected, update the governing spec/TODO and migration behavior explicitly rather than silently contradicting it.

Installation tests must begin from a nonexistent Whisper root and prove that first install succeeds without workflow/test harness pre-creation.

If an older layout may exist in deployed profiles, define deterministic migration or compatibility behavior and test it.

### WPR-210 — Installed-model verification must be streaming and off the async command path

At the review baseline, installed-model verification can use `fs::read()` on a roughly 488 MB model, allocating the full file in memory before hashing it. `get_asr_models()` can invoke Whisper verification directly from an async Tauri command path, unlike Moonshine's blocking isolation.

#### Required remediation

Verify installed artifacts by streaming through a bounded buffer while computing SHA-256 and size. Do not read the entire model into one byte vector.

Any full-file filesystem/hash verification reached from an async Tauri command must execute behind `spawn_blocking` or an equivalent bounded blocking boundary.

Model descriptor retrieval must remain responsive and must not monopolize an async executor thread while hashing hundreds of megabytes.

Tests must verify both successful streaming verification and corrupt/short/oversized artifact handling without requiring a real 488 MB fixture.

### WPR-220 — Whisper-specific error taxonomy

The code contains corruption-aware installer error mapping, but `WhisperEngine::open()` can collapse verified-model failures into `ModelLoadFailed`. This makes the original TODO's claim that all documented Whisper error kinds are reachable and correctly mapped unreliable.

#### Required remediation

Define one explicit mapping table from installer/verification/native/runtime failures to public `AsrErrorKind` values.

At minimum:

- missing artifact -> `ModelNotInstalled`;
- size/hash/magic/integrity mismatch -> `ModelCorrupt`;
- native Whisper unavailable/not linked/unsupported runtime -> `RuntimeUnavailable`;
- verified artifact that native Whisper cannot load -> `ModelLoadFailed`;
- malformed PCM/input conversion failure -> `AudioInput`;
- `whisper_full`/segment extraction inference failure -> `Inference`;
- invalid lifecycle/API state -> `InvalidState`;
- explicit cancellation -> `Cancelled`;
- unexpected invariant/worker failure -> `Internal`.

Every claimed public kind must have a production-reachable path or the documentation/TODO must stop claiming that it does. Add focused tests around the mapping boundary.

### WPR-300 — Coherent utterance and partial/final transcript semantics

The reviewed `WhisperEngine` uses a `WHISPER_BATCH_THRESHOLD_SAMPLES` of 4,800 samples (approximately 300 ms at 16 kHz), takes/clears that buffer, transcribes it independently, and emits the resulting segments as `Final` updates. It does not emit Whisper `Partial` updates.

The downstream transcript state treats final transcript events as utterance boundaries and can send each final transcript as an independent text turn to the provider. Therefore ordinary speech can be fragmented into a sequence of very short “final” user turns.

#### Required remediation

Introduce an explicit Whisper utterance state machine.

A Whisper utterance must have:

- one stable utterance/segment identity across its partial updates;
- a bounded PCM history sufficient for partial retranscription;
- a partial-transcription cadence separate from finalization;
- explicit endpoint/finalization state;
- a bounded maximum utterance duration to prevent unbounded memory/compute growth;
- reset behavior after finalization.

A partial run may retranscribe a bounded recent/current utterance window, but it must emit `Partial`, not `Final`, and it must not cause a provider user-turn commit.

`Final` must represent a real utterance endpoint: endpoint silence, explicit stop/final flush, or another documented terminal condition. Merely reaching a 300 ms inference threshold is not an endpoint.

The implementation may use a simple deterministic local endpoint detector (for example amplitude/RMS plus hangover) or reuse an existing local speech-end signal if one is already authoritative. The chosen algorithm and constants must be testable and must not require cloud inference.

### WPR-310 — Stop-time final transcript must be delivered

At the review baseline, `WhisperEngine::stop()` transcribes leftover PCM and then discards the `collect_updates(...)` result. The pipeline engine stop contract returns only `Result<(), AsrError>`, so the worker has no path to forward final updates produced during stop.

#### Required remediation

Change the engine/pipeline shutdown contract so stop/finalize can return or emit transcript updates.

The final leftover utterance must be delivered through the same transcript-event path used during normal operation before the pipeline is considered drained.

The stop operation must be idempotent. A second stop must not duplicate a final transcript.

Tests must prove that a final utterance shorter than the normal partial cadence is still delivered exactly once on stop.

### WPR-320 — Drain accepted PCM before worker exit

The reviewed worker loop terminates based on `stop_requested` before ensuring that all chunks already accepted into the bounded queue have been processed. This can discard the tail of the user's utterance before `WhisperEngine::stop()` even sees it.

#### Required remediation

Separate “stop accepting new audio” from “abort immediately.”

Normal conversation teardown must:

1. stop microphone capture/producer input;
2. close or mark the PCM input stream as no-more-input;
3. drain all PCM chunks already accepted by the local-ASR queue;
4. finalize the current Whisper utterance;
5. deliver final transcript updates;
6. terminate and join the worker;
7. release the model lease.

An explicit emergency/abort path may discard queued audio only if it is semantically distinct, documented, and not used for normal conversation stop.

Add deterministic tests with queued chunks present at shutdown. Do not rely only on sleeps.

### WPR-330 — Conversation-layer finality contract

The conversation layer currently relies on local-ASR `FinalTranscript` semantics to decide when user text is committed. The Whisper engine bug demonstrates that this boundary needs stronger tests.

#### Required remediation

Document and test the invariant:

- `PartialTranscript` may update local UI/diagnostics but never commits a provider user turn;
- `FinalTranscript` commits exactly one completed local-ASR utterance;
- multiple partials for one utterance collapse into one final;
- stop-time finalization produces at most one provider user turn;
- an empty/whitespace final transcript produces no user turn.

Add an integration-level fake-engine regression that would fail if 100/300 ms batches were promoted to independent user turns.

### WPR-400 — Mode-specific local-ASR dependencies

At the review baseline, `prepare_local_asr()` can require the Moonshine installer before branching on the selected local-ASR mode. A valid Whisper configuration can therefore theoretically fail because an unrelated Moonshine dependency is absent.

#### Required remediation

Resolve installer/runtime/model dependencies inside the selected mode branch. Whisper startup must depend only on dependencies required by Whisper plus shared local-ASR infrastructure. Moonshine startup must remain independently functional.

Add a regression that constructs a Whisper-capable state with Moonshine-specific installer state unavailable and proves Whisper preparation does not fail for a Moonshine reason.

### WPR-410 — FFI thread-safety contract

`WhisperModel` currently has unsafe `Send` and `Sync` implementations while comments/design indicate one inference worker owns the context. `Sync` is a stronger concurrency guarantee than required and could permit future concurrent access without compiler protection.

#### Required remediation

Remove `Sync` unless upstream whisper.cpp guarantees and actual application usage require concurrent shared references to the same context. Prefer single-worker ownership with `Send` only.

If `Sync` must remain, document the upstream guarantee, synchronization model, and tests/audit evidence that make the unsafe promise valid.

Run a focused unsafe-code audit over the Whisper FFI module after the change.

### WPR-500 — Native rebuild invalidation

The reviewed `build.rs` watches only selected Whisper/CMake inputs and can miss relevant header or ggml-source changes.

#### Required remediation

Make native rebuild invalidation comprehensive and deterministic for the source set that can affect the linked Whisper library. Acceptable solutions include recursive source-manifest enumeration at build-script time or an explicit complete file manifest.

At minimum, changes to relevant:

- whisper headers;
- whisper sources;
- ggml headers;
- ggml sources;
- native CMake configuration

must force a rebuild/relink.

Add a repository/build-policy check if practical so newly introduced native source roots are not silently omitted.

### WPR-510 — CPU parallelism detection

The reviewed build logic attempts to read `/proc/nproc`, which is not the normal Linux CPU-count interface and generally falls back to a hard-coded value.

#### Required remediation

Use `std::thread::available_parallelism()` or an equivalent portable mechanism, with a safe minimum/fallback. Preserve any explicit operator override if one exists.

This is primarily build-performance correctness, not runtime ASR behavior, but it should be fixed while the build path is being remediated.

### WPR-600 — Frontend behavior and wording

Most P4 UI work is already present, but some text describes behavior the engine does not currently provide. The settings panel also uses generic verification wording that mentions CRC32C even though the Whisper path is SHA-256 based.

#### Required remediation

After WPR-300 is implemented, update UI text to describe the actual partial/final behavior precisely.

Verification text must be artifact/engine appropriate. Do not claim CRC32C for Whisper unless Whisper actually performs and relies on that verification.

The UI must continue to show:

- Whisper Small as local;
- install state;
- expected/installed bytes;
- runtime/source revision;
- disabled install/delete actions while unsafe during an active conversation;
- explicit local privacy disclosure.

Frontend tests must cover the corrected wording/state behavior.

### WPR-610 — Documentation reconciliation

Current README/privacy/license/third-party/handoff/TODO state is not internally consistent.

#### Required remediation

Update the relevant documents after production behavior is fixed, including as applicable:

- `docs/WHISPER_CPP_LOCAL_ASR_TODO.md`;
- `docs/WHISPER_CPP_LOCAL_ASR_SPEC.md` only where the governing design itself changes;
- `docs/LOCAL_ASR_WHISPER_HANDOFF_2026-10-03.md`;
- `docs/PRIVACY.md`;
- `docs/WHISPER_MODEL_LICENSES.md`;
- `docs/THIRD_PARTY_NOTICES.md`;
- `README.md`;
- a dedicated pipeline document if needed;
- CPU benchmark/evidence documentation once real measurements exist.

Historical documents may remain historical, but must not be edited to falsely imply they were generated from a later source state. Current authoritative docs must clearly point to current evidence.

### WPR-700 — Repair real-CPU acceptance workflow execution

The reviewed `.github/workflows/whisper-real-cpu-acceptance.yml` contains an embedded Python block with unexpected indentation in the evidence-validation section. Python compilation fails with `IndentationError`, so the workflow cannot currently complete its validation step.

#### Required remediation

Fix the script syntax and add a cheap static/compile check for embedded validation code so equivalent breakage is caught before a real-model run.

The workflow must be runnable manually on supported Linux hardware and must remain separate from ordinary CI.

Do not let workflow setup pre-create conditions that mask production installer defects; setup may create the application-data parent, but the production Whisper installer must be responsible for its own model root.

### WPR-710 — Acceptance provenance must verify actual source

The reviewed acceptance report can record the manifest's claimed source SHA rather than independently proving the native source revision checked out/built for the run.

#### Required remediation

Acceptance must independently capture and compare:

- exact repository source SHA being qualified;
- exact `third_party/whisper.cpp` gitlink/native source revision;
- canonical expected Whisper source revision;
- exact model SHA-256;
- exact model byte size.

The workflow must fail if any expected/actual source identity differs.

Evidence must be machine-readable and uploaded with immutable run identity.

### WPR-720 — Performance measurement correctness and tuning

P5 remains incomplete. The existing workflow has instrumentation hooks but does not yet provide trustworthy closeout for CPU, memory, latency, partial cadence, endpointing, or overload behavior. The reviewed “peak RSS” approach appears snapshot-based rather than a true high-water measurement.

#### Required remediation

For the exact final source SHA, record at least:

- first partial latency;
- final transcript latency after endpoint;
- real-time factor;
- CPU utilization using a documented sampling method;
- true/high-water process RSS using an OS-appropriate metric;
- dropped chunk count under nominal load;
- dropped chunk behavior under an intentional overload scenario;
- partial cadence;
- endpoint silence threshold/hangover;
- maximum utterance behavior;
- queue capacity justification.

Performance gates should be explicit enough to catch gross regressions but should not invent arbitrary thresholds before baseline evidence exists. Record the measured baseline first, then set justified acceptance bounds in the TODO/evidence if needed.

### WPR-800 — Regression and property coverage

The remediation must add tests at the behavior boundaries that failed review, not only implementation-unit tests.

Minimum regression coverage:

- source revision mismatch fails deterministically;
- clean-profile installation creates required roots;
- streaming hash verification succeeds without whole-file reads;
- corrupt model maps to `ModelCorrupt`;
- Whisper runtime unavailable maps to `RuntimeUnavailable`;
- partial updates do not become provider turns;
- one utterance produces multiple partials and exactly one final;
- stop flushes a sub-threshold final utterance;
- worker drains accepted queued PCM before normal stop returns;
- stop is idempotent and does not duplicate final text;
- empty final does not create a user turn;
- Whisper preparation does not depend on Moonshine-only installer state;
- model deletion remains blocked while leased;
- native build provenance checker detects revision mismatch;
- acceptance validation script at least parses/compiles in ordinary CI without downloading the model.

Tests around worker shutdown/queue draining should use deterministic barriers/channels rather than timing-only sleeps.

### WPR-900 — Original TODO reconciliation

`docs/WHISPER_CPP_LOCAL_ASR_TODO.md` contains checkboxes that the 2026-10-07 review found to be false or stale. It also leaves P4 tasks unchecked even though much of the UI work exists.

#### Required remediation

After production remediation and qualification, revisit every task and subtask in the original TODO.

For each checkbox:

- mark complete only if current final-source code and evidence prove the exact requirement;
- leave incomplete if it is still unsatisfied;
- update implementation notes that are factually incorrect;
- distinguish historical completion from final-source qualification where relevant;
- do not infer completion from a prior checkbox or commit message alone.

At minimum, the reconciliation must explicitly revisit the previously incorrect claims about:

- exact whisper.cpp source pin;
- model directory layout;
- partial transcript emission;
- bounded partial windowing;
- endpoint/final flush semantics;
- `ModelCorrupt` mapping;
- P4 frontend state;
- P5 workflow/source evidence.

### WPR-950 — Exact-head qualification and closeout

No Whisper closeout is valid until the final production source is qualified at one exact immutable SHA.

#### Required remediation

On the exact final source SHA:

1. run the repository's ordinary required checks;
2. run generated-tree/contract/security/privacy checks applicable to changed paths;
3. run focused Whisper unit/integration tests;
4. run the repaired real-CPU Whisper acceptance workflow;
5. collect machine-readable provenance and performance evidence;
6. verify all required jobs are terminal and successful;
7. record exact workflow/run/job/artifact identities in an evidence document;
8. reconcile both the remediation TODO and original Whisper TODO.

If final evidence/TODO commits occur after source qualification, verify that those commits are documentation/evidence only. Run the repository's documentation-scope CI on final `master`. If production source changes, repeat exact-head qualification.

## 7. Implementation sequencing

Recommended dependency order:

1. **WPR-100/WPR-110** — establish source/license truth before producing new evidence.
2. **WPR-200/WPR-210/WPR-220** — make installation and verification correct.
3. **WPR-300/WPR-310/WPR-320/WPR-330** — fix transcript semantics and shutdown data loss as one coherent vertical slice.
4. **WPR-400/WPR-410** — clean mode-specific dependencies and FFI contracts.
5. **WPR-500/WPR-510** — harden build behavior.
6. **WPR-600/WPR-610** — align UI/docs only after runtime behavior is truthful.
7. **WPR-700/WPR-710/WPR-720** — repair and execute acceptance/performance qualification.
8. **WPR-800** — regression coverage should be added alongside each slice and audited here for completeness.
9. **WPR-900/WPR-950** — final checklist reconciliation and exact-head/master closeout.

Independent implementation pieces may proceed concurrently where they do not create duplicate/conflicting work, but transcript finalization and queue-drain changes should be treated as one lifecycle design rather than patched independently.

## 8. Required verification commands and gates

The exact command names may evolve with the repository, but final qualification must include the current equivalents of:

```bash
npm run check:frontend
npm run check:rust
npm run check:all
npm run check:generated-trees
npm run check:generated-backend-contract
npm run check:tauri-command-contract
npm run check:frontend-contract-shapes
python3 scripts/check_local_llm_packaging_policy.py
```

Also run any Whisper-specific tests/checkers added by this remediation and the manual real-CPU Whisper workflow.

If packaging-policy paths are provably untouched, the packaging-policy check may be recorded as not applicable only when repository policy allows that classification; otherwise run it.

## 9. Acceptance criteria

This remediation is complete only when all of the following are true:

- Actual built whisper.cpp source revision equals the canonical reported revision.
- Repository license/provenance docs agree and are release-ready.
- A clean profile can install Whisper without pre-creating the model root.
- Installed model verification is bounded-memory and isolated from async executor threads.
- Integrity failures map to `ModelCorrupt` and native-unavailable failures map truthfully.
- Whisper emits coherent partials and one final per utterance; 300 ms inference cadence is not treated as utterance finality.
- Normal shutdown drains accepted PCM and forwards the stop-time final transcript exactly once.
- No local-ASR finality bug can create multiple provider user turns from one ordinary utterance.
- Whisper startup does not depend on Moonshine-only state.
- Unsafe Whisper FFI traits match the actual threading model.
- Native source/header changes trigger rebuilds and CPU parallelism detection is portable.
- Frontend/docs describe actual verification and transcript behavior.
- The real-CPU workflow parses, runs, verifies actual source/model provenance, and produces valid evidence.
- CPU/RSS/latency/RTF/overload measurements are recorded for the exact final source SHA.
- Every original Whisper TODO checkbox has been re-audited and reconciled.
- All mandatory exact-head CI and real-CPU gates pass on the final production source.
- Final `master` contains no unresolved mandatory item from this remediation.