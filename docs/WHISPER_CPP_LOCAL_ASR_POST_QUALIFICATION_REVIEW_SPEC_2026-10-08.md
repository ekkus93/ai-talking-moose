# Whisper.cpp Local ASR Post-Qualification Review Spec

**Date:** 2026-10-08
**Status:** Implementation underway; local code gates pass, exact-source ordinary CI and real-CPU qualification remain open
**Review baseline:** `master` at `0268fb2bd3af2e31d355d9064824129ba89c1887`
**Companion checklist:** `docs/WHISPER_CPP_LOCAL_ASR_POST_QUALIFICATION_REVIEW_TODO_2026-10-08.md`

## Purpose and evidence boundary

The original Whisper implementation and the 2026-10-07 remediation completed a substantial local-ASR path. The pinned source/model checks, bounded installer verification, private native FFI, dedicated inference worker, explicit download, utterance state machine, queue drain, and separate real-CPU acceptance must be preserved. The successful real-CPU run `37744371559` at checkout `0628de3d0cd946d8d1c0c3fe89afce8f3253854c` remains valid historical evidence for that source and the measured Linux x86_64 runner. It does not prove the conversation callback always commits stop-time finals or that current diagnostics are truthful.

This spec covers the remaining defects found by the subsequent code review. The callback race is established by the asynchronous control flow but has not been reproduced in a live conversation. A deterministic regression must establish the failure before, or while, the production fix is made. The existing checked boxes in `docs/WHISPER_CPP_LOCAL_ASR_TODO.md` and `docs/WHISPER_CPP_LOCAL_ASR_POST_REVIEW_REMEDIATION_TODO_2026-10-07.md` are historical claims, not acceptance evidence for this follow-up.

## Scope and priorities

| ID      | Priority        | Requirement                                                                                                                                                        |
| ------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| WPQ-100 | Release blocker | A normal Stop must deliver every eligible final transcript accepted by the worker to the correct live session before teardown completes.                           |
| WPQ-200 | Required        | Whisper diagnostics must show live and final-snapshot runtime metrics, and installed-model verification in diagnostics must not block an async executor worker.    |
| WPQ-300 | Required        | Existing wrong-size and other integrity failures must be classified as corrupt, while a genuinely absent model remains not installed.                              |
| WPQ-400 | Required        | Legacy layout migration must recover safely from interrupted copy/promotion and remain retryable.                                                                  |
| WPQ-500 | Required        | The model download's cancellation contract must be explicit and usable, or the public/docs claim of cancellation must be narrowed to the actual internal behavior. |
| WPQ-600 | Required        | Native source rebuild invalidation must cover file additions and build-tool failures must not silently select stale libraries.                                     |
| WPQ-700 | Required        | Current Whisper docs and UI wording must match qualified behavior and actual diagnostics.                                                                          |
| WPQ-800 | Qualification   | All applicable local/CI gates and a fresh exact-source real-CPU run must pass after relevant production source changes.                                            |

## Preserved invariants

- Whisper microphone PCM remains local. No failure, Stop, or retry may silently switch to Gemini Live, Moonshine, or a fake ASR provider.
- The existing single microphone owner, 16 kHz mono PCM capture, bounded ingress queue, drop-newest overload policy, and dedicated native inference worker remain intact.
- Partials remain local state updates. Only nonempty final text can create a provider user turn. A logical final is committed at most once.
- Normal Stop drains accepted PCM; an explicitly documented emergency abort may discard it.
- A stopped or superseded generation must never send a transcript into a replacement session.
- Model installation remains an explicit user action; the verified artifact and lease remain protected from unsafe concurrent deletion/replacement. No real model weights enter Git, ordinary CI, or bundles.
- Model hashes, source revision, and acceptance results are reported from checked artifacts rather than inferred from manifest constants alone.
- Production logs and tests must not print raw PCM, transcript content, provider payloads, or credentials beyond the existing privacy contract.
- Moonshine behavior and Gemini Live audio routing remain unchanged unless a shared lifecycle correction necessarily applies to them.

## WPQ-100 — Stop-time final delivery and conversation ownership

### Existing failure path

`asr/pipeline.rs` emits a stop-time `FinalTranscript` on the worker before it exits. The event callback in `conversation/session/local_asr.rs` schedules `handle_local_asr_event` with `tauri::async_runtime::spawn` and returns immediately. `LocalAsrLifecycle::stop_and_clear` considers the worker stopped when it joins, then clears `draining_generation`; conversation shutdown increments the generation and removes the session. The scheduled callback can start after these steps and be rejected. The existing stop regression calls `handle_local_asr_event` directly from a fake resource's `stop`, so it does not exercise this scheduling boundary.

### Required contract

1. Stop microphone production, drain every accepted queue item, run final inference, and produce the final event in that order.
2. Treat callback scheduling as pending work. Normal Stop must wait for **delivery disposition** of its generation's accepted final events before clearing the draining generation or live session. A disposition may be committed, intentionally ignored because final text is empty, or a surfaced bounded provider/terminal error. It may not silently disappear because the callback task was delayed.
3. Keep event order and final deduplication deterministic for a single generation. A partial cannot commit a user turn; repeated Stop cannot commit the same final twice.
4. Avoid deadlock between shutdown, the event consumer, lifecycle locks, and the live-session lock. Bound any wait according to the existing provider operation timeout policy; a provider failure must surface through the existing error/lifecycle path.
5. Preserve generation and session-ID validation after a new conversation starts. A stale event cannot be delivered to the replacement session.
6. Make the worker-to-conversation handoff explicit. An acknowledged channel, tracked join handles, or another equivalent mechanism is acceptable if the delivery and teardown guarantees are testable.

### Proof

Use barriers/channels to pause the callback **after it has been scheduled but before it handles the event**. Start normal Stop; prove teardown waits, then release the callback and assert one final provider turn before Stop returns. Also cover several queued 100 ms chunks, multiple partials, empty final text, repeated Stop, provider failure/timeout, and a new-generation race. The regression must use the production event callback path, not a fake resource that directly awaits `handle_local_asr_event`. Keep the existing engine and queue-drain tests.

## WPQ-200 — Truthful Whisper diagnostics without async blocking

`get_asr_diagnostics` currently passes `None` to `compose_asr_diagnostics` for Whisper. It therefore substitutes a non-streaming empty runtime even while Whisper is active or a final snapshot exists. It also calls `whisper_descriptor` directly; that call can stream-hash the 487,601,967-byte installed model on the async command worker.

For the selected Whisper mode, use the same active-runtime-or-last-snapshot policy as the other local modes. Live metrics must include truthful running state, queue depth/capacity, dropped chunks, last error, first partial/final latency, inference time, CPU, RSS, and real-time factor where available. After Stop, mark retained metrics as a snapshot and keep the recorded dropped-chunk count. When no run/snapshot exists, display explicit empty values rather than invented measurements. Run full installed-model verification through `spawn_blocking` or equivalent isolation. Keep model identity and install state tied to the verified descriptor.

Test active, stopped-snapshot, no-runtime, and corrupt-model cases through the command's Whisper selection path. A test should prove the descriptor work is delegated off the async executor rather than merely testing the pure composition function. The settings panel must describe a peak/high-water RSS measurement accurately.

## WPQ-300 — Installed-artifact error taxonomy

The canonical artifact path being absent may return `NotInstalled`/`ModelNotInstalled`. Once an artifact exists, wrong size, wrong SHA-256, bad magic/header, malformed marker, and any other integrity mismatch must return `Corrupt`/`ModelCorrupt` (or a distinct, documented incompatible revision state where appropriate). Do not turn a truncated artifact into `NotInstalled` merely because it cannot be used. Keep I/O failures distinguishable from deliberate absence where the public error model permits it.

The installer, engine open path, model descriptor, diagnostics, and Settings action label must agree. A corrupt artifact should offer reinstall/recovery. Add production-path fixture tests for missing, truncated, oversized, wrong SHA, bad header, and invalid/incompatible marker cases. Cover installer result, `AsrErrorKind`, and descriptor state, not only the low-level hash helper.

## WPQ-400 — Recoverable legacy layout migration

The old `<app-data>/models/whisper/ggml-small.bin` layout may be migrated into `<app-data>/models/whisper/whisper-small/`. A rename is atomic only on the same filesystem. If migration falls back to copying, copy into a unique staging path, verify that staged artifact, and promote it atomically with its metadata. Retain the verified legacy artifact until canonical promotion is complete. An interrupted or failed attempt must be retryable at next startup/verification; a partial canonical file must not permanently suppress migration. Do not follow attacker-controlled symlinks or arbitrary paths. Keep deletion and reinstall behavior explicit for either layout.

Test interrupted copy, marker write/promotion failure, restart/retry, and a valid legacy artifact with a partial canonical destination. Use small injected fixtures; ordinary tests must not download the real model.

## WPQ-500 — Download cancellation contract

The installer cancellation token is currently created inside the install command and is not reachable by the user during the download. Choose and implement a clear public contract:

- Preferred: expose a scoped cancel operation for the active Whisper model install, provide a Cancel control and visible terminal state in Settings, and make cancellation race safely with completion, verification, concurrent install, and delete. Cancellation affects only the download/install operation, not an active ASR inference session.
- If product scope deliberately excludes user cancellation, remove or qualify UI/docs/TODO claims that users can cancel. Retain internal cancellation only where actually invoked, and state that the download cannot presently be canceled from Settings.

The preferred path is required before claiming **user-cancellable** installation. If adding IPC, update generated Rust/frontend contracts and test command/action behavior as well as installer cleanup. The selected option must be documented in the companion TODO before closure.

**Selected product behavior:** Whisper installation is user-cancellable from Settings. The cancel action is scoped to the active Whisper install and does not cancel ASR inference.

## WPQ-600 — Native rebuild and build failure truthfulness

The recursive build-script walker emits `rerun-if-changed` for files already present but not their directories. Changes to existing files are covered; creation of a relevant source/header/configuration file by itself is not. Watch the relevant directories or another deterministic inventory input so new and removed files cause a rerun. Keep the exact-source provenance check and Linux target boundary.

The `cmake` and `make` spawn results must be handled explicitly. A failed or missing build tool must not fall through to `has_whisper_runtime` and link libraries left from an earlier source build as though they were current. Either fail the build with a useful bounded diagnostic or fail closed with `whisper_native_linked` absent. Test the rerun-path policy and stale-artifact failure path with lightweight fixtures or a focused build-policy checker.

## WPQ-700 — Current documentation and UI

Update the current Whisper pipeline, benchmark/evidence pointer, README/privacy/Settings wording where this follow-up changes behavior. Remove stale statements that qualification or real-profile installation remains pending when referring to the already completed 2026-10-08 run. Preserve earlier failed runs as clearly dated historical evidence. Distinguish the runtime's source revision from the model revision and from whether the runtime is actually linked/available. Describe diagnostics as live or retained snapshots and label RSS according to the source of the measurement. Do not rewrite the old accepted run as if it covered new production code.

The two original TODOs should retain their historical trail. Add a prominent link or status note to this follow-up wherever a checked claim is now known to have a qualification limit. This follow-up TODO becomes the current source of truth until closed.

## WPQ-800 — Verification and qualification

1. Before code changes, preserve the current accepted run identity and baseline source SHA in the evidence record. After changes, record the new production source SHA separately.
2. Run `npm run check:all` (which includes frontend, Rust, contract, and Whisper static checks), `npm run check:generated-backend-contract` when IPC shapes change, and any applicable packaging/native build-policy checks. Do not substitute narrower ad hoc Clippy/test commands for `npm run check:rust`.
3. Keep ordinary CI free of real Whisper model downloads. The manual `.github/workflows/whisper-real-cpu-acceptance.yml` remains the only real-model acceptance path.
4. Because this follow-up is expected to change installer/runtime/stop behavior, rerun real-CPU acceptance on the exact new production source checkout. Verify installer state, source/model/corpus provenance, offline transcription, pipeline partial/final counts, zero nominal drops on the measured corpus, deliberate overload, CPU, RSS, latency, and real-time factor. Treat these as measured-run evidence, not universal performance guarantees.
5. Record exact workflow run, job, repository SHA, source/model revisions, artifact name/ID/hash/bytes, and observed metrics in `docs/evidence/`. Verify ordinary CI on the same production source. Documentation-only closeout may follow without relabeling the accepted source.
6. Close the companion TODO only when every mandatory subtask and regression passes. If a choice or target is out of scope, state the deliberate limit in the spec/TODO and make the public wording match it.

## Completion criteria

The follow-up is complete only when normal Stop provably waits for its final-event disposition, Whisper diagnostics are truthful and nonblocking, installed-artifact states and legacy recovery are correct, cancellation claims match behavior, native rebuild/build failures fail safely, current docs agree with the code, and exact-source ordinary plus real-CPU gates have passed. No checkbox or prior workflow result substitutes for that evidence.
