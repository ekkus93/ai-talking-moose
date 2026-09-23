# WWR-800 — Wake required gates evidence matrix

Commit audited: `b6c33cd999191bfedcc6358f4f80165b5c9e937c`

This file records the current Wake Word V1 specialized-gate inventory after PR #412. It is evidence for WWR-800 gate infrastructure only. It is not a final qualification claim for WWR-610, WWR-620, WWR-630, WWR-640, WWR-950, or WWR-960.

## Exact merged-master validation observed

- Ordinary CI: run `35888332561`, success on `b6c33cd999191bfedcc6358f4f80165b5c9e937c`.
- Wake Word qualification report: run `35888332515`, success on `b6c33cd999191bfedcc6358f4f80165b5c9e937c`.
- Wake Word required gates audit: run `35888332562`, success on `b6c33cd999191bfedcc6358f4f80165b5c9e937c`.

## Implemented exact-head gate inventory

The authoritative machine-readable inventory is `docs/wake-word-required-gates.json`. At the audited commit it records:

- `ordinary_ci` — implemented by `.github/workflows/ci.yml`; required for final closeout; exact-head required; skipped conclusions do not count as pass.
- `required_gates_manifest` — implemented by `.github/workflows/wake-word-required-gates.yml`; audits the required-gate manifest and drift policy.
- `deterministic_corpus_manifest` — implemented by `.github/workflows/wake-word-corpus.yml`; validates fixture schema/provenance/license/privacy and pending acceptance criteria policy.
- `deterministic_corpus_contract` — implemented by `.github/workflows/wake-word-corpus-contract.yml`; validates the companion Python contract for `docs/wake-word-corpus.json`.
- `native_packaging_architecture_policy` — implemented by `.github/workflows/wake-word-native-packaging.yml`; validates hosted native packaging and architecture policy for Linux x86_64 and macOS arm64 without treating that as real KWS inference acceptance.
- `lifecycle_stability_policy` — implemented by `.github/workflows/wake-word-lifecycle-stability.yml`; validates deterministic runtime-manager state-machine stability tests.
- `performance_evidence_policy` — implemented by `.github/workflows/wake-word-performance-evidence.yml`; validates performance-report schema and pending-measurement policy.
- `privacy_audit` — implemented by `.github/workflows/wake-word-privacy-audit.yml`; validates diagnostic privacy, sanitized error surfaces, and private-audio guardrails.
- `documentation_audit` — implemented by `.github/workflows/wake-word-documentation-audit.yml`; validates truthfulness while real acceptance remains pending.
- `source_security_audit` — implemented by `.github/workflows/wake-word-source-security-audit.yml`; validates source ownership, capture routing, artifact/architecture checks, and no idle network/full-ASR dependency.
- `qualification_report` — implemented by `.github/workflows/wake-word-qualification-report.yml`; added by PR #412 and validates the reusable JSON report validator for corpus, native, lifecycle, performance, and privacy acceptance evidence.

## Pending specialized acceptance gates

The following gates remain intentionally pending in `docs/wake-word-required-gates.json` and must not be treated as passed by ordinary CI or by policy/report validators alone:

- `linux_real_kws_acceptance` — pending specialized runner acceptance; requires real positive and negative pinned sherpa KWS inference on Linux x86_64.
- `macos_real_kws_acceptance` — pending specialized runner acceptance; requires real positive and negative pinned sherpa KWS inference on macOS arm64.
- `integrated_production_lifecycle_acceptance` — pending integrated acceptance; requires repeated wake→ASR→Thinking→Talking→wake cycles with resource counts.
- `measured_performance_acceptance` — pending measurement; requires representative Linux and macOS CPU, memory, latency, repeated-cycle, and continuous-ASR comparison measurements.

## WWR-800 reconciliation guidance

The implemented gate inventory supports marking only the mechanically proven gate-definition and exact-head/skipped-policy portions of WWR-800 after a TODO reconciliation PR updates the checklist with this evidence and the exact run IDs above. It does not support marking the final WWR-800 acceptance complete because several specialized real/native/integrated/performance gates remain pending by design.

Final feature qualification must still collect exact-head run IDs and reports for every required gate in `docs/wake-word-required-gates.json`; skipped required gates cannot be counted as passing evidence.
