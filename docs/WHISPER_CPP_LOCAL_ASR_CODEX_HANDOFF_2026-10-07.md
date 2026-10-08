# Codex Handoff — Whisper.cpp Local ASR Post-Review Remediation

**Date:** 2026-10-07  
**Repository:** `ekkus93/ai-talking-moose`  
**Branch:** `master`  
**Pre-handoff source SHA reviewed:** `60883bdd71108bb223ca5a5663faaa6c75b6b82d`  
**Authoritative remediation checklist:** `docs/WHISPER_CPP_LOCAL_ASR_POST_REVIEW_REMEDIATION_TODO_2026-10-07.md`  
**Governing spec:** `docs/WHISPER_CPP_LOCAL_ASR_POST_REVIEW_REMEDIATION_SPEC_2026-10-07.md`  
**Original checklist to reconcile before closeout:** `docs/WHISPER_CPP_LOCAL_ASR_TODO.md`

This document hands the remaining Whisper.cpp local-ASR remediation to Codex. The handoff commit itself is documentation-only and therefore advances `master` beyond the pre-handoff SHA above. Always reload current `master` before doing work and bind any qualification evidence to the exact commit actually tested.

## Operating rules

Work directly on `master` unless repository policy prevents it. Do not create a branch/PR for every checklist item. Prefer coherent implementation slices when adjacent items share production code, tests, or qualification evidence.

Treat `docs/WHISPER_CPP_LOCAL_ASR_POST_REVIEW_REMEDIATION_TODO_2026-10-07.md` as the sole source of completion truth for this remediation. Do not mark an item complete merely because infrastructure exists or an older checklist says it is complete. Reload the checklist after every merge/meaningful master advance.

Use exact-head discipline for CI and acceptance. Ordinary CI success is not a substitute for the real-CPU Whisper acceptance workflow or performance evidence.

Do not weaken model/source verification for convenience. The production path is intentionally fail-closed.

## Current state

At pre-handoff `master` `60883bdd71108bb223ca5a5663faaa6c75b6b82d`, the remediation checklist mechanically contains **135 checked** and **140 unchecked** checkbox lines (including nested checkboxes). The project is therefore not closeout-complete even though much of the production implementation has already landed.

The canonical native whisper.cpp source identity recorded by the current remediation is:

- whisper.cpp/native source revision: `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`
- Hugging Face model-artifact revision: `5359861c739e955e79d9a303bcbc70fb988958b1`

Those are intentionally different identities and must remain distinguished.

The last source-bearing CI run I verified was:

- GitHub Actions run `37717431107`
- exact SHA `b329854e8fb01ad9427a52401e45ab32659d4230`
- conclusion: success
- Rust tests: success
- Rust quality: success
- CI plumbing: success
- Whisper source/model/license provenance check: success

The subsequent `master` commit `60883bdd71108bb223ca5a5663faaa6c75b6b82d` was documentation/TODO reconciliation only. Its CI run `37718708600` succeeded through the docs-only path; Rust/frontend/source jobs were correctly skipped for that docs-only commit. Do not interpret that docs-only CI run as a fresh full-source qualification.

## What has already been completed

The current TODO records substantial completed work, including:

- canonical whisper.cpp source provenance and deterministic source-revision mismatch checking;
- separation of native-source identity from model-artifact identity;
- model SHA/size/provenance recording;
- canonical clean-profile model layout and root creation logic;
- streaming/bounded-memory model verification with blocking isolation;
- most public Whisper error mappings;
- explicit utterance state, stable utterance IDs, partial events, endpointing, and forced-finalization behavior;
- stop-time final transcript delivery and idempotent stop behavior;
- graceful queue drain infrastructure;
- conversation-layer partial/final safeguards;
- removal of Whisper startup's dependency on Moonshine-only installer state;
- tightened FFI ownership, including removal of unnecessary `Sync`;
- recursive native rebuild invalidation;
- portable CPU parallelism detection via `available_parallelism()`;
- frontend wording/state/provenance corrections;
- repair of the real-CPU acceptance workflow's malformed embedded validation code;
- ordinary-CI checks for acceptance-script syntax/provenance policy;
- a large regression suite covering many of the reviewed failure classes.

Do not reimplement these from scratch. Re-read the source and tests first, then close only the gaps that remain objectively open.

## Important correctness concern to re-verify first

### WPR-320 queued-PCM shutdown regression

A previous reconciliation marked the WPR-320 item

> Test shutdown with multiple queued ~100 ms chunks pending.

as complete. During review I noticed that the existing deterministic queue-drain regression appeared to use tiny/two-byte chunks rather than realistic approximately 100 ms PCM chunks.

I prepared the idea for a stronger FIFO regression but did not get it committed in that session.

**Codex should verify this immediately.** If the current test still does not actually queue multiple ~100 ms PCM chunks, reopen that checkbox and add a deterministic test using realistic chunk sizes. The test should prove FIFO drain of multiple accepted chunks through normal graceful stop, final inference, final event delivery, and worker termination. Do not rely on sleep-only timing.

This is a specific case where the current checkbox state may be more optimistic than the evidence.

## Remaining implementation/correctness work

### WPR-100 / WPR-110 / WPR-200

Still open:

- update current handoff/pipeline/acceptance documents that report the native source revision;
- ensure required redistributable license/notice text is actually present in the release/package path;
- execute a true clean-profile first install without workflow/test code pre-creating the Whisper root;
- execute delete/reinstall verification after the canonical layout changes.

### WPR-220 — error taxonomy

Still open:

- define one explicit internal-to-`AsrErrorKind` mapping contract;
- remove or wire any dead error-mapping helpers;
- add focused tests for every error kind that is actually production-reachable;
- correct docs/original TODO instead of fabricating paths for intentionally unreachable kinds.

### WPR-300 — utterance semantics

Still open:

- prove/implement a bounded utterance/current-window PCM buffer;
- make partial-inference cadence explicitly configurable;
- ensure utterance reset occurs only after finalization delivery;
- prove one ordinary sentence can produce multiple partials but exactly one final user utterance;
- prove bounded memory and bounded compute per utterance.

Inspect the production implementation before assuming these require new architecture; some may only need tightening plus evidence.

### WPR-320 / WPR-330

Still open:

- explicitly verify capture/producer input is stopped before local-ASR queue drain;
- re-verify the realistic queued-PCM shutdown test described above;
- add/prove conversation-layer stop-time finalization commits at most one provider user turn.

### WPR-410 — FFI safety

Still open:

- re-audit every Whisper `unsafe` block for lifetime, ownership, null, string/UTF-8, and thread assumptions;
- if any `Sync` implementation exists again, either remove it or document the exact upstream/thread-synchronization guarantee.

### WPR-800 regression audit

Still open:

- re-verify that an active model lease blocks unsafe deletion/replacement.

## Documentation still to reconcile

WPR-610 remains broadly open. Reconcile current behavior and evidence in at least:

- `docs/WHISPER_CPP_LOCAL_ASR_SPEC.md`
- `docs/LOCAL_ASR_WHISPER_HANDOFF_2026-10-03.md` or a clearly superseding current document
- `docs/PRIVACY.md`
- `docs/WHISPER_MODEL_LICENSES.md`
- `docs/THIRD_PARTY_NOTICES.md`
- `README.md`
- `docs/WHISPER_CPP_CPU_BENCHMARK.md` or equivalent evidence document

`docs/WHISPER_CPP_LOCAL_ASR_PIPELINE.md` is already recorded as updated, but re-check it against the final production semantics.

Current documents must distinguish independently verifiable current source/model identity from historical evidence generated at older SHAs.

## Real-CPU acceptance and performance are the largest unfinished block

WPR-710 and WPR-720 are effectively still open and are the main reason this remediation cannot be declared complete.

The final acceptance run must bind evidence to the exact source under test and independently record/verify:

- exact repository SHA;
- actual `third_party/whisper.cpp` gitlink/native source revision;
- canonical expected Whisper source revision;
- downloaded model SHA-256;
- downloaded model byte count;
- test-audio identity/hash;
- transcription output;
- machine-readable evidence artifact;
- immutable workflow run/job/artifact identity.

Performance evidence must include:

- first partial latency;
- final transcript latency from endpoint/finalization;
- real-time factor;
- CPU utilization using a documented sampling method;
- true/high-water process RSS using an OS-appropriate metric;
- dropped chunks under nominal load;
- a deliberate overload case and its dropped-chunk behavior;
- selected partial cadence;
- endpoint silence/hangover;
- maximum utterance duration/forced-finalization behavior;
- queue-capacity justification.

Do not claim P5 complete until the CPU and RSS evidence exists.

## Real-CPU workflow blocker encountered in the ChatGPT/Ralph session

The ChatGPT run attempted to invoke real-CPU acceptance through Ralph Bridge, but the requested validation profile was rejected by the Ralph Bridge execution/configuration layer. That prevented me from obtaining the required real-model/real-CPU evidence.

This was an execution-environment/tooling blocker, not evidence that the repository workflow itself is invalid.

For Codex:

1. inspect `.github/workflows/whisper-real-cpu-acceptance.yml`;
2. determine the supported way to dispatch it in the environment available to Codex;
3. run it against an exact final production SHA on supported Linux CPU hardware;
4. fix any workflow/harness issue that appears;
5. preserve bounded diagnostics and artifact identity;
6. do not substitute ordinary CI for this acceptance run.

A later Ralph source-write rejection also occurred during the prior ChatGPT session. However, this handoff file was successfully committed through Ralph Bridge, so repository source-write authorization is working again as of this handoff. Treat the earlier write rejection as transient/historical unless it recurs.

## Original TODO reconciliation remains mandatory

WPR-900 is not optional. The older `docs/WHISPER_CPP_LOCAL_ASR_TODO.md` currently contains many checked items that predate this post-review remediation.

After production source is stable and acceptance evidence exists, re-read final source and reconcile every P0-P5 claim in the original TODO against actual code/evidence.

In particular, do not preserve old checks for target-matrix, performance, transcript semantics, or provenance simply because an older implementation note said they were done.

## Final qualification sequence

After all implementation gaps are closed:

1. Reload current `master`; record exact qualification SHA.
2. Run all repository-required ordinary checks listed in WPR-950, including frontend, Rust, generated-tree/contracts, Tauri command contracts, frontend contract shapes, packaging-policy checks when applicable, and all Whisper-focused checks.
3. Run the repaired real-CPU acceptance workflow on that exact SHA.
4. Verify all ordinary CI and specialized acceptance runs are terminal and successful.
5. Verify the acceptance artifact contains exact repo/native/model/test-audio identities plus CPU/RSS/latency/RTF/overload metrics.
6. Create final evidence under `docs/evidence/` with exact workflow/run/job/artifact identities.
7. Reconcile `docs/WHISPER_CPP_LOCAL_ASR_TODO.md`.
8. Reconcile the authoritative post-review remediation TODO.
9. Re-read `master` after those docs/evidence commits.
10. If production source changed after qualification, repeat exact-head qualification. If only docs/evidence changed, record that fact and run the applicable exact-master/docs CI.
11. Complete WPR-960 only when final `master` contains the qualified production source unchanged and no mandatory review finding remains open.

## Suggested immediate Codex execution order

1. Pull/reload current `master` and read the authoritative TODO/spec.
2. Re-audit WPR-320's realistic multi-chunk shutdown test; fix/reopen if necessary.
3. Close the remaining source/test gaps in WPR-220, WPR-300, WPR-320, WPR-330, WPR-410, and WPR-800 as one or a few coherent slices.
4. Run focused deterministic tests during development; then full exact-head ordinary CI for source-bearing changes.
5. Exercise clean-profile install and delete/reinstall.
6. Repair/extend the acceptance harness as necessary and run real-CPU qualification with machine-readable evidence.
7. Reconcile documentation and performance evidence.
8. Re-audit the original TODO (WPR-900).
9. Perform WPR-950 exact-head qualification.
10. Perform WPR-960 exact-master closeout.

## Definition of done

Do not declare this project/remediation complete until **every mandatory checkbox** in `docs/WHISPER_CPP_LOCAL_ASR_POST_REVIEW_REMEDIATION_TODO_2026-10-07.md` is objectively satisfied and reconciled, the original Whisper TODO has been re-audited, real-CPU evidence exists for the exact final production source, and all exact-head/exact-master gates pass.
