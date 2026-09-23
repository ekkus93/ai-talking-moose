# WWR-700 — Wake Word V1 documentation audit

Audited against exact master `b59e768cf843f750aebb5c268f2d4220c5b30832` after PR #398.

## Result

`docs/WAKE_WORD_V1.md` now provides the authoritative Wake Word V1 developer/user documentation and is deliberately qualification-aware. It documents:

- the consolidated single Wake Word subsystem and one `WakeWordRuntimeManager`;
- fixed phrase `Hey, Moose` and disabled-by-default policy;
- local/offline idle keyword spotting and the fact that the selected microphone remains locally active while Listening;
- the command-ASR privacy boundary: wake phrase plus following prompt may be processed by the user's selected normal command ASR after handoff;
- the two-second memory-only pre-roll/ring behavior and privacy-safe diagnostics;
- Talking/TTS suspension, no-barge-in V1 policy, terminal resume policy, and fail-closed Wake errors;
- immutable model/runtime provenance and separate licensing evidence;
- diagnostics and fail-closed troubleshooting guidance;
- explicit qualification limits for Linux x86_64 and macOS arm64.

The document explicitly states that production lifecycle startup wiring, deterministic corpus qualification, real Linux/macOS native KWS acceptance, performance evidence, and final end-to-end qualification remain open. It therefore does not describe those planned/partially integrated behaviors as production-qualified.

## WWR-700 reconciliation boundary

The documentation content requirements that can be objectively established before final acceptance are satisfied by `docs/WAKE_WORD_V1.md`. The following WWR-700 items intentionally remain open until later gates provide the missing facts:

- supported-platform claims based on real WWR-610/620 acceptance;
- README/user-facing feature-availability claims once production startup/lifecycle wiring is usable;
- final acceptance that user-facing docs match the completed production behavior on the final feature head.

No subjective accuracy claim is made. Platform artifacts are described as eligibility inputs, not support evidence.

## Evidence

- Documentation implementation: PR #398, merged as `b59e768cf843f750aebb5c268f2d4220c5b30832`.
- Exact-master ordinary CI for PR #398 predecessor/merge validation was recorded in the Ralph loop before this audit; final WWR-700 acceptance remains gated by the final feature head and WWR-610/620.
- Canonical documentation: `docs/WAKE_WORD_V1.md`.
- Model identity/provenance: `docs/evidence/WWR-100_MODEL_IDENTITY_2026-09-17.md`.
- Runtime identity/provenance: `docs/evidence/WWR-110_SHERPA_RUNTIME_IDENTITY_2026-09-17.md`.
- Architecture consolidation: `docs/evidence/WWR-020_WAKE_WORD_ARCHITECTURE_CONSOLIDATION_2026-09-17.md`.
- Capture ownership/routing: WWR-300 evidence files linked from the remediation TODO.
- Handoff: `docs/evidence/WWR-310_HANDOFF_BOUNDARY_2026-09-22.md`.
- Diagnostics/privacy: WWR-510 evidence linked from the remediation TODO.
