# MCR-950 Final Qualification Evidence — 2026-10-01

**Qualification head:** `b05acecc0174abd241b60f7b8c5ada789015db5d`
**Baseline:** `9fc10b488c47ad52bb9960bc9181dfe7763a6d9c`
**Status:** Exact-head qualification complete.

## Scope review

The remediation diff from the review baseline through qualification head `b05acecc0174abd241b60f7b8c5ada789015db5d` contains the MCR implementation, deterministic regressions, security/policy checkers, machine-readable Wake performance correction, documentation/evidence, formatting-only follow-up fixes, and behavior-neutral workflow-comment triggers used to select the complete qualification matrix. The historical Wake V1 closeout remains unchanged.

The final qualification trigger strategy follows the repository's established WWR-950 precedent: behavior-neutral workflow-comment changes select mandatory Wake workflows through their normal path filters. No skipped workflow is counted as passing.

## Unified exact-head final matrix

Every specialized workflow below completed successfully on the same exact qualification SHA `b05acecc0174abd241b60f7b8c5ada789015db5d`:

- [x] Ordinary CI — `36866839171`.
- [x] Wake Artifact Verification — `36866839152`.
- [x] Wake deterministic corpus validation — `36866838999`.
- [x] Wake corpus contract — `36866839246`.
- [x] Wake native packaging/architecture — `36866838977`.
- [x] Wake lifecycle stability — `36866839058`.
- [x] Wake production performance evidence — `36866839098`.
- [x] Wake privacy audit — `36866839003`.
- [x] Wake source-security audit — `36866839007`.
- [x] Wake documentation audit — `36866838959`.
- [x] Wake required-gates audit — `36866839085`.
- [x] Wake runtime identity freeze — `36866839070`.
- [x] Wake real KWS acceptance — `36866838980`.

Real-KWS acceptance `36866838980` passed both required native jobs: Linux x86_64 job `110384459768` and macOS arm64 job `110384460020`. Both built the exact-head acceptance binary, prepared and verified pinned model/runtime inputs, ran real pinned sherpa KWS inference, required command-fixture detection, extracted reusable performance measurements, ran production-listener performance acceptance, and uploaded exact-head reports.

## Focused MCR qualification inside ordinary CI

Ordinary CI `36866839171` passed on the same exact head and exercises the accumulated deterministic regression suites and repository policy checks added by this remediation. In conjunction with the source/test audit in `docs/evidence/MCR-900_FINAL_SOURCE_PRIVACY_SECURITY_AUDIT_2026-10-01.md`, this qualifies the focused MCR requirements for:

- conversation cancellation/local-ASR reserve-await-commit behavior;
- blocking built-in timeout isolation and bounded error handling;
- Moonshine/local-LLM/local-TTS installer timeout/security policy and packaging invariants;
- frontend settings persistence rollback/result behavior;
- frontend listener-registration cleanup and exception safety;
- Tauri CSP/capability security policy; and
- placeholder-test rejection policy.

The Wake lifecycle workflow `36866839058` additionally qualifies the exact-head native Wake lifecycle/concurrency path. The production performance workflow `36866839098` additionally qualifies authoritative Linux x86_64 and macOS arm64 production-listener measurements rather than standalone-only evidence.

## Exact-head conclusion

MCR-950 is objectively satisfied on exact qualification head `b05acecc0174abd241b60f7b8c5ada789015db5d`: every required specialized workflow selected for final qualification completed successfully on one SHA, ordinary CI passed, both native real-KWS platforms passed, and no skipped required gate was treated as success.

MCR-960 remains a separate exact-master closeout step. Documentation-only reconciliation commits after this qualification do not change the qualified production source; they must still receive the exact-master ordinary/documentation gates required by their changed scope before final closeout.
