# Whisper.cpp Local ASR Post-Qualification Review TODO

**Date:** 2026-10-08
**Status:** Implementation complete locally; exact-source ordinary CI and real-CPU acceptance remain open
**Review baseline:** `master` at `0268fb2bd3af2e31d355d9064824129ba89c1887`
**Spec:** `docs/WHISPER_CPP_LOCAL_ASR_POST_QUALIFICATION_REVIEW_SPEC_2026-10-08.md`
**Prior accepted run:** `37744371559`, job `113202252940`, checkout `0628de3d0cd946d8d1c0c3fe89afce8f3253854c`; see `docs/evidence/WHISPER_CPP_LOCAL_ASR_QUALIFICATION_2026-10-08.md`

All items start open. Mark a subtask complete only after the current source and a named test, gate, or recorded artifact support it. The prior run remains historical evidence and does not close changes made after its source SHA. Work remains on `master` unless the user changes that instruction.

## WPQ-100 — Stop-time final reaches the provider before teardown

- [x] Reproduce the delayed-callback race deterministically through the production pipeline event callback, using barriers/channels rather than sleeps.
- [x] Define an explicit worker-event-to-conversation delivery acknowledgement or equivalent pending-event ownership contract.
- [x] Stop capture before draining accepted PCM; retain the existing bounded queue and normal-stop drain semantics.
- [x] Keep the generation/session routable until every eligible stop-time final has a delivery disposition.
- [x] Wait for final disposition before clearing `draining_generation`, incrementing generation, or removing the live session.
- [x] Ensure final text is committed at most once; partials and whitespace finals commit no provider turn.
- [x] Surface provider failure/timeout through the existing bounded lifecycle/error path; avoid deadlock with lifecycle/session locks.
- [x] Reject stale events from an earlier generation after a replacement session begins.
- [x] Add regressions covering scheduled handoff, accepted queue drain, repeated Stop, whitespace final, provider error/timeout, and stale-generation replacement.
- [x] Confirm the new regression crosses the production callback scheduling boundary; retain the direct fake-resource regression as separate coverage.

**Acceptance:** a delayed stop-time final is committed once before normal Stop returns, or a bounded surfaced terminal error explains why it was not; it is never silently lost.

## WPQ-200 — Accurate, nonblocking Whisper diagnostics

- [x] Route the Whisper diagnostics command through an active runtime snapshot when the selected Whisper pipeline is running.
- [x] Route it through the retained final snapshot, including captured dropped chunks, after normal Stop.
- [x] Report truthful running/snapshot state, queue, error, latency, inference, CPU, RSS, and RTF fields when present; leave unavailable values empty.
- [x] Move Whisper installed-model verification reached from `get_asr_diagnostics` into `spawn_blocking` or equivalent blocking isolation.
- [x] Preserve verified model ID/revision/install state and accurate engine/source identity.
- [x] Test composition for active runtime, stopped snapshot, and no runtime; corrupt descriptor state is covered by the descriptor/installer regressions.
- [x] Add a check that descriptor verification runs on a blocking worker rather than the async command worker.
- [x] Align UI diagnostics labels, including peak/high-water RSS, with the backend measurement.

**Acceptance:** active Whisper is reported as active with its collected metrics; stopped metrics are labeled as a snapshot; diagnostics refresh does not hash the model on an async executor worker.

## WPQ-300 — Corrupt versus absent model classification

- [x] Define result states for absent artifact, existing wrong-size artifact, wrong SHA, bad magic, malformed marker, and incompatible revision.
- [x] Make wrong-size/truncated/oversized existing artifacts return an integrity failure rather than `Ok(None)`.
- [x] Map integrity failures to `AsrErrorKind::ModelCorrupt` in engine startup.
- [x] Map them to a corrupt descriptor and recovery action in model descriptors/diagnostics/Settings.
- [x] Keep genuinely absent model mapped to `ModelNotInstalled`/NotInstalled.
- [x] Cover size, SHA, magic, marker, incompatibility, engine mapping, and descriptor corruption with fixture-based regressions.

**Acceptance:** a truncated on-disk `ggml-small.bin` is shown as corrupt and cannot be mistaken for a model that was never installed.

## WPQ-400 — Crash-safe legacy migration

- [x] Copy cross-filesystem legacy artifacts to a unique staging path under the canonical model directory.
- [x] Verify the staged artifact and promote model/marker without exposing a partial canonical installation.
- [x] Keep the valid legacy artifact until promotion and marker writing complete.
- [x] Recover or retry when a prior attempt left a partial canonical artifact/marker.
- [x] Preserve symlink/path defenses, lock/lease behavior, delete, and reinstall semantics.
- [x] Test interrupted copy, failed marker promotion, retry, and valid legacy plus partial canonical state.

**Acceptance:** an interrupted migration can be retried without losing the previously valid legacy model or requiring an unnecessary full download.

## WPQ-500 — Download cancellation contract

- [x] Record the product choice: expose user cancellation, or explicitly narrow public claims to internal-only cancellation.
- [x] If user cancellation is chosen, store a scoped token for the active install and expose a cancel IPC command.
- [x] If user cancellation is chosen, add a Settings Cancel control and clear canceled/progress state.
- [x] If user cancellation is chosen, handle cancel/completion/delete/concurrent-install races without corrupting the canonical model or canceling inference.
- [x] If IPC changes, regenerate and review `src/generated/backendContract.json` and update frontend/backend contract tests.
- [x] Not applicable: user cancellation is selected and implemented; no internal-only claims remain.
- [x] Test the selected contract and staging cleanup.

**Acceptance:** a user can actually cancel an in-progress Whisper download when the UI/docs say they can; otherwise those surfaces state the narrower behavior accurately.

## WPQ-600 — Native rebuild invalidation and stale-library defense

- [x] Emit Cargo rerun coverage for creation/removal under every relevant whisper.cpp, ggml, and native-build configuration root, as well as changes to existing files.
- [x] Keep source provenance tied to the tracked/checked-out revision and preserve the supported-target matrix.
- [x] Handle `cmake` and `make` spawn failures explicitly.
- [x] Prevent stale `build/whisper` libraries from being linked as the current runtime after configuration/build failure.
- [x] Add a lightweight policy/fixture test for directory watches and missing/failing build-tool paths.

**Acceptance:** relevant native tree changes cause the build script to rerun; a failed build cannot silently link a prior artifact under a new source identity.

## WPQ-700 — Documentation and UI reconciliation

- [x] Update `docs/WHISPER_CPP_LOCAL_ASR_PIPELINE.md` to state the completed historical qualification and the new follow-up status precisely.
- [x] Correct stale pending-acceptance/real-profile notes in the 2026-10-07 remediation TODO without erasing dated run history.
- [x] Add a prominent follow-up pointer to the original Whisper TODO and prior remediation TODO where their checked claims have a known limit.
- [x] Align README, privacy, Settings, diagnostics, benchmark, and evidence pointers with the final code and the chosen cancellation behavior.
- [x] Distinguish source revision, model revision, runtime availability, live metrics, retained snapshots, and peak RSS in public wording.
- [x] Update frontend wording tests and applicable documentation/provenance checks.

**Acceptance:** a reader can identify what passed on the prior source, what remains open now, and what the current application actually reports and permits.

## WPQ-800 — Exact-source verification and closeout

- [x] Run focused regression tests for WPQ-100 through WPQ-600; use deterministic synchronization for races.
- [x] Run `npm run check:all`, including the full `npm run check:rust` quality gate and ordinary frontend/contract/Whisper static checks.
- [x] Run `npm run check:generated-backend-contract` when IPC/exported shapes change; no generated shape drift was produced.
- [x] Run affected native build-policy gates; keep ordinary CI real-model-free.
- [ ] Verify ordinary CI passes at the final production source SHA.
- [ ] After installer/runtime/stop source changes, run the manual real-CPU Whisper acceptance workflow at that exact source checkout.
- [ ] Verify source/model/corpus hashes and bytes, clean install/delete/reinstall, network-denied transcription, partial/final counts, nominal drops, overload, CPU, RSS, latency, and RTF from the workflow artifact.
- [ ] Record run/job/source SHA/artifact identity, artifact hash/bytes, and measured limits in `docs/evidence/`; retain the earlier run as a separate historical baseline.
- [ ] Re-audit every item in this checklist against final code and evidence before changing Status to Complete.

**Acceptance:** the final production source has passing ordinary gates and a successful exact-source real-CPU run; no new finding remains open or hidden by a historical checkbox.
