# WWR-510 Measured Diagnostics Placeholders — 2026-09-22

## Scope

This evidence records privacy-safe diagnostic schema support for future Wake Word measurement evidence.

## Change

`WakeWordDiagnostics` now includes optional aggregate measurement fields:

- `measured_idle_cpu_percent`
- `measured_memory_rss_bytes`
- `last_inference_duration_ms`
- `last_handoff_duration_ms`

These fields are initialized to `None` until real, accepted measurement sources are added. They expose aggregate counters/timings only and cannot represent raw audio, transcripts, credentials, or filesystem paths.

The privacy audit now requires these fields and requires current-behavior documentation to state that they remain empty until accepted measurements exist.

## Non-claims

This does not complete WWR-630 performance evidence. It does not record representative Linux/macOS CPU, memory, inference, handoff, repeated-cycle, or continuous-ASR comparison measurements. It only makes the diagnostics schema ready for those values when real acceptance evidence exists.
