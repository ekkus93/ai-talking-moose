# Wake Word V1 CI and Acceptance Gates

This document records the Wake Word V1 required gate inventory for historical WWR evidence and the authoritative post-closeout WPCR checklist in `docs/WAKE_WORD_V1_POST_CLOSEOUT_REMEDIATION_TODO_2026-09-25.md`. The machine-readable inventory lives in `docs/wake-word-required-gates.json` and is audited by `scripts/check_wake_word_required_gates.mjs`.

Ordinary CI alone is not final Wake Word V1 qualification; ordinary CI alone is not final Wake Word V1 qualification. A workflow with conclusion `skipped` is evidence only that its path filter or condition did not select that workflow. It is not evidence that the acceptance scenario passed.

## Required gate titles audited by the manifest

- Ordinary CI
- Wake Word required gates manifest audit
- Wake Word deterministic corpus manifest gate
- Wake Word corpus contract gate
- Native packaging/architecture policy gate
- Lifecycle stability gate
- Settings/listener lifecycle acceptance
- Manual shared-capture transfer acceptance
- Selected ASR policy acceptance
- Downstream first-command-word acceptance
- Clean-install artifact provisioning acceptance
- Performance evidence policy gate
- Production listener performance evidence
- Privacy/security source audit gate
- Documentation truthfulness audit
- Source/security ownership audit gate
- Linux x86_64 real KWS acceptance
- macOS arm64 real KWS acceptance
- Integrated production lifecycle acceptance
- Measured performance acceptance

## Gate boundaries

Ordinary CI (`.github/workflows/ci.yml`) runs formatting, linting, Rust tests, frontend tests, generated contract checks, and selected packaging/dependency checks. It is required for mergeable Wake Word changes, but it is not final acceptance by itself.

The Wake Word required gates manifest audit (`.github/workflows/wake-word-required-gates.yml`) validates gate IDs, titles, exact-head requirements, workflow references, pull-request triggers, and the rule that skipped workflow conclusions never count as passing evidence.

The Wake Word corpus contract and deterministic corpus manifest gates validate fixture schema, provenance, license, privacy, SHA-256 identities, and expected detection outcomes. A skipped corpus gate is not evidence that real corpus acceptance passed. Passing schema/contract gates do not mean the repository contains real audio fixtures or final performance acceptance.

The Native packaging/architecture policy gate (`.github/workflows/wake-word-native-packaging.yml`) validates pinned runtime manifests, architecture rejection, cache/hash behavior, and clean-install fail-closed artifact behavior. This gate does not by itself prove real KWS inference or packaged runtime loading.

The Lifecycle stability gate (`.github/workflows/wake-word-lifecycle-stability.yml`) runs deterministic runtime-manager lifecycle tests for repeated wake/command/resume cycles, disable/enable, shutdown, and bounded retained-audio/resource state.

Settings/listener lifecycle acceptance runs through ordinary CI and covers live Settings enable/disable wiring, listener diagnostics, backend/frontend disclosure state, and generated contract compatibility.

Manual shared-capture transfer acceptance runs through ordinary CI and covers manual command ownership transfer, Wake listener stop/resume boundaries, no duplicate microphone ownership, and command-transfer shutdown that is not recorded as a Wake capture failure.

Selected ASR policy acceptance preserves Wake V1 Policy B: Wake-triggered command ASR is local-Moonshine-only. It covers unsupported ASR mode rejection, Settings disclosure, and the distinction between ordinary manual ASR choices and Wake-triggered command-ASR eligibility.

Downstream first-command-word acceptance covers deterministic router and local-ASR ingress tests proving wake pre-roll, trigger frame, and the first command word reach the downstream command boundary as one ordered utterance. It distinguishes deterministic boundary receipt from real ASR transcription accuracy.

Clean-install artifact provisioning acceptance runs through native packaging policy and covers empty app-data behavior, corrupt-cache handling, architecture mismatch rejection, developer-prepared provisioning disclosure, and fail-closed verification before any Listening claim.

The Performance evidence policy gate (`.github/workflows/wake-word-performance-evidence.yml`) validates the report schema. `docs/wake-word-performance-evidence.json` is `accepted` with Linux x86_64 and macOS arm64 platform baselines. Cross-cutting exact-run evidence records wake→command-ASR latency, pre-roll startup timing, and repeated-cycle resource behavior.

Production listener performance evidence also runs through `.github/workflows/wake-word-performance-evidence.yml` and requires representative production native listener measurements for startup duration, idle capture CPU, memory overhead, route/inference latency, wake-to-command-ASR activation, pre-roll startup, repeated enable/disable and wake/command/resume cycles, and continuous full-ASR comparison. Hosted policy validation cannot manufacture representative measurements.

The Privacy/security source audit gate (`.github/workflows/wake-word-privacy-audit.yml`) checks diagnostic privacy, sanitized error surfaces, private-audio guardrails, and production Wake Word Rust error/log surfaces.

The Source/security ownership audit gate (`.github/workflows/wake-word-source-security-audit.yml`) checks single runtime ownership, shared `AudioCapture` ownership, capture routing, artifact/architecture checks, local-Moonshine-only Wake ASR policy, and absence of idle network/full-ASR dependency.

The Documentation truthfulness audit (`.github/workflows/wake-word-documentation-audit.yml`) keeps docs aligned with the post-closeout WPCR checklist, local/offline boundaries, active microphone disclosure, local-Moonshine Wake ASR policy, clean-install provisioning limits, and pending final qualification.

## Real acceptance gates and final qualification

Linux x86_64 real KWS acceptance and macOS arm64 real KWS acceptance both run through `.github/workflows/wake-word-real-kws.yml` and require exact-head evidence on runners able to load and execute the pinned native runtime and deterministic generated fixtures.

Integrated production lifecycle acceptance runs through `.github/workflows/wake-word-lifecycle-stability.yml` and must be cited separately from component tests. Measured performance acceptance and Production listener performance evidence run through `.github/workflows/wake-word-performance-evidence.yml` and must cite exact reports.

WPCR-950/960 final closeout must record exact PR-head and exact merged-master evidence for every required Wake-specific gate and reopened acceptance scenario. A passing policy/schema gate is not evidence that still-pending real-world measurements, hardware acceptance, or source/privacy review passed.
