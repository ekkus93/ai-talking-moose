# Wake Word V1 CI and Acceptance Gates

This document records the current Wake Word V1 gate inventory and the merge-eligibility policy implied by `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`.

The machine-readable inventory lives in `docs/wake-word-required-gates.json` and is audited by `scripts/check_wake_word_required_gates.mjs`. The manifest is intentionally explicit that ordinary CI alone is not final Wake Word V1 qualification and that a workflow conclusion of `skipped` never counts as passed acceptance evidence.

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

### Wake Word required gates manifest audit

Workflow: `.github/workflows/wake-word-required-gates.yml`

Current check: `node scripts/check_wake_word_required_gates.mjs`.

Purpose:

- validate the versioned required-gate inventory in `docs/wake-word-required-gates.json`
- verify implemented gates point at real workflow files and include pull-request triggers
- verify every final-closeout gate requires exact-head evidence
- verify skipped workflow conclusions are never treated as passing acceptance evidence
- preserve required specialized-runner entries for Linux real KWS, macOS real KWS, integrated lifecycle, and measured performance acceptance

Audited manifest gate titles:

- Wake Word deterministic corpus manifest gate
- Wake Word corpus contract gate
- Native packaging/architecture policy gate
- Lifecycle stability gate
- Performance evidence policy gate
- Privacy/security source audit gate
- Documentation truthfulness audit
- Source/security ownership audit gate
- Linux x86_64 real KWS acceptance
- macOS arm64 real KWS acceptance
- Integrated production lifecycle acceptance
- Measured performance acceptance

Policy:

- This is a policy/source-of-truth gate; it records which real KWS, lifecycle, and performance gates are required for final closeout.
- A passing manifest audit means the gate inventory is internally consistent; it does not replace the exact-head run evidence required by WWR-950.

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
- keep recall/false-accept criteria explicit and versioned
- keep the legacy Python corpus contract aligned with the active `docs/wake-word-corpus.json` schema

Policy:

- These gates must pass when their path filters select them.
- A skipped corpus gate is not evidence that real corpus acceptance passed.
- Passing schema/contract gates do not mean the repository contains real audio fixtures or final performance acceptance.

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
- Final packaged-runtime evidence remains separate from real KWS acceptance.

### Real KWS acceptance

Workflow: `.github/workflows/wake-word-real-kws.yml`

Matrix:

- Linux x86_64 on `ubuntu-latest`
- macOS arm64 on `macos-15`

Current checks:

- generate the deterministic Wake Word V1 PCM corpus from `docs/wake-word-corpus.json`
- build the exact-head `wake_word_acceptance` binary
- prepare and verify the pinned model/runtime artifacts
- assert the host architecture for each matrix target
- run positive and negative real pinned sherpa KWS inference offline
- upload privacy-safe corpus and per-platform real KWS acceptance reports

Purpose:

- bind Linux x86_64 and macOS arm64 real KWS acceptance to an exact commit
- exercise the production native KWS boundary with the frozen model/runtime identities, one-thread policy, score `1.0`, and threshold `0.25`
- prove both recall and false-trigger behavior on the deterministic generated fixture set instead of a single happy path

Policy:

- The workflow is a required final-closeout gate for WWR-610 and WWR-620.
- A skipped real-KWS workflow is not acceptance evidence; final closeout must record exact run IDs and artifacts for both matrix targets.

### Lifecycle stability gate

Workflow: `.github/workflows/wake-word-lifecycle-stability.yml`

Current check: `cargo test --manifest-path src-tauri/Cargo.toml --all-targets --all-features wake_word_stability -- --nocapture`.

Purpose:

- exact-head targeted execution of deterministic Wake Word runtime-manager stability tests
- exercise bounded lifecycle/state-machine invariants independently of broad ordinary CI
- prove repeated wake→ASR→Thinking→Talking→wake cycles, TTS terminal outcomes, disable/enable cycles, shutdown scenarios, and bounded retained audio/resource state for the defined deterministic acceptance

Policy:

- Changes to authoritative Wake runtime/lifecycle paths select this gate.
- A successful deterministic stability run is required WWR-640 evidence for the defined lifecycle acceptance.
- A skipped lifecycle workflow is not lifecycle acceptance evidence.

### Selected ASR policy acceptance

Current checks:

- ordinary Rust tests in `src-tauri/src/app/wake_word_state.rs`
- frontend disclosure tests in `src/components/Settings/WakeWordSettingsPanel.test.tsx`
- documentation truthfulness audit in `.github/workflows/wake-word-documentation-audit.yml`
- source/security ownership audit in `.github/workflows/wake-word-source-security-audit.yml`

Purpose:

- preserve the selected Wake Word V1 Policy B: Wake-triggered command ASR supports local Moonshine streaming modes only
- verify Gemini Live audio and other unsupported command ASR modes cannot start or keep the Wake listener active
- keep user-facing Settings disclosure aligned with the selected policy
- distinguish ordinary manual ASR choices from Wake-triggered command-ASR eligibility

Policy:

- A passing policy/unit/disclosure path proves the selected Wake ASR policy boundary, not full command transcription accuracy.
- Wake-triggered Gemini Live audio is not a supported V1 claim and must not be advertised by documentation or Settings UI.
- Manual command interaction remains outside the Wake-triggered command-ASR restriction.

### Performance evidence policy gate

Workflow: `.github/workflows/wake-word-performance-evidence.yml`

Current check: `node scripts/check_wake_word_performance_evidence.mjs`.

Purpose:

- version the performance-report schema and measurement policy
- require Linux x86_64 and macOS arm64 measurements before the report can move out of its pending state
- define required metrics for idle CPU, memory, inference latency, wake-to-ASR latency, pre-roll startup, repeated-cycle resource behavior, and continuous-ASR comparison
- preserve the one-thread inference policy

Current status:

- `docs/wake-word-performance-evidence.json` is `accepted` with Linux x86_64 and macOS arm64 platform baselines.
- Platform baselines record idle KWS CPU, runtime memory, inference latency, continuous-ASR idle CPU comparison, and the one-thread policy.
- Cross-cutting exact-run evidence records wake→command-ASR latency, pre-roll startup timing, and repeated-cycle resource behavior.

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

### Source/security ownership audit gate

Workflow: `.github/workflows/wake-word-source-security-audit.yml`

Current check: `node scripts/check_wake_word_source_security_audit.mjs`.

Purpose:

- preserve the single authoritative `WakeWordRuntimeManager` definition
- preserve shared `AudioCapture` ownership without a competing Wake-specific capture owner
- verify lifecycle, shutdown, Talking suspension, and one-trigger router boundaries stay present
- verify model/runtime identity checks and native architecture verification boundaries stay present
- verify local Moonshine-only Wake command-ASR policy enforcement stays present
- reject network or cloud/full-ASR references in the production KWS engine boundary

Policy:

- This automated source audit advances WWR-900 coverage but does not replace real production audio acceptance or final manual source review.

### Documentation truthfulness audit

Workflow: `.github/workflows/wake-word-documentation-audit.yml`

Current check: `node scripts/check_wake_word_documentation.mjs`.

Purpose:

- keep current-behavior documentation aligned with implemented and still-pending production integration
- preserve local/offline, active-microphone, cloud-boundary, local-Moonshine Wake ASR policy, and no-barge-in Settings disclosures
- reject unqualified final-acceptance claims while performance/final-audit/final-closeout work remains pending

## Pending required acceptance evidence

The following evidence is still required before final Wake Word V1 closeout. Implemented policy or component gates above must not be confused with these production acceptance scenarios.

### Packaged-runtime load acceptance

The native packaging/architecture policy workflow is implemented. Final packaging acceptance still must verify the runtime library actually loaded by a packaged build on each claimed platform and must remain fail-closed for unsupported platforms.

### Final source/privacy/security audit

The automated privacy and source/security gates are implemented, but final WWR-900 still requires review of runtime ownership, microphone transitions, cancellation/shutdown, ring clearing, Wake-disabled behavior, Talking suspension/resume, one-trigger/one-command behavior, selected local-Moonshine ASR policy, provider separation, exact artifact loading, architecture verification, offline idle inference, and documentation truthfulness.

## Specialized runners and hardware

- Deterministic corpus schema/contract, performance-policy, privacy-source, source/security, required-gates, and documentation gates run on ordinary hosted CI and do not constitute real KWS acceptance.
- Native packaging policy runs on hosted Linux x86_64 and macOS arm64; it validates policy/tests rather than real KWS audio acceptance.
- Lifecycle stability runs on hosted Linux CI and records deterministic lifecycle acceptance for the defined state-machine/resource scenarios.
- Linux x86_64 real KWS acceptance runs through `.github/workflows/wake-word-real-kws.yml` on a runner/environment capable of loading and executing the pinned Linux native runtime and deterministic generated fixtures.
- macOS arm64 real KWS acceptance runs through `.github/workflows/wake-word-real-kws.yml` on an arm64 macOS runner/environment capable of loading and executing the pinned macOS native runtime and the same deterministic generated fixture policy.
- Representative performance evidence must be recorded on the acceptance environments; hosted policy validation cannot manufacture those measurements.

## Final merge eligibility

A final Wake Word V1 feature head is not eligible based on ordinary CI alone. Final closeout must record exact PR-head and exact merged-master evidence for every required Wake-specific gate and acceptance scenario applicable to the final feature claim.

A workflow with conclusion `skipped` is evidence only that its path filter or condition did not select that workflow. It is not evidence that the acceptance scenario passed. Likewise, a passing policy/schema gate is not evidence that still-pending real-world measurements or native acceptance passed.
