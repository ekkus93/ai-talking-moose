# Wake Word V1 CI and Acceptance Gates

This document records the current Wake Word V1 gate inventory and the merge-eligibility policy implied by `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`.

## Implemented gates

### Ordinary CI

Workflow: `.github/workflows/ci.yml`

Purpose:

- frontend typecheck, lint, formatting, tests, and build
- Rust formatting, Clippy, and Rust tests
- generated contract checks
- dependency and packaging checks when path filters select them

Policy:

- Ordinary CI is required for every mergeable Wake Word change.
- Ordinary CI alone is not final Wake Word V1 qualification.

### Deterministic corpus manifest gate

Workflow: `.github/workflows/wake-word-corpus.yml`

Inputs guarded by path filters:

- `docs/wake-word-corpus.json`
- `docs/fixtures/wake-word-v1/**`
- `scripts/check_wake_word_corpus_manifest.mjs`
- `.github/workflows/wake-word-corpus.yml`

Current check:

```bash
node scripts/check_wake_word_corpus_manifest.mjs
```

Purpose:

- enforce the corpus manifest schema version
- enforce the fixture schema version
- enforce the acceptance criteria version
- enforce fixed Wake Word policy: 16 kHz, mono, `pcm_s16le`, fixed fixture root
- reject private-room-audio policy omissions
- require fixture provenance, redistributable license evidence, byte size, SHA-256, and expected detection outcome for any committed fixture
- reject fixture paths outside `docs/fixtures/wake-word-v1`
- keep recall/false-accept thresholds pending until real redistributable fixtures exist

Merge policy:

- This gate must pass when its path filters select it.
- A skipped corpus gate is acceptable only for changes that do not touch corpus inputs, checker, fixture tree, or workflow.
- A skipped corpus gate must not be treated as proof that real corpus acceptance passed.

## Pending required gates

The following gates are still required before final Wake Word V1 closeout, but are not yet implemented as complete acceptance gates on `master`.

### Linux x86_64 real KWS acceptance

Required future proof:

- prepare the exact pinned model and runtime
- verify every hash before inference
- verify ELF x86_64 runtime architecture
- verify CPU-only production path
- verify one-thread policy
- run at least one real positive Wake Word fixture
- run at least one real negative fixture
- prove inference succeeds offline after artifact preparation
- upload or record privacy-safe diagnostics/evidence
- record exact commit, manifest, runner/platform details, and run ID

A component test or manifest check is not sufficient for this claim.

### macOS arm64 real KWS acceptance

Required future proof:

- prepare the exact pinned model and runtime
- verify every hash before inference
- verify Mach-O arm64 runtime architecture
- verify CPU-only production path
- verify one-thread policy
- run at least one real positive Wake Word fixture
- run at least one real negative fixture
- prove inference succeeds offline after artifact preparation
- upload or record privacy-safe diagnostics/evidence
- record exact commit, manifest, runner/platform details, and run ID

A Linux-only result must not be reused as macOS arm64 evidence.

### Native packaging/architecture gate

Required future proof:

- verify packaged runtime layout for each supported platform
- verify architecture of the runtime library actually loaded by the packaged build
- verify cached artifacts cannot bypass size/hash checks
- fail closed for unsupported platforms

Existing model/runtime identity and packaging component tests are useful prerequisites but not final packaged acceptance by themselves.

### Integrated lifecycle stability gate

Required future proof:

- repeated wake→ASR→Thinking→Talking→wake cycles
- no native runtime/session growth
- no capture-stream multiplication
- bounded ring-buffer memory
- successful TTS resumes Wake Word when enabled
- cancelled TTS resumes Wake Word when enabled
- recoverable TTS failure resumes Wake Word when enabled
- repeated disable/enable cycles
- shutdown while Listening
- shutdown during handoff
- bounded soak/false-trigger behavior where practical

Current deterministic runtime-manager stability tests cover important state-machine invariants, but they are not the full production audio/lifecycle acceptance gate.

### Performance evidence gate

Required future proof:

- idle Wake Word CPU utilization on representative Linux and macOS environments
- memory overhead
- inference timing/real-time behavior
- wake detection to command-ASR activation latency
- pre-roll replay/startup timing
- repeated-cycle resource behavior
- comparison with continuously running full ASR

No performance claim should be made until measured evidence is recorded.

### Privacy/security final audit gate

Required future proof:

- single Wake runtime ownership
- microphone ownership transitions
- cancellation/shutdown
- ring-buffer clearing
- Wake-disabled behavior
- Talking suspension/resume
- one-trigger/one-command invariant
- provider separation/no cloud fallback
- exact artifact/runtime loading
- native architecture verification
- diagnostics/logs/errors do not expose raw audio, secrets, or unnecessary paths
- no network dependency during idle KWS inference
- no full-time ASR merely for wake detection
- user-facing docs do not overstate acceptance

## Final merge eligibility

A final Wake Word V1 feature head is not eligible based on ordinary CI alone. Final closeout must record exact PR-head and exact merged-master evidence for all required Wake-specific gates that apply to the final feature claim.

A workflow with conclusion `skipped` is evidence only that its path filter or condition did not select that workflow. It is not evidence that the acceptance scenario passed.
