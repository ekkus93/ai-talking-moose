# WWR-910 Original TODO Reconciliation Matrix

**Date:** 2026-09-25
**Source TODO:** `docs/WAKE_WORD_V1_TODO_2026-09-14.md`
**Remediation TODO:** `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md`
**Matrix base:** `master` at `2ff9dc9ba88b4fcaabb31da3fcddb256502a03e0`

This matrix reconciles the original Wake Word V1 TODO against the remediation evidence. It is a linked evidence matrix for WWR-910; it does not create evidence-only recursion and does not mark final exact-head / exact-master closeout items complete before WWR-950/960 evidence exists.

## Reconciliation rules

- A section is **closed** only when objective source, test, CI, or evidence records satisfy the original requirement.
- A section is **superseded by remediation** only when the remediation TODO narrowed, clarified, or replaced the original requirement with explicit evidence.
- WW-960 and WW-970 remain intentionally open until final exact-head qualification and exact-master verification complete.
- README promotion remains deferred until final closeout says the feature is fully user-ready.

## Section matrix

| Original section | Status | Evidence / rationale |
| --- | --- | --- |
| WW-000 — Freeze scope and implementation baseline | Closed | WWR-000 evidence records the remediation baseline, preservation scope, manual-listen preservation, ASR/TTS policy preservation, no-barge-in/no-trimming V1 policy, and exact baseline traceability. Evidence: `docs/evidence/WWR-000_REMEDIATION_BASELINE_2026-09-17.md`. |
| WW-100 — Select and freeze sherpa-onnx KWS artifacts | Closed | WWR-030 and WWR-100 freeze V1 KWS policy, model archive identity, encoder/decoder/joiner/tokenizer/BPE identities, keyword representation, manifest consistency, provenance, and licensing. Evidence: `docs/evidence/WWR-030_CANONICAL_KWS_POLICY_2026-09-17.md`, `docs/evidence/WWR-100_MODEL_IDENTITY_2026-09-17.md`; exact runs `35289861261`, `35289861279`, `35289861291`. |
| WW-110 — Add sherpa native runtime packaging | Closed | WWR-110 and WWR-120 freeze Linux x86_64 and macOS arm64 runtime identities, architecture checks, deterministic runtime layout, offline preparation, cache verification, safe extraction, sanitized unsupported-platform errors, and Apache-2.0 runtime notices. Evidence: `docs/evidence/WWR-110_SHERPA_RUNTIME_IDENTITY_2026-09-17.md`, `docs/evidence/WWR-120_ARTIFACT_CI_COVERAGE_2026-09-17.md`; exact runs `35296408130`, `35296408236`, `35296408238`, `35296408308`. |
| WW-200 — Implement generic `PcmRingBuffer` | Closed | WWR-210 and lifecycle evidence cover bounded in-memory PCM retention, chronological pre-roll snapshots, validation before retention, clear/reset behavior, privacy-safe diagnostics, and bounded long-cycle behavior. Evidence: WWR-210 PRs #187/#189, WWR-300/310 handoff evidence, WWR-640 lifecycle stability. |
| WW-300 — Implement `SherpaKwsEngine` | Closed | WWR-200 implements the real verified native sherpa C-API KWS session with exact model/runtime verification, fixed policy, bounded detection event, reset/shutdown, sanitized errors, no network dependency, and no full transcription. Real positive/negative platform acceptance is closed by WWR-600/610/620. Evidence: source `src-tauri/src/app/wake_word_engine.rs`, `docs/evidence/WWR-610_620_REAL_KWS_ACCEPTANCE_2026-09-24.md`, real-KWS run `35992665781`. |
| WW-310 — Implement `WakeWordRuntimeManager` | Closed | WWR-020/300/400/410/510/640 establish one authoritative runtime manager, phases, ring-buffer coordination, microphone routing, start/stop/suspend/resume, duplicate-start/duplicate-trigger protections, shutdown/cancellation boundaries, manual preservation after recoverable errors, and no hidden full-time ASR/cloud fallback. Evidence: source/security audit `36173594597`, lifecycle stability runs `35999823097` and `36090294124`, WWR-900 audit evidence. |
| WW-400 — Integrate authoritative microphone routing | Closed | WWR-300/310/400/640 establish one shared `AppState::audio_capture` ownership model, no competing Wake capture owner, canonical 16-kHz mono PCM routing, shared ring/KWS timeline, deterministic handoff/return, error/cancellation behavior, and no stream multiplication under repeated cycles. Evidence: `docs/evidence/WWR-300_APP_STATE_CAPTURE_COMPOSITION_2026-09-22.md`, `docs/evidence/WWR-300_CAPTURE_RECOVERY_CYCLES_2026-09-22.md`, source-security audit `36173594597`, lifecycle stability `36090294124`. |
| WW-410 — Implement ring-buffer wake→ASR pre-roll handoff | Closed | WWR-310 evidence covers chronological snapshot, complete wake phrase retention, no V1 acoustic trimming, ordered pre-roll replay, live continuation without duplicate/inversion, first-command-word preservation in deterministic provider-neutral command-ASR handoff, failure recovery, and stale pre-roll clearing. Evidence: `docs/evidence/WWR-310_HANDOFF_BOUNDARY_2026-09-22.md`, command activation source/tests, source-security audit `36173594597`. |
| WW-500 — Add persisted Wake Word settings | Closed | WWR-010 and WWR-500 close persisted wake enablement, disabled defaults, fixed phrase, settings validation/normalization, generated contract/current settings integration, live update rollback behavior, restart/load behavior, and unrelated ASR/TTS preservation. Evidence: PR #169 exact CI `35254726268`, WWR-500 UI/settings evidence embedded in the remediation TODO, current-behavior docs. |
| WW-510 — Add Wake Word Settings UI | Closed | WWR-500 closes Settings UI section, enable toggle, fixed phrase display, local/offline and active microphone disclosure, no custom phrase/sensitivity controls, runtime status/error display, live runtime update, restore manual behavior, cloud-transcription wording guard, and accessibility/keyboard coverage. Evidence: WWR-500 section in remediation TODO and documentation audit `36173015873`. |
| WW-600 — Integrate wake state with conversation lifecycle | Closed | WWR-400/410/640 establish enabled/listening lifecycle, single command interaction per trigger, repeated positive-frame debounce, command/Thinking/Talking suspension boundaries, TTS success/cancellation/recoverable-failure resume, ring clear before/after Talking, disabled-during-interaction behavior, and no barge-in. Evidence: lifecycle stability `35999823097`, `36090294124`, source-security audit `36173594597`, current-behavior lifecycle section. |
| WW-610 — Debounce and trigger policy | Closed | WWR-410 closes one wake event → one command activation, repeated-frame ignore after trigger, reset after return to listening, later phrase after resume, no cooldown needed, privacy-safe trigger count/timing, and deterministic tests. Evidence: `docs/evidence/WWR-410_DEBOUNCE_TRIGGER_SEMANTICS_2026-09-22.md`, source-security audit `36173594597`. |
| WW-700 — Add privacy-safe diagnostics | Closed | WWR-510 closes enabled/state/model/runtime/platform/thread/sample/ring/threshold/trigger/init/Talking/error diagnostics, optional measured performance fields, no raw PCM representation, and credential/path/audio-content audit coverage. Evidence: WWR-510 evidence files, privacy audit `36173015854`, WWR-900 audit evidence. |
| WW-710 — Add performance instrumentation | Closed | WWR-630 accepts Linux/macOS idle KWS CPU, memory overhead, inference timing/real-time behavior, wake→ASR latency, pre-roll startup timing, repeated-cycle resource behavior, one-thread policy, and continuous-ASR comparison. Evidence: `docs/wake-word-performance-evidence.json`, performance gate `36173015866`. |
| WW-800 — Build deterministic wake-word acceptance corpus | Closed | WWR-600 closes generated corpus manifest, reproducible provenance/license policy, positive variants, command-following positives, background/noise variants, ordinary/near-miss/media-like negatives, threshold/score reporting, recall/false-accept criteria, and private/copyright fixture guardrails. Evidence: `docs/evidence/WWR-600_DETERMINISTIC_CORPUS_ACCEPTANCE_2026-09-24.md`, real-KWS run `35992665781`. |
| WW-810 — Add real sherpa KWS platform acceptance | Closed | WWR-610/620 close Linux x86_64 and macOS arm64 real pinned-model KWS acceptance with exact artifact hashes, runtime architecture verification, CPU-only one-thread inference, positive/negative fixtures, offline inference after preparation, and privacy-safe evidence. Evidence: `docs/evidence/WWR-610_620_REAL_KWS_ACCEPTANCE_2026-09-24.md`, run `35992665781`. |
| WW-820 — Add long/repeated lifecycle stability acceptance | Closed | WWR-640 closes repeated wake→ASR→Thinking→Talking→wake cycles, native/session/capture/ring boundedness, TTS terminal outcomes, disable/enable cycles, shutdown scenarios, bounded soak/false-trigger coverage where practical, and interval/resource records. Evidence: PR #435, exact master `3d892865241579eccfcfc6a5084a522b43eefa08`, runs `35999823102` and `35999823097`; additional 100-cycle resource evidence in run `36090294124`. |
| WW-900 — Documentation and user-facing behavior | Closed | WWR-700 closes architecture/default phrase/disabled-by-default/local-offline/privacy/downstream ASR/Talking/no-barge-in/ring/provenance/diagnostics docs, avoids subjective accuracy claims, and defers README promotion until final user-ready closeout. Evidence: `docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md`, `docs/WAKE_WORD_V1_CI_GATES.md`, documentation audit `36173015873`. |
| WW-950 — Final source/privacy/security audit | Closed | WWR-900 final audit evidence reviews runtime ownership, microphone transitions, cancellation/shutdown, ring clearing, disabled behavior, Talking resume/suspension, provider separation, artifact/runtime loading, native architecture verification, logs/errors/metrics privacy, offline KWS, no full-time ASR, documentation truthfulness, and mandatory findings. Evidence: `docs/evidence/WWR-900_FINAL_SOURCE_PRIVACY_SECURITY_AUDIT_2026-09-25.md`, exact-master source-security audit `36173594597`, ordinary CI `36173594560`. |
| WW-960 — Exact-head qualification | Pending final closeout | Do not close here. WWR-950 in the remediation TODO must record the final PR head, ordinary CI, specialized Wake gates, artifact/corpus revisions, acceptance platform details, and final diff review. |
| WW-970 — Guarded merge and exact-master verification | Pending final closeout | Do not close here. WWR-960 in the remediation TODO must record the exact guarded merge, merged master SHA, exact-master ordinary CI, required exact-master Wake gates, manifest/corpus identity continuity, and final closeout evidence. |

## Final checklist reconciliation

| Original final checklist item | Status | Evidence / rationale |
| --- | --- | --- |
| sherpa-onnx KWS is the wake engine | Closed | WWR-030/100/200; real-KWS acceptance. |
| Production model/runtime artifacts are pinned and provenance documented | Closed | WWR-100/110/120 evidence and artifact manifest. |
| Default wake phrase is `Hey, Moose` | Closed | WWR-010/030/500/700. |
| Wake feature can be enabled/disabled in Settings | Closed | WWR-500. |
| Wake feature defaults disabled | Closed | WWR-010/500. |
| Wake-disabled behavior preserves current manual interaction behavior | Closed | WWR-000/010/400/500/900. |
| Generic 2-second PCM ring buffer is implemented and tested | Closed | WWR-030/210/310/640. |
| Wake→ASR handoff preserves wake phrase + first command words | Closed | WWR-310 evidence. |
| No V1 acoustic trimming requirement exists | Closed | WWR-000/310/700. |
| Exactly one command activation occurs per wake event | Closed | WWR-410 and source-security audit. |
| Wake listening is suspended while Moose is Talking | Closed | WWR-400/640. |
| No barge-in is implemented in V1 | Closed | WWR-000/400/700. |
| Wake listening resumes after TTS completion/cancellation/failure recovery | Closed | WWR-400/640. |
| Raw wake PCM remains memory-only and absent from logs | Closed | WWR-510/900 privacy audit. |
| No silent cloud/provider fallback is introduced | Closed | WWR-000/900 source audit. |
| KWS remains local/offline during idle listening | Closed | WWR-200/610/620/900. |
| Diagnostics are privacy-safe | Closed | WWR-510/900. |
| Deterministic positive/negative/near-miss corpus acceptance passes | Closed | WWR-600. |
| Linux x86_64 real sherpa KWS acceptance passes | Closed | WWR-610. |
| macOS arm64 real sherpa KWS acceptance passes | Closed | WWR-620. |
| Repeated lifecycle stability passes | Closed | WWR-640. |
| Performance baseline is recorded | Closed | WWR-630. |
| Documentation is complete and truthful | Closed | WWR-700. |
| Exact PR-head required gates pass | Pending final closeout | WWR-950 only. |
| Guarded merge uses exact tested head | Pending final closeout | WWR-960 only. |
| Exact merged-master required gates pass | Pending final closeout | WWR-960 only. |

## Remaining work after this matrix

1. Update the remediation TODO's WWR-910 section to link this matrix and keep WW-960/WW-970 pending for final qualification/merge evidence.
2. Perform WWR-950 exact-head final qualification from a fresh branch/head.
3. Perform WWR-960 guarded merge and exact-master verification.
