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
- Audit the former CPAL `unsafe impl Send` wrappers.
- CPAL 0.17.3 now supplies the stream thread-safety contract directly; the application-owned `SafeStream` wrappers and both manual `unsafe impl Send` assertions were removed.
- Existing capture/playback lifecycle regressions cover ownership and shutdown after the wrapper removal.
- V1R-006 development-tool advisory cleanup is complete.
- Gate P0 is complete.

Current evidence: V1R-004 publication `16e68280faddb3ffe5b5a689379d7cf9f1cc08a4` / CI `33192344369`, plus V1R-006 publication `2d61e7b5eade6ab27d2013b669a6ff2874da346c` / CI `33195721746`.

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
- Explicit cancellation is implemented and acceptance-covered: production `invoke_standalone_speech` routes through `synthesize_and_queue_cancellable`; `StandaloneSpeechController::cancel` cancels in-flight synthesis and flushes already-queued playback; Start, Barge-in, Mute, Dismiss, and the explicit cancel command use that same controller.
- `explicit_cancellation_aborts_in_flight_synthesis_and_flushes_playback` suspends the production cancellable synthesis helper on a never-completing synthesizer, cancels it, and proves the stable cancellation error plus zero queued/playing audio. `ambient_barge_in_flushes_standalone_audio_and_returns_to_idle` separately exercises the IPC interruption boundary with non-empty playback.
- Rate/pitch settings mapped into Gemini performance direction.
- No subprocess/browser/synthetic production fallback on provider failure.

Source/test acceptance evidence: the cancellation tests are part of the Rust suite exercised by later full repository CI, including run `33200290753`, which passed after the final V1R-014 change. This closes the stale source/test cancellation gap in this overlay. It does **not** replace the separate supported-Mac human cancellation checks in `MACOS_P6_VOICE_ACCEPTANCE.md`; those physical checks remain deferred with the voice listening pass.

### V1R-066 — Optional voice post-processing decision

Accepted complete:

- V1 decision is no additional local pitch/EQ/compression DSP.
- The no-DSP rationale and measured-condition reconsideration rule are documented.
- The conditional `if yes` DSP implementation criterion is not applicable to the V1 decision.

## Current residual P6 status — 2026-08-28

V1R-065 is source/test complete, including explicit cancellation behavior. P7 and the later source-level phases have already superseded the old "proceed next with P8" instruction. P6 still has a **supported-Mac physical acceptance tranche**: V1R-064 intentional human listening comparison across the supported Google voices plus the documented in-flight/playback cancellation listening checks. Those are human/device evidence, not remaining TTS implementation defects.

The repository-wide residual backlog is classified in `RECONCILIATION_REMAINING_NON_MAC_20260828.md`.

## Current next P1 acceptance checkpoint — 2026-08-28

P1 source-level remediation is complete. The next P1 task is **V1R-010 real macOS secure-store restart acceptance**: store/replace a Google API key through the production UI/backend, terminate the app process, relaunch it, prove the credential is still available for authentication without being returned to the frontend, and inspect the real SQLite/settings state to prove the plaintext key is absent. This must be recorded from a real supported Mac/process restart; ordinary CI is intentionally insufficient.
