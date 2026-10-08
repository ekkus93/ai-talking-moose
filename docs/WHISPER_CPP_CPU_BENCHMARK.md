# Whisper.cpp CPU Acceptance and Benchmark Record

**Status:** Pending exact-head real-CPU acceptance.
**Canonical native source:** `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`.
**Model artifact revision:** `5359861c739e955e79d9a303bcbc70fb988958b1`.

No final-source CPU or RSS measurements are recorded yet. The progress evidence under `docs/evidence/` belongs to its stated historical repository SHA and does not qualify current `master`.

## Required measurement method

Run `.github/workflows/whisper-real-cpu-acceptance.yml` on the exact production SHA. The workflow records one clean-profile install, then runs the pinned corpus transcription with Linux network access denied. The evidence JSON independently hashes the installed model and corpus, records the repository and native-source identities, and includes workflow run/job/attempt metadata. Linux peak RSS comes from `/proc/self/status` `VmHWM`; CPU utilization is computed from process CPU time over measured phase wall time.

The workflow also sends the production pipeline 100 ms chunks at its normal cadence and performs a deliberate bounded-queue overload attempt. Its machine-readable pipeline report includes transcript latency, real-time factor, CPU and RSS metrics, dropped chunks, and behavior constants.

## Results

| Measurement | Result |
|---|---|
| Qualified repository SHA | Pending |
| Workflow run, job, attempt | Pending |
| Evidence artifact name and immutable artifact identity | Pending |
| CPU model and runner identity | Pending |
| Actual whisper.cpp gitlink and canonical revision | Pending |
| Actual model SHA-256 and bytes | Pending |
| Test-audio identity and SHA-256 | Pending |
| Transcription output | Pending |
| First partial latency | Pending |
| Final latency from endpoint/finalization | Pending |
| Real-time factor | Pending |
| CPU utilization | Pending |
| Current and high-water RSS | Pending |
| Dropped chunks at nominal load | Pending |
| Dropped chunks under deliberate overload | Pending |
| Partial interval / endpoint silence / maximum utterance | 300 ms / 500 ms / 30 seconds |
| Queue capacity and budget | 8 x 100 ms = 800 ms |

The queue budget is bounded ingress protection, not a throughput claim. Keep it unchanged unless exact-source measurements show that nominal CPU load drops accepted chunks or that another capacity is needed.

## Qualification blocker

The local GitHub CLI credential was invalid when this remediation resumed, so the manual workflow could not be dispatched. Do not treat this pending record, ordinary CI, or historical evidence as real-CPU acceptance. After access is restored, run acceptance against the stabilized production SHA and replace each pending result with values and immutable Actions run/job/artifact links.
