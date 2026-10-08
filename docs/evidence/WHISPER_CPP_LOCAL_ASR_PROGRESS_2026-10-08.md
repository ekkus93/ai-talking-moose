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
