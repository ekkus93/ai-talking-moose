# Whisper.cpp local ASR remediation progress — 2026-10-08

This is progress evidence for `docs/WHISPER_CPP_LOCAL_ASR_POST_REVIEW_REMEDIATION_TODO_2026-10-07.md`. It is not final closeout evidence. Real-CPU acceptance and final exact-head/exact-master reconciliation remain open.

## Exact source heads covered

### `78646593cdb4412940e86ec3ecbb4bbdb679b27b`

Commit: `test(asr): check whisper ffi safety policy`

CI run: `37680412841`, attempt 2.

Successful jobs:

- `Classify CI scope` — job `113110843663`
- `Rust quality` — job `113110881854`
- `Rust tests` — job `113110881880`

Evidence established at this head:

- `scripts/check_whisper_provenance.py` runs in Rust quality and passed.
- The provenance checker verifies the manifest/source gitlink relationship and documentation provenance.
- The provenance checker now also enforces the Whisper FFI safety policy: no `unsafe impl Sync for WhisperModel`, retained `Send` ownership-transfer safety contract, and private raw Whisper C-layout structs.
- Rust formatting and Clippy passed.
- The complete Rust test suite passed.

Skipped jobs were path-scope skips and are not treated as pass evidence for unrelated frontend/generated/release/dependency gates.

### `2864610fae659a7e60e09b59967a142349e81799`

Commit: `test(asr): harden whisper acceptance workflow check`

CI run: `37716584632`, attempt 1.

Successful jobs:

- `Classify CI scope` — job `113114310054`
- `Rust quality` — job `113114428508`
- `Rust tests` — job `113114428475`

Evidence established at this head:

- `scripts/check_whisper_acceptance_workflow.py` passed through Rust quality.
- The ordinary-CI acceptance-workflow checker compiles embedded Python without downloading the real model.
- The checker now also fails closed if the workflow loses its manual-only `workflow_dispatch` trigger, gains `push`/`pull_request`/`schedule` triggers, omits `scripts/check_whisper_provenance.py`, omits the opt-in `whisper-acceptance` feature, omits the clean-profile model-root precondition, stops invoking the production installer, omits network-isolated transcription, omits network-denial assertion, omits machine-readable evidence output, omits SHA-bound artifact upload, or adds ad hoc `curl`/`wget` shell downloads.
- Rust formatting and Clippy passed.
- The complete Rust test suite passed.

Skipped jobs were path-scope skips and are not treated as pass evidence for unrelated frontend/generated/release/dependency gates.

## Still open

- Real-CPU Whisper acceptance has not been dispatched successfully from Ralph Bridge. The attempted configured validation profile `whisper-real-cpu-acceptance` was rejected by the bridge before a GitHub workflow run was created.
- Final real-CPU evidence is still required for repository SHA, actual native `third_party/whisper.cpp` gitlink, expected source revision, model SHA-256, model byte count, test-audio identity/hash, first-partial latency, final latency, real-time factor, CPU utilization, true/high-water RSS, nominal/drop overload behavior, and uploaded artifact identity.
- Generated backend contract was skipped for these script-only source scopes and is not newly established by these runs.
- Final TODO reconciliation in `docs/WHISPER_CPP_LOCAL_ASR_TODO.md` and `docs/WHISPER_CPP_LOCAL_ASR_POST_REVIEW_REMEDIATION_TODO_2026-10-07.md` remains incomplete.

### Evidence-backed checklist reconciliation — 2026-10-08 (source `b329854e8fb01ad9427a52401e45ab32659d4230`)

The following **source-level and deterministic-regression** items were checked only after re-reading the implementation, tests, and CI results on `b329854e8fb01ad9427a52401e45ab32659d4230`. This is not a real-model acceptance pass or final WPR-950/WPR-960 qualification. Source/fixture checks remain distinct from end-to-end native model evidence.

- **WPR-100/110:** `scripts/check_whisper_provenance.py` compares the independently tracked `third_party/whisper.cpp` gitlink with the pinned manifest, optionally compares checked-out native source, and validates the license-document identity strings. `scripts/test_check_whisper_provenance.py` has the deliberately mismatched-SHA regression. The real-CPU workflow runs provenance preflight before compiling.
- **WPR-200/210:** `src-tauri/src/asr/whisper/installer.rs` creates the canonical `models/whisper/whisper-small` root before probing disk or staging, co-locates model and marker, and implements verified legacy-layout migration. The clean-directory and migration fixture tests pass. `manifest.rs` streams verification in 64-KiB chunks with incremental SHA-256, exact size, and magic; success, truncated, oversized, wrong-SHA, and wrong-magic fixtures pass. `commands/asr_models.rs` isolates descriptor verification using `spawn_blocking`.
- **WPR-220/300/310/320/330:** `whisper/engine.rs` distinguishes missing/corrupt/runtime/load/PCM/inference/state errors and maintains an utterance identity across partials; endpoint silence, max duration, and explicit stop produce finals. `pipeline.rs` returns and forwards stop-time updates, drains accepted PCM before worker stop, and distinguishes graceful stop from abort. `pipeline_tests.rs` uses barriers for queue-drain/stop races and checks sub-threshold and blank stop finals; conversation routing tests prove partials and blank finals do not commit provider turns. The full production error-taxonomy audit, strict input-buffer bound, configurable cadence, and exact post-delivery state-reset requirement remain open.
- **WPR-400/410/500/510:** mode-specific startup no longer requires Moonshine for Whisper, with regression tests. FFI has private C-layout structs, exclusive-ownership `Send` rationale, and no `Sync` promise; the static policy rejects regressions. `build.rs` recursively watches whisper/ggml source, headers, and CMake roots and preserves `TALKING_MOOSE_WHISPER_BUILD_JOBS` with `available_parallelism()` fallback; Python policy tests cover missing roots and overrides. A full manual FFI unsafe-block audit remains open.
- **WPR-600/610/700/800:** `AsrSettingsPanel.tsx` shows local Whisper, state, bytes, runtime identity, active mutation guards, SHA-256-specific verification, local-audio privacy, and explicit download; frontend wording tests exist. `docs/WHISPER_CPP_LOCAL_ASR_PIPELINE.md` documents current utterance, stop, and queue behavior. `scripts/check_whisper_acceptance_workflow.py` compiles the workflow's embedded Python and rejects policy regressions, and the workflow is manual-only with no harness-created model root. The CI plumbing job executes this checker on workflow changes. Deterministic Rust/Python regressions listed in WPR-800 passed in ordinary CI; these are not a substitute for real-CPU acceptance.

**Exact evidence:** CI `37717431107` (commit `b329854e8fb01ad9427a52401e45ab32659d4230`) terminal success: Rust quality (provenance Python unit tests, formatting, Clippy), Rust tests (`cargo test --all-targets --all-features`), and CI plumbing (workflow checker); Wake required-gates audit `37717431047` terminal success on the same SHA. Earlier `37716584632` passed at `2864610fae659a7e60e09b59967a142349e81799`. All skipped jobs remain unqualified, and real-model acceptance has not been run.

**Deliberately still open:** clean-profile **real-model** first-install/delete/reinstall acceptance; FFI complete manual audit; any remaining mapping/utterance strictness issues; WPR-710 actual uploaded source/model/audio evidence; WPR-720 real CPU, RSS, latency, RTF, and overload measurements; WPR-900 original-TODO re-audit; WPR-950 and WPR-960 final exact-head/exact-master gates. Ralph Bridge rejected validation profile `whisper-real-cpu-acceptance` before dispatch; no workflow run or artifact exists from that request.
