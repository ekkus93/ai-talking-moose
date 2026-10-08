# Whisper.cpp CPU Acceptance and Benchmark Record

**Status:** Offline batch transcription passed. Nominal streaming acceptance exposed overload and is not qualified for realtime use pending cadence/runtime tuning.
**Canonical native source:** `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`.
**Model artifact revision:** `5359861c739e955e79d9a303bcbc70fb988958b1`.

The real-CPU acceptance completed on `master` at `1f78a43adaf4b3907aa82fc96215e4d353bccb8c`. The immutable workflow identity and raw measurements are recorded in [`WHISPER_CPP_LOCAL_ASR_QUALIFICATION_2026-10-08.md`](evidence/WHISPER_CPP_LOCAL_ASR_QUALIFICATION_2026-10-08.md).

## Measurement method

The workflow runs a clean-profile install/delete/reinstall, then transcribes the pinned corpus with network access denied. It hashes the installed model and corpus and records repository/native-source identity. Linux high-water RSS uses `/proc/self/status` `VmHWM`; CPU utilization is process CPU time divided by measured phase wall time. The production pipeline is also fed 100 ms chunks at 100 ms intervals, followed by a deliberate bounded-queue overload attempt.

## Results

| Measurement | Result |
|---|---|
| Qualified repository SHA | `1f78a43adaf4b3907aa82fc96215e4d353bccb8c` |
| Workflow run / job / attempt | `37738499386` / `113183457764` / 1 (workflow succeeded) |
| Artifact | `whisper-real-cpu-37738499386-1-1f78a43adaf4b3907aa82fc96215e4d353bccb8c`; ID `11532886674`; SHA-256 `0a8f72cfe7c86f274d9ba6c21585af2c14fd33cc9b0af729076a8fd24ad5314a` |
| CPU / platform | AMD EPYC 7763 64-Core Processor; Linux x86_64; 4 available parallel workers |
| Native source / model revision | `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774` / `5359861c739e955e79d9a303bcbc70fb988958b1` |
| Model SHA-256 / bytes | `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b` / `487601967` |
| Corpus SHA-256 / bytes | `59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e` / `352078` |
| Batch transcript | Expected JFK sentence; 11,000 ms audio; 1 segment; no-speech probability 0.000019276282 |
| Batch transcription wall / CPU / utilization | 3,360 ms transcription (3,470 ms phase); 13,391 ms CPU; 385.91% |
| Batch baseline / current / high-water RSS | 25,686,016 / 688,021,504 / 688,021,504 bytes |
| Streaming first partial / first final | 3,522 / 21,676 ms |
| Streaming audio processed / inference wall / reported RTF | 2,300 / 21,172 ms / 9.205407 |
| Streaming CPU time / average utilization / high-water RSS | 84,088 ms / 387.26% / 838,987,776 bytes |
| Nominal dropped chunks | 87 at 100 ms input cadence |
| Deliberate overload | 64 attempted, 8 accepted, 56 dropped |
| Partial / endpoint silence / maximum utterance | 300 ms / 500 ms / 30 s |
| Queue capacity | 8 x 100 ms = 800 ms |

The workflow's green status establishes that install/delete/reinstall, model verification, and offline batch transcription passed. It does not establish realtime streaming performance. Synchronous partial inference re-transcribes the growing utterance every 300 ms; on this four-worker GitHub runner, normal-cadence input dropped 87 chunks and the pipeline processed only 2.3 seconds of the 11-second corpus. The reported RTF is based on processed audio, not the full source duration. Do not increase queue capacity based on this result: that would permit more queued latency without fixing throughput. Tune partial inference cadence/runtime, then repeat exact-source acceptance before closing WPR-720.
