# WWR-630 — command activation timing evidence

Date: 2026-09-24
Status: partial WWR-630 evidence, not final performance acceptance

## Scope

This evidence records the command-activation timing seam added for Wake Word performance reporting. It is intentionally partial: it does not close WWR-630 because the accepted representative performance baseline still requires platform measurements for the remaining metrics and the KWS-vs-continuous-ASR comparison.

## Implementation evidence

PR #443 added privacy-safe timing metadata to `src-tauri/src/app/wake_word_command_activation.rs`:

- `WakeCommandActivationTiming` records bounded numeric timing only.
- `wake_to_command_asr_ms` covers the elapsed time for the single-use Wake handoff to reach the command-ASR ingress.
- `command_start_ms` covers the normal command-start boundary after successful handoff delivery.
- `total_activation_ms` covers the whole Wake activation wrapper.
- The existing `activate_wake_command_and_start_normal_asr_once` behavior remains source-compatible and delegates through the measured wrapper.
- Tests verify successful timing capture and the consumed-handoff/no-command-start case.

The timing record contains no PCM, transcript text, filesystem path, credential, provider payload, or model/runtime secret. It is suitable for later WWR-630 aggregation into `docs/wake-word-performance-evidence.json` once the remaining metrics are measured.

## Exact-head qualification

PR #443 exact head: `a4cd0ffd636536b5b2a422cdc757ce7422cf5050`.

Exact-head validation:

- Ordinary CI: `36063972014` — success.
- Wake Word lifecycle stability: `36063972064` — success.
- Wake Word source-security audit: `36063972020` — success.
- P21-P23 Rust Stability Acceptance: `36063972084` — skipped and unrelated to this Wake Word scope; it is not counted as acceptance evidence.

## Exact-master qualification

PR #443 was merged by guarded squash as master `2d35a6662f3ed6c01d526bc438676b9ed1ce1971`.

Exact-master validation:

- Ordinary CI: `36067453325` — success.
- Wake Word lifecycle stability: `36067453316` — success.
- Wake Word source-security audit: `36067453264` — success.

## WWR-630 status after this evidence

This evidence advances the instrumentation path for these pending metrics:

- wake detection → command ASR activation latency;
- pre-roll/command-ASR ingress timing, via the same measured handoff boundary.

It does not yet satisfy the WWR-630 acceptance criteria. The following remain open:

- accepted representative Linux and macOS performance measurements in `docs/wake-word-performance-evidence.json`;
- repeated-cycle resource delta aggregation into the accepted report;
- continuous full-ASR idle CPU comparison;
- final proof that idle KWS is lighter than continuous full ASR;
- final WWR-630 remediation TODO reconciliation.

## Non-goals

This evidence file must not be used to claim that WWR-630 is complete, that the performance baseline is accepted, or that the final Wake Word V1 closeout is eligible without the remaining exact-head and exact-master gates.
