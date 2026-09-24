# WWR-900 — Source/privacy/security audit pass 1

Date: 2026-09-24
Base master: `145bf21d9baeb2b5f58fb50ae079e3094d11f4c6`
Scope: static source/evidence audit only. This file does not claim final WWR-900 acceptance and does not mark any remediation checkbox complete by itself.

## Audited evidence and source areas

- `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`
- `docs/WAKE_WORD_V1.md`
- `docs/evidence/WWR-020_WAKE_WORD_ARCHITECTURE_CONSOLIDATION_2026-09-17.md`
- `docs/evidence/WWR-100_MODEL_IDENTITY_2026-09-17.md`
- `docs/evidence/WWR-110_SHERPA_RUNTIME_IDENTITY_2026-09-17.md`
- `docs/evidence/WWR-300_APP_STATE_CAPTURE_COMPOSITION_2026-09-22.md`
- `docs/evidence/WWR-300_CAPTURE_RECOVERY_CYCLES_2026-09-22.md`
- `docs/evidence/WWR-310_HANDOFF_BOUNDARY_2026-09-22.md`
- `docs/evidence/WWR-410_DEBOUNCE_TRIGGER_SEMANTICS_2026-09-22.md`
- `docs/evidence/WWR-510_TODO_RECONCILIATION_2026-09-22.md`
- `docs/evidence/WWR-510_WAKE_PRIVACY_LOG_AUDIT_2026-09-21.md`
- `docs/evidence/WWR-700_DOCUMENTATION_AUDIT_2026-09-23.md`
- `docs/evidence/WWR-800_CONVERSATION_PATH_GATE_COVERAGE_2026-09-24.md`
- `src-tauri/src/app/wake_word*`
- `src-tauri/src/app/state.rs`
- `src-tauri/src/commands/conversation/*`
- `src-tauri/src/asr/pipeline.rs`
- `src-tauri/src/conversation/session*`
- `wake-word-artifacts.json`
- `scripts/prepare_wake_word_artifacts.py`
- `scripts/prepare_wake_word_runtime.py`

## Findings already supported by merged source/evidence

### Single Wake runtime ownership

Merged evidence supports one authoritative Wake Word subsystem and one authoritative `WakeWordRuntimeManager` under the consolidated `src-tauri/src/app/wake_word/` boundary. The legacy duplicate `src-tauri/src/app/wake_word_runtime.rs` remains only as a tracked marker that points to the canonical runtime and is not declared by `app::mod` as an implementation module. Current production-facing composition stores `WakeWordApplicationRuntime` in `AppState`, and the canonical capture/routing modules share that same runtime manager clone.

Status for final WWR-900: partially supported. Final audit still must be repeated after production startup/listening wiring and real native acceptance are complete.

### Microphone ownership transitions

Merged evidence supports the intended ownership model: Wake capture, manual conversation, diagnostics, and command ASR all use `AppState::audio_capture`; Wake-specific capture owners are receive-side orchestration around that same shared capture object rather than independent physical microphone owners. The deterministic capture recovery/cycle tests cover repeated wake-command-wake ownership transfer, disable/release for manual listen, cancellation return, closed-queue failure, and restart through the same capture owner.

Status for final WWR-900: partially supported. Hardware disconnect/reconnect acceptance and final production startup/listening integration remain open.

### Cancellation/shutdown

Merged source paths call Wake runtime shutdown during application exit, and deterministic Wake lifecycle tests cover shutdown while listening and shutdown during triggered handoff. Conversation start failure, stop, and terminal lifecycle callbacks resume or disable Wake according to the latest persisted setting.

Status for final WWR-900: partially supported. Final audit must bind these checks to the final production activation path and exact-head lifecycle gate.

### Ring-buffer clearing

Merged runtime and composition tests cover ring/pre-roll clearing on Talking suspension, disable, capture error, shutdown, stale handoff failure, and terminal interaction resolution. WWR-310 evidence covers stale handoff clearing after success/failure/cancellation at the handoff boundary.

Status for final WWR-900: supported for deterministic source paths; final integrated acceptance remains open.

### Wake-disabled behavior

Settings/UI evidence and command lifecycle tests support disabled-by-default behavior, invalid live settings rejection, immediate disable behavior, and disabled-during-interaction winning over resume. Manual listen behavior is preserved when Wake is disabled in the deterministic tests reviewed.

Status for final WWR-900: partially supported pending final production activation wiring and exact-head regression evidence.

### Talking suspension/resume and no barge-in

Merged command-lifecycle source treats ASR, Thinking, and Talking as one guarded command-interaction interval for Wake. Wake resumes only after terminal success/cancellation/recoverable failure when still enabled; disabling during the interaction resolves to Disabled. `docs/WAKE_WORD_V1.md` documents no barge-in for V1 and Talking/TTS suspension.

Status for final WWR-900: partially supported. Final production lifecycle acceptance and Moose-can-not-wake-itself acceptance remain open.

### One-trigger/one-command invariant

WWR-410 evidence and command activation tests support one accepted wake trigger delivering one handoff payload at most once, and repeated positive KWS frames do not produce repeated command activations after acceptance.

Status for final WWR-900: partially supported pending final production startup/listening orchestration and real command-ASR activation evidence.

### Provider separation and no silent cloud fallback

Merged docs and source maintain Local/Gemini ASR separation. Local Moonshine ASR remains the default ASR path; Gemini Live audio is an explicit selectable ASR mode. Current Wake KWS artifact/runtime preparation is local/offline and distinct from command ASR. No reviewed Wake path silently falls back from local KWS to full-time cloud ASR.

Status for final WWR-900: partially supported. Final audit must include the final Wake startup path and real acceptance harness once present.

### Exact artifact/runtime loading and native architecture verification

WWR-100/110/120 evidence and `wake-word-artifacts.json` support immutable model/runtime identities, license separation, archive/library byte counts, SHA-256 checks, and Linux x86_64 / macOS arm64 architecture verification before native inference. Runtime preparation re-verifies cached artifacts and fails closed on unsafe or corrupt inputs.

Status for final WWR-900: source/tooling supported. Real Linux/macOS KWS acceptance remains open and must not be inferred from freezer/preparation tests alone.

### Logs/errors/metrics privacy

WWR-510 evidence supports privacy-safe Wake diagnostics with enabled/state/model/runtime/platform/policy/ring/trigger/timing/error fields and excludes raw PCM serialization. Reviewed docs state raw Wake PCM remains memory-only and that the wake phrase plus prompt may enter command ASR after a trigger. Existing privacy evidence audits logs/errors for raw audio, secrets, and unnecessary paths at the deterministic/source level.

Status for final WWR-900: partially supported pending final integrated acceptance and final log/metrics audit after startup/listening code lands.

### Network dependency and full-time ASR

Reviewed source/evidence supports local/offline idle KWS intent and no full-time command ASR merely for wake detection. The Wake KWS session is separate from command ASR and does not perform transcription. However, production startup/listening integration and real native KWS inference acceptance remain open, so final claims must wait for WWR-600/610/620/630/640.

Status for final WWR-900: not final.

### Documentation overclaim audit

`docs/WAKE_WORD_V1.md` and WWR-700 audit intentionally avoid claiming supported Linux/macOS production Wake Word until real acceptance passes. The docs distinguish implemented source behavior from remaining native/platform qualification. README feature-availability updates remain intentionally deferred until the feature is genuinely usable.

Status for final WWR-900: partially supported; final docs must be re-audited after production integration and native acceptance.

## Current exact-master gate state consulted

The WWR-800 conversation gate coverage merge at `145bf21d9baeb2b5f58fb50ae079e3094d11f4c6` passed ordinary CI, Wake lifecycle stability, Wake source-security audit, and Wake required-gates audit on exact master. This strengthens gate coverage for conversation-session changes, but it does not close final WWR-800, WWR-900, WWR-950, or WWR-960.

## Remaining final-audit blockers

The following WWR-900 items cannot be closed from this pass:

- Production startup/listening wiring is still open under WWR-300/400.
- Full normal command-ASR activation from an accepted Wake trigger is still open under WWR-310/400.
- Deterministic corpus and reportable pass/fail harness are still open under WWR-600.
- Real Linux x86_64 and macOS arm64 KWS inference acceptance are still open under WWR-610/620.
- Performance baseline and KWS-versus-full-ASR evidence are still open under WWR-630.
- Integrated lifecycle stability acceptance is still open under WWR-640.
- Specialized required Wake gates and final exact-head/exact-master policy are still open under WWR-800/950/960.
- Original TODO reconciliation remains open under WWR-910.

## Audit conclusion

This pass records the source/privacy/security state that is already objectively supported on master `145bf21d9baeb2b5f58fb50ae079e3094d11f4c6`. It should be reused as an input to final WWR-900, but it is not final WWR-900 acceptance. Final WWR-900 must be rerun against the exact final feature head after the remaining production wiring, native acceptance, performance evidence, specialized gates, and documentation reconciliation are complete.
