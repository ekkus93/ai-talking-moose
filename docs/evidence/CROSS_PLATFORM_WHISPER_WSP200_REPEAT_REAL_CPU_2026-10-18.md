# WSP-200 — repeated real-model macOS Whisper CPU acceptance

**Evidence captured:** 2026-10-19 UTC from GitHub Actions run completed 2026-10-18 UTC.
**Exact tested source:** `1ab2f8644432c8848f12477c487f27aac766bb9a`.
**Manual workflow:** [Whisper macOS Real CPU Acceptance run 39174236096](https://github.com/ekkus93/ai-talking-moose/actions/runs/39174236096), `workflow_dispatch`, attempt 1, **completed success**.
**Pinned whisper.cpp source:** `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`.
**Pinned model:** `ggml-small.bin`, 487,601,967 bytes, SHA-256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`.

## Job identity and machine-readable evidence

| Runner | Job | JSON-only artifact ID | Result |
| --- | --- | --- | --- |
| macOS ARM64, Apple M1 (Virtual), 3 available workers | [117813491605](https://github.com/ekkus93/ai-talking-moose/actions/runs/39174236096/job/117813491605) | `12154086280` | success |
| macOS Intel x86_64, i7-8700B @ 3.20 GHz, 4 available workers | [117813491538](https://github.com/ekkus93/ai-talking-moose/actions/runs/39174236096/job/117813491538) | `12153628215` | success |

Both jobs confirmed the pinned source and architecture, built native-linked `whisper_acceptance`, ran deterministic pipeline regressions, explicitly installed and SHA-verified the weight in test/user data, deleted and clean-reinstalled it, rejected missing/corrupt model paths, transcribed genuine JFK WAV in two separate processes without redownloading the verified model between these processes, verified CPU-only native output and meaningful JFK text, checked unique final delivery and queue behavior, then deleted the model. Only JSON reports, not weights, were uploaded. This was the **new repeated-cold-start test**, not a live microphone or sustained same-session test.

## Measured results (each process includes its own pipeline inference)

| Metric | ARM64 first | ARM64 repeated | Intel first | Intel repeated |
| --- | ---: | ---: | ---: | ---: |
| Direct model load wall (ms) | 16,839 | 261 | 22,519 | 602 |
| Direct inference wall (ms) | 2,184 | 2,173 | 22,926 | 4,228 |
| First partial event (ms) | 7,350 | 7,704 | 10,730 | 9,857 |
| First final event (ms) | 14,776 | 14,531 | 19,766 | 21,618 |
| Pipeline real-time factor | 0.5597565 | 0.58407843 | 1.3215393 | 1.4265287 |
| Pipeline peak RSS (bytes) | 897,171,456 | 886,652,928 | 842,985,472 | 838,864,896 |
| Nominal dropped queue chunks | 0 | 0 | 0 | 0 |
| Synthetic overload dropped chunks | 8 | 8 | 8 | 8 |
| Partial events / final events | 2 / 1 | 2 / 1 | 2 / 1 | 2 / 1 |

The large first-model-load versus repeated-load difference is **observed** but its cause is not yet established by this report. Direct inference is a separately timed API call, whereas pipeline RTF is the app-owned ingest/inference path; do not equate or directly compare these different clocks. Intel remains slower than real time in both repeated pipeline runs (RTF > 1). This is a performance investigation / release-decision item, not an accepted regression. Earlier baseline: historical Linux x86_64 run `37756211710` at `cf3280ea1d4e91d298828e4935e3369f8cbd6be6`, AMD EPYC 7763, first partial 8,279 ms, final 17,099 ms and peak RSS 707,932,160 bytes. Mac host/vCPU and chronology are not directly equivalent to that Linux machine.

## Acceptance boundaries and follow-up

- Both architecture-specific job conclusions and genuine fixed-corpus, user-data model evidence **pass** at the tested source. This strengthens the WSP-200 two-Mac genuine-transcription prerequisite and establishes repeated-process correctness.
- The JSON contract explicitly reports `network_denied_required=false` and `network_denial_probe_passed=false`: **no enforced macOS egress-denial acceptance** occurred. The workflow itself installs the model over HTTPS before inference; no inference network barrier was demonstrated.
- Running two separate CLI processes is **not** sustained multi-utterance native lifecycle qualification, power/thermal measurement or clean .app microphone/audio/TTS/wake-word acceptance. It does not establish a safe Intel performance threshold.
- The installer WSP-200 overall checkbox remains open because disk-space, active-user-session and remaining edge cases are not all qualified at full Mac application level.
- Keep WSP-200 performance/egress/lifecycle, WSP-300/400 app routing acceptance, WSP-500 deletion, WSP-600 production release/signing and WSP-700 physical tests open. KittenTTS uses distinct ONNX Runtime; neither it nor Moonshine was removed here.
- [Exact-source main CI 39171305711](https://github.com/ekkus93/ai-talking-moose/actions/runs/39171305711) previously passed for the tested head. The real-model job provides separate weight-backed acceptance. Subsequent documentation commits must be checked at their own head rather than treated as identical inference SHA.
