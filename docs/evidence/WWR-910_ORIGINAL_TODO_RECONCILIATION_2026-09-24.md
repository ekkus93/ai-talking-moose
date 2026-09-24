# WWR-910 — original Wake Word V1 TODO reconciliation matrix

Date: 2026-09-24
Matrix status: partial reconciliation, not final closeout
Current master reviewed: `f7459270fde6be048a94911b766f721442a04779`
Original TODO: `docs/WAKE_WORD_V1_TODO_2026-09-14.md`
Remediation TODO: `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`

## Scope

This matrix reconciles the original WW-### TODO sections against the remediation evidence that now exists on `master`. It intentionally does not mark final closeout complete while WWR-630 measured performance, WW-960/WW-970 final exact-head/exact-master closeout, and remaining final TODO reconciliation work are still open.

The original TODO is retained as historical source material. This evidence matrix is the linked reconciliation record required by WWR-910 so stale unchecked original boxes are not misread as the current state.

## Reconciliation summary

| Original section | Reconciled status | Evidence / boundary |
| --- | --- | --- |
| WW-000 — Freeze scope and implementation baseline | Complete | WWR-000 baseline evidence, fixed phrase, disabled default, manual behavior preservation, ASR/TTS separation, no barge-in, and no acoustic trimming are reflected in the remediation TODO and current docs. |
| WW-100 — Select and freeze sherpa-onnx KWS artifacts | Complete | WWR-100 model identity evidence and WWR-110 runtime identity evidence record immutable model/runtime/tokenizer/runtime identities, provenance, licenses, supported architectures, and fail-closed verification. |
| WW-110 — Add sherpa native runtime packaging | Complete for claimed Linux x86_64 and macOS arm64 paths | WWR-110, WWR-120, WWR-610, and WWR-620 evidence cover deterministic runtime layout, architecture verification, cache/hash verification, unsupported-platform errors, notices, and real accepted Linux/macOS inference. Final packaged application load evidence remains a final-closeout consideration rather than a missing WW-110 identity task. |
| WW-200 — Implement generic `PcmRingBuffer` | Complete | WWR-210 and WWR-640 evidence cover bounded in-memory retention, chronological pre-roll, invalid PCM rejection before retention, clearing, repeated-cycle boundedness, and privacy-safe diagnostics/logging. |
| WW-300 — Implement `SherpaKwsEngine` | Complete | WWR-200, WWR-600, WWR-610, and WWR-620 cover exact verified artifacts, one-thread policy, score/threshold, streaming 16 kHz mono PCM, bounded detection, sanitized errors, reset/shutdown behavior, no transcription, no network inference dependency, and real positive/negative KWS acceptance. |
| WW-310 — Implement `WakeWordRuntimeManager` | Complete for current remediation scope | WWR-020, WWR-300, WWR-400, WWR-410, and WWR-640 cover the single authoritative runtime manager, lifecycle states, duplicate-start prevention, duplicate-command prevention, shutdown, recovery, manual-path preservation, and no hidden ASR/cloud fallback. |
| WW-400 — Integrate authoritative microphone routing | Complete for deterministic/source acceptance; final production closeout still depends on WWR-950/960 | WWR-300/310/640 evidence records the shared `AppState::audio_capture` ownership strategy, one chronological canonical stream, no competing capture opens, deterministic handoff/return, recovery behavior, and repeated lifecycle stability. |
| WW-410 — Implement ring-buffer wake→ASR pre-roll handoff | Complete for deterministic handoff evidence | WWR-310 evidence records chronological snapshot, wake phrase plus first command word preservation, no acoustic trimming, ordered pre-roll/live handoff, no duplicate/inverted ranges, stale handoff clearing, and recoverable failure handling. |
| WW-500 — Add persisted Wake Word settings | Complete | WWR-010/030/500 cover disabled default, fixed phrase, settings validation/normalization, live update rejection without partial persistence, generated contract, and unrelated ASR/TTS preservation. |
| WW-510 — Add Wake Word Settings UI | Complete | WWR-500/510 cover Settings UI toggle, fixed phrase display, local/offline and active-microphone disclosures, runtime state/error display, live enable/disable behavior, and no full-time cloud transcription implication. |
| WW-600 — Integrate wake state with conversation lifecycle | Complete for deterministic lifecycle acceptance | WWR-400/410/640 cover one wake event to one command activation, repeated positive-frame debounce, Talking suspension, TTS success/cancel/recoverable-failure resume, disable-during-interaction behavior, no V1 barge-in, and no stuck suspended state under the defined acceptance. |
| WW-610 — Debounce and trigger policy | Complete | WWR-410 evidence covers one wake event to one command activation, repeated positive-frame suppression, later phrase after resume, no cooldown requirement, trigger count privacy, and monotonic last-trigger diagnostics. |
| WW-700 — Add privacy-safe diagnostics | Complete | WWR-510 and WWR-900 evidence cover enabled/runtime/model/platform/thread/sample/ring/threshold/trigger/timing/error diagnostics and privacy audits proving no raw PCM, credentials, secrets, unnecessary paths, or audio content exposure. |
| WW-710 — Add performance instrumentation | Partial / pending | WWR-630 partial evidence now records idle KWS CPU, memory, inference timing, real-time behavior, and one-thread policy from exact Linux/macOS real-KWS acceptance. Wake→ASR activation latency, pre-roll startup timing, repeated-cycle resource delta, continuous-ASR comparison, and final accepted performance baseline remain pending. |
| WW-800 — Build deterministic wake-word acceptance corpus | Complete | WWR-600 evidence covers positive variants, command-following positives, background/noise variants, ordinary speech negatives, near-misses including `Hey Moosey`, media/background-style negatives, provenance/license, score/threshold, recall/false-accept criteria, CI/report friendliness, and tested-condition boundaries. |
| WW-810 — Add real sherpa KWS platform acceptance | Complete | WWR-610/620 evidence covers Linux x86_64 and macOS arm64 real pinned-model KWS inference, CPU-only path, hash verification, native architecture checks, one-thread policy, positive/negative fixtures, offline inference, and privacy-safe reports. |
| WW-820 — Add long/repeated lifecycle stability acceptance | Complete | WWR-640 evidence covers repeated wake→ASR→Thinking→Talking→wake cycles, bounded native/session/resource state, no stream multiplication, bounded ring memory, repeated TTS terminal outcomes, disable/enable cycles, shutdown while listening, shutdown during handoff, and bounded soak/false-trigger acceptance. |
| WW-900 — Documentation and user-facing behavior | Complete | WWR-700 evidence updates behavior, architecture, CI/gate, Settings disclosure, and audit enforcement docs. Exact-master documentation/privacy/required-gates audits passed on `e7a8dc91f317a62184a8ae7fead90afc1be6343c`. |
| WW-950 — Final source/privacy/security audit | Complete for WWR-900 scope | WWR-900 evidence records final source/privacy/security audit acceptance. It does not replace WW-960/970 final qualification/merge evidence. |
| WW-960 — Exact-head qualification | Pending | Final qualification cannot close until WWR-630 accepted performance evidence and any final TODO/doc reconciliation diff are complete and exact-head required gates pass. |
| WW-970 — Guarded merge and exact-master verification | Pending | Final merge/exact-master verification remains pending until the final qualification head is ready and merged. |

## Final checklist mapping

Complete or accepted with current evidence:

- sherpa-onnx KWS is the wake engine;
- production model/runtime artifacts are pinned and provenance documented;
- default wake phrase is `Hey, Moose`;
- Wake feature can be enabled/disabled in Settings;
- Wake feature defaults disabled;
- Wake-disabled behavior preserves manual interaction behavior;
- generic two-second PCM ring/pre-roll behavior is implemented and tested;
- wake→ASR handoff preserves wake phrase plus first command words in deterministic acceptance;
- no V1 acoustic trimming requirement exists;
- exactly one command activation occurs per wake event;
- Wake listening is suspended while Moose is Talking;
- no barge-in is implemented in V1;
- Wake listening resumes after TTS completion/cancellation/recoverable failure in deterministic lifecycle acceptance;
- raw Wake PCM remains memory-only and absent from logs/diagnostics;
- no silent cloud/provider fallback is introduced;
- KWS remains local/offline during idle inference after verified artifacts are prepared;
- diagnostics are privacy-safe;
- deterministic positive/negative/near-miss corpus acceptance passes;
- Linux x86_64 real sherpa KWS acceptance passes;
- macOS arm64 real sherpa KWS acceptance passes;
- repeated lifecycle stability passes;
- documentation is complete and truthful for current evidence boundaries.

Still pending:

- accepted WWR-630 performance baseline;
- exact final PR-head required gates;
- guarded final merge using exact tested head;
- exact merged-master final required gates;
- final non-recursive WWR-950/960 closeout record.

## Boundary

This matrix intentionally avoids evidence-only recursion. It reconciles the original TODO to current evidence and identifies the exact remaining items; it does not claim final Wake Word V1 closeout until the remaining performance and final exact-head/exact-master gates pass.
