# WWR-800 — Wake required gates evidence matrix

Commit audited: `d5403747c79fcb1e0cd61a8d23e31fdbe3370827`

This file records the current Wake Word V1 specialized-gate inventory after the current WWR-800 gate inventory merge. It is evidence for WWR-800 gate infrastructure only. It is not final qualification evidence for WWR-610, WWR-620, WWR-630, WWR-640, WWR-950, or WWR-960.

## Exact merged-master validation observed

- Ordinary CI: run `35986409622`, success on `d5403747c79fcb1e0cd61a8d23e31fdbe3370827`.

## Authoritative manifest

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

## Pending specialized acceptance gates

The following gates remain intentionally pending in `docs/wake-word-required-gates.json` and must not be treated as passed by ordinary CI or by policy/report validators alone:

- `linux_real_kws_acceptance` — pending specialized runner acceptance; requires real positive and negative pinned sherpa KWS inference on Linux x86_64.
- `macos_real_kws_acceptance` — pending specialized runner acceptance; requires real positive and negative pinned sherpa KWS inference on macOS arm64.
- `integrated_production_lifecycle_acceptance` — pending integrated acceptance; requires repeated wake→ASR→Thinking→Talking→wake cycles with resource counts.
- `measured_performance_acceptance` — pending measurement; requires representative Linux and macOS CPU, memory, latency, repeated-cycle, and continuous-ASR comparison measurements.

## Current blocker note

The open real-KWS acceptance harness branch has already produced green real KWS, lifecycle, corpus, privacy, source-security, required-gates, and lockfile-consistency runs on its exact head, but ordinary dependency audit remains blocked by `RUSTSEC-2026-0285` until `src-tauri/Cargo.lock` is updated from `rustls 0.23.44` to `0.23.45` without weakening audit policy.

## WWR-800 reconciliation guidance

This matrix supports only the mechanically proven manifest/gate-inventory and skipped-policy portions of WWR-800. It does not support marking final WWR-800 acceptance complete because several specialized real/native/integrated/performance gates remain pending by design.

Final feature qualification must still collect exact-head run IDs and reports for every required gate in `docs/wake-word-required-gates.json`; skipped required gates cannot be counted as passing evidence.
