# Wake Word V1 CI and Acceptance Gates

This document records the current Wake Word V1 gate inventory and the merge-eligibility policy implied by `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`.

## Required-gate source of truth

`docs/wake-word-required-gates.json` is the machine-readable required-gate inventory. `scripts/check_wake_word_required_gates.mjs` audits that inventory on pull requests and rejects drift, missing workflow files, `continue-on-error: true`, non-exact-head policy, or any policy that treats a skipped workflow as acceptance evidence.

The audited titles are: Ordinary CI; Wake Word required gates manifest audit; Wake Word deterministic corpus manifest gate; Wake Word corpus contract gate; Native packaging/architecture policy gate; Lifecycle stability gate; Performance evidence policy gate; Privacy/security source audit gate; Documentation truthfulness audit; Source/security ownership audit gate; Linux x86_64 real KWS acceptance; macOS arm64 real KWS acceptance; Integrated production lifecycle acceptance; and Measured performance acceptance.

`scripts/check_wake_word_final_qualification_readiness.mjs` is the fail-closed final-readiness check. It rejects final qualification while any required gate in the manifest remains pending. The manually dispatched `.github/workflows/wake-word-final-qualification-readiness.yml` exists specifically so a final feature head cannot qualify using ordinary CI alone. Passing it is still not a substitute for recording the exact-head run IDs and reports required by WWR-950.

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

### Deterministic corpus manifest and contract gates

The Wake Word corpus contract is the Python companion to the Node manifest gate.

Workflows:

- `.github/workflows/wake-word-corpus.yml`
- `.github/workflows/wake-word-corpus-contract.yml`

Current checks:

- `node scripts/check_wake_word_corpus_manifest.mjs`
- `python scripts/validate_wake_word_corpus.py`
- `PYTHONPATH=scripts python -m unittest scripts/test_validate_wake_word_corpus.py`

Purpose:

- enforce corpus, fixture, and acceptance-criteria schema versions
- enforce fixed Wake Word policy: 16 kHz, mono, `pcm_s16le`, fixed fixture root
- require fixture provenance, redistributable license evidence, byte size, SHA-256, and expected detection outcome for committed fixtures
- reject fixture paths outside `docs/fixtures/wake-word-v1`
- keep recall/false-accept thresholds pending until real redistributable fixtures exist
- keep the legacy Python corpus contract aligned with the active `docs/wake-word-corpus.json` schema

Policy:

- These gates must pass when their path filters select them.
- A skipped corpus gate is not evidence that real corpus acceptance passed.
- Passing schema/contract gates do not mean the repository contains real audio fixtures or calibrated detection thresholds.

### Native packaging/architecture policy gate

Workflow: `.github/workflows/wake-word-native-packaging.yml`

Matrix: hosted Linux x86_64 and macOS arm64.

Current checks:

- `python3 scripts/validate_wake_word_artifact_manifest.py --production`
- `python3 -m unittest tests/test_wake_word_runtime_artifacts.py`
- runner OS/architecture recording

Purpose:

- exercise the frozen production manifest and pinned runtime preparation policy on both target operating systems
- verify architecture rejection and cache/hash behavior represented by the runtime-artifact tests
- bind packaging-policy changes to an exact-head specialized workflow

Policy:

- This gate does not by itself prove real KWS inference or that a packaged application loaded the native runtime.
- WWR-610/620 real positive/negative inference and final packaged-runtime evidence remain separate acceptance requirements.

### Lifecycle stability gate

Workflow: `.github/workflows/wake-word-lifecycle-stability.yml`

Current check: `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features wake_word_stability -- --nocapture`.

Purpose:

- exact-head targeted execution of deterministic Wake Word runtime-manager stability tests
- exercise bounded lifecycle/state-machine invariants independently of broad ordinary CI

Policy:

- Changes to authoritative Wake runtime/lifecycle paths select this gate.
- A successful deterministic stability run is prerequisite evidence, not proof of the full production audio soak required by WWR-640.
- A skipped lifecycle workflow is not lifecycle acceptance evidence.

### Performance evidence policy gate

Workflow: `.github/workflows/wake-word-performance-evidence.yml`

Current check: `node scripts/check_wake_word_performance_evidence.mjs`.

Purpose:

- version the performance-report schema and measurement policy
- require Linux x86_64 and macOS arm64 measurements before the report can move out of its pending state
- define required metrics for idle CPU, memory, inference latency, wake-to-ASR latency, pre-roll startup, repeated-cycle resource behavior, and continuous-ASR comparison
- preserve the one-thread inference policy

Current status:

- `docs/wake-word-performance-evidence.json` is intentionally `pending_measurement` with no measurements.
- A passing policy gate proves report structure/policy validity only; it does not prove WWR-630 performance acceptance.

### Privacy/security source audit gate

Workflow: `.github/workflows/wake-word-privacy-audit.yml`

Current check: `node scripts/check_wake_word_privacy_audit.mjs`.

Purpose:

- fail closed if Wake diagnostics gain raw PCM/transcript/credential/private-audio fields
- scan production Wake Word Rust error/log surfaces for direct logging macros and sensitive outward-facing string literals
- require sanitized runtime error behavior and path/token-like-secret sanitizer evidence
- prevent filesystem-path serialization from the diagnostics module
- require truthful privacy documentation and the corpus private-room-audio prohibition

Policy:

- This is an automated source/privacy guardrail.
- It supplements, but does not replace, the final WWR-900 source/privacy/security audit.

### Documentation truthfulness audit

Workflow: `.github/workflows/wake-word-documentation-audit.yml`

Current check: `node scripts/check_wake_word_documentation.mjs`.

Purpose:

- keep current-behavior documentation aligned with implemented and still-pending production integration
- preserve local/offline, active-microphone, cloud-boundary, and no-barge-in Settings disclosures
- reject unqualified final-acceptance claims while real fixtures/platform acceptance/performance remain pending

## Pending required acceptance evidence

The following evidence is still required before final Wake Word V1 closeout. Implemented policy or component gates above must not be confused with these production acceptance scenarios.

### Linux x86_64 real KWS acceptance

Required proof:

- prepare the exact pinned model and runtime and verify every hash
- verify ELF x86_64 runtime architecture
- verify CPU-only production path and one-thread policy
- run real positive and negative Wake Word fixtures
- prove inference succeeds offline after artifact preparation
- record privacy-safe diagnostics, exact commit, manifest, runner/platform details, and run ID

A component test or manifest check is not sufficient for this claim.

### macOS arm64 real KWS acceptance

Required proof mirrors Linux acceptance but must verify the Mach-O arm64 runtime and execute on macOS arm64. Linux evidence must not be reused as macOS evidence.

### Packaged-runtime load acceptance

The native packaging/architecture policy workflow is implemented. Final packaging acceptance still must verify the runtime library actually loaded by a packaged build on each claimed platform and must remain fail-closed for unsupported platforms.

### Integrated production lifecycle acceptance

The deterministic lifecycle workflow is implemented, but WWR-640 still requires integrated repeated wake→ASR→Thinking→Talking→wake cycles, resource-count observations, TTS success/cancel/failure resume behavior, disable/enable cycles, shutdown scenarios, and bounded soak behavior.

### Measured performance acceptance

The performance evidence policy gate is implemented, but WWR-630 remains pending until representative Linux and macOS measurements populate `docs/wake-word-performance-evidence.json` and demonstrate that idle KWS is lighter than continuously running full ASR.

### Final source/privacy/security audit

The automated privacy source gate is implemented, but final WWR-900 still requires review of runtime ownership, microphone transitions, cancellation/shutdown, ring clearing, Wake-disabled behavior, Talking suspension/resume, one-trigger/one-command behavior, provider separation, exact artifact loading, architecture verification, offline idle inference, and documentation truthfulness.

## Specialized runners and hardware

- Deterministic corpus schema/contract, performance-policy, privacy-source, and documentation gates run on ordinary hosted CI and do not constitute real KWS acceptance.
- Native packaging policy runs on hosted Linux x86_64 and macOS arm64; it validates policy/tests rather than real KWS audio acceptance.
- Deterministic lifecycle stability currently runs on hosted Linux CI and does not constitute a production audio soak.
- Linux x86_64 real KWS acceptance requires a runner/environment capable of loading and executing the pinned Linux native runtime and real redistributable fixtures.
- macOS arm64 real KWS acceptance requires an arm64 macOS runner/environment capable of loading and executing the pinned macOS native runtime and the same acceptance corpus policy.
- Representative performance evidence must be recorded on the acceptance environments; hosted policy validation cannot manufacture those measurements.

## Final merge eligibility

A final Wake Word V1 feature head is not eligible based on ordinary CI alone. Final closeout must record exact PR-head and exact merged-master evidence for every required Wake-specific gate and acceptance scenario applicable to the final feature claim.

A workflow with conclusion `skipped` is evidence only that its path filter or condition did not select that workflow. It is not evidence that the acceptance scenario passed. Likewise, a passing policy/schema gate is not evidence that still-pending real-world measurements or native acceptance passed.
