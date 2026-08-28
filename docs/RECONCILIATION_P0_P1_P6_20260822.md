# P0 / P1 / P6 Reconciliation — 2026-08-22

This file records the accepted reconciliation against `docs/TODO(20260818-163801).md` after the user-reported passing CI checkpoint `32596720054` on commit `a964ccd645ed805e889dd72bc6adc0f230a5b2d3`.

The timestamped TODO remains the comprehensive task inventory. This overlay is authoritative for the rows below until these status changes are mechanically folded back into that monolithic tracker. Manual/device criteria remain open unless explicitly listed as verified.

## 2026-08-28 P1 evidence refresh

The P1 accepted-complete claims below are now backed by the final remediation changes that closed the two source-level gaps discovered during the later P1 re-audit:

- **V1R-013 provider-error sanitization:** PR #9 head `3cc5ad4d7450e373b1d5c7483a889946a5e17f5b` converted remaining Google Text/TTS/Live failures to structured, bounded provider categories, prevented raw transport/parser details from crossing normal logging and IPC boundaries, and added private-sentinel logging regressions. Full repository CI run `33199001645` passed frontend quality, dependency audits, Rustfmt, Clippy, the full Rust suite, backend failure/stress matrices, release metadata/license collection, and both macOS bundle architectures. PR #9 merged to `master` as `6a631f0d05dc42ada94569c7a09dd9fccb541fb2`.
- **V1R-014 diagnostic microphone teardown:** PR #10 head `0d523a6bebaf504a58126f689ce5fde4e03e3239` added a production `DiagnosticCaptureLease` RAII guard so cancellation, abort, unwind, or future early exit cannot bypass `AudioCapture::stop()` after Settings diagnostic capture starts. Its async regression aborts the diagnostic owner while suspended and proves capture becomes inactive. Full repository CI run `33200290753` passed. PR #10 merged to `master` as `2194a5ee072f9fc835c3365388bc16bfa2e41437`.

Accordingly, the P1 privacy-logging regression and fresh-profile/privacy acceptance sub-gates remain accepted complete. The only P1 gate still open is the **real macOS secure-store restart acceptance** and its associated real-process/real-database checks; automated CI is not treated as a substitute for that evidence.

## P0 — Repository quality baseline

### V1R-004 — Audit lint suppressions and unsafe assertions

Accepted complete:

- Inventory every Rust `allow(...)` / lint suppression.
- Remove suppressions that mask fixable warnings.
- Document rationale for necessary suppressions.
- Audit the CPAL `unsafe impl Send` wrappers.
- Add ownership/shutdown regression tests for retained unsafe wrappers.

Still open:

- Prefer architecture that removes manual unsafe thread-safety assertions where practical.
- Acceptance: no unexplained production lint suppression or unsafe thread-safety assertion remains.
- V1R-006 development-tool advisory cleanup.
- Gate P0.

## P1 — Security and privacy repair

Accepted complete:

- V1R-010 acceptance: frontend never receives the stored secret.
- V1R-013: memory facts are absent from normal logs.
- V1R-013: desktop observations/window titles are absent from normal logs.
- V1R-013: remaining provider failures use structured/sanitized categories where required.
- V1R-013: captured-log regression intentionally exercises transcript/prompt/secret-like/private observer values.
- V1R-014: fresh-profile regression covers conservative defaults.
- V1R-014: onboarding/privacy copy reflects those defaults.
- V1R-014: microphone lifecycle regression proves capture is limited to conversation or explicit diagnostics.
- Gate P1: privacy logging regression suite complete.
- Gate P1: fresh-profile privacy acceptance complete.

Still open:

- Real macOS Keychain persistence across an actual process restart.
- Restart and authenticate with a previously stored key.
- Inspect real SQLite/settings state and prove the key is absent.
- Gate P1 real macOS secure-store restart acceptance.

## P6 — Character, prompting, and voice

### V1R-060 — Authoritative character config

Accepted complete:

- One validated Rust character/personality configuration source.
- Settings mapping complete.
- Duplicated/divergent defaults removed.
- Serialization/default-mapping regression coverage.

### V1R-061 — Prompt/context budgets

Accepted complete:

- Hard system/personality/memory/observation budgets.
- Tool/transcript context remains fail-closed because `PromptBuilder` exposes no such input surface in V1; no hidden unbounded injection path exists.
- Deterministic truncation order.
- Privacy gates applied before prompt assembly.
- Oversized-context regression coverage.

### V1R-062 — Character behavior regression suite

Accepted complete:

- Dry/sarcastic/friendly/absurd/helpful/verbosity/talkativeness mapping coverage.
- Quiet-hours/annoyance/dismissal behavior coverage.
- Model-generated text cannot bypass local deny policy.

### V1R-063 — Voice audition corpus

Accepted complete:

- One fixed original Moose audition script.
- Greeting, sarcasm, annoyed/comedic, explanatory, short and long material represented.
- Fake/offline path available to ordinary tests.

### V1R-064 — Select final original Moose voice

Accepted complete:

- Chosen V1 default and rationale recorded.
- Explicit non-imitation rule for Bullwinkle, Bill Scott, and other identifiable performers.

Still open:

- Real intentional listening comparison across supported Google voices.

### V1R-065 — Standalone TTS hardening

Accepted complete:

- Rust playback path authoritative.
- Centralized TTS model/voice catalog and validation.
- Typed provider errors.
- Bounded request timeout.
- Rate/pitch settings mapped into Gemini performance direction.
- No subprocess/browser/synthetic production fallback on provider failure.

Still open:

- Explicit cancellation acceptance in addition to the implemented timeout.

### V1R-066 — Optional voice post-processing decision

Accepted complete:

- V1 decision is no additional local pitch/EQ/compression DSP.
- The no-DSP rationale and measured-condition reconsideration rule are documented.
- The conditional `if yes` DSP implementation criterion is not applicable to the V1 decision.

## Next implementation checkpoint

P7 ambient behavior is already accepted complete under `RECONCILIATION_P7_20260822.md`; the 2026-08-23 audit additionally repairs ambient-only barge-in cancellation after the P6 standalone-TTS changes. With P6 real listening/cancellation acceptance intentionally left open for later physical testing, proceed next with P8 real desktop observation.

## Current next P1 acceptance checkpoint — 2026-08-28

P1 source-level remediation is complete. The next P1 task is **V1R-010 real macOS secure-store restart acceptance**: store/replace a Google API key through the production UI/backend, terminate the app process, relaunch it, prove the credential is still available for authentication without being returned to the frontend, and inspect the real SQLite/settings state to prove the plaintext key is absent. This must be recorded from a real supported Mac/process restart; ordinary CI is intentionally insufficient.
