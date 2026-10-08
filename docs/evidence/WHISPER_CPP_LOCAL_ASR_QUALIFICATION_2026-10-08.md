# Whisper.cpp Local ASR Qualification Evidence

**Historical qualification status:** The final recorded run passed installation/reinstall, offline transcription, and zero nominal-load drops on its measured Linux x86_64 runner. This evidence applies only to checkout `0628de3d0cd946d8d1c0c3fe89afce8f3253854c`; subsequent production changes require fresh exact-source qualification.
**Current follow-up:** `docs/WHISPER_CPP_LOCAL_ASR_POST_QUALIFICATION_REVIEW_TODO_2026-10-08.md`.

## Immutable workflow identity

| Field | Value |
|---|---|
| Repository | `ekkus93/ai-talking-moose` |
| Branch | `master` |
| Repository source SHA | `1f78a43adaf4b3907aa82fc96215e4d353bccb8c` |
| Workflow | Whisper Real CPU Acceptance |
| Run | [37738499386](https://github.com/ekkus93/ai-talking-moose/actions/runs/37738499386) |
| Job | `113183457764` — Real whisper-small CPU acceptance |
| Attempt | `1` |
| Result / duration | Success / 6m 46s job (6m 51s run) |
| Artifact name | `whisper-real-cpu-37738499386-1-1f78a43adaf4b3907aa82fc96215e4d353bccb8c` |
| Artifact ID | `11532886674` |
| Artifact size | 6.88 KB |
| Artifact SHA-256 | `0a8f72cfe7c86f274d9ba6c21585af2c14fd33cc9b0af729076a8fd24ad5314a` |

The successful workflow validator verified the clean-profile production install, delete, reinstall, model identity/size/hash, native-source identity, network-denied transcription, and required CPU/pipeline metric fields. The artifact is subject to the repository's three-day Actions artifact retention limit. Its contents were downloaded and inspected for this record.

The run qualified native source `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774` and model revision `5359861c739e955e79d9a303bcbc70fb988958b1`, SHA-256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`, size `487601967` bytes.

## Initial baseline measurements (300 ms partial interval)

| Measurement | Value |
|---|---|
| Host CPU / OS / architecture | AMD EPYC 7763 64-Core Processor / Linux / x86_64; 4 available parallel workers |
| Actual whisper.cpp source revision | `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774` |
| Model revision / SHA-256 / bytes | `5359861c739e955e79d9a303bcbc70fb988958b1` / `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b` / `487601967` |
| Corpus path / bytes / SHA-256 | `third_party/whisper.cpp/samples/jfk.wav` / `352078` / `59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e` |
| Batch transcript / duration / segments / no-speech probability | “And so my fellow Americans, ask not what your country can do for you, ask what you can do for your country.” / 11,000 ms / 1 / 0.000019276282 |
| Batch transcription wall time | 3,360 ms transcription; 3,470 ms measured phase |
| Batch CPU time / utilization | 13,391 ms / 385.91%; process CPU time divided by measured phase wall time |
| Batch baseline / resident / high-water RSS | 25,686,016 / 688,021,504 / 688,021,504 bytes; high-water from Linux `/proc/self/status` `VmHWM` |
| Streaming first partial / first final latency | 3,522 ms / 21,676 ms |
| Streaming processed audio / inference wall / reported RTF | 2,300 ms / 21,172 ms / 9.205407 |
| Streaming CPU time / average utilization / high-water RSS | 84,088 ms / 387.26% / 838,987,776 bytes |
| Nominal input drops | 87 chunks while offering 100 ms chunks at 100 ms intervals |
| Deliberate overload | 64 attempted; 8 accepted; 56 dropped |
| Partial interval / endpoint silence / maximum utterance | 4,800 / 8,000 / 480,000 samples (300 ms / 500 ms / 30 s) |
| Queue capacity | 8 chunks, approximately 800 ms of audio |

The batch path produced the expected transcript with networking denied. The streaming result is not a realtime pass: inference is synchronous on the worker, and each partial re-transcribes the accumulated utterance. Under this runner and the configured 300 ms cadence, the producer outpaced inference, most input was dropped, and the reported RTF describes the 2.3 seconds actually processed rather than the full 11-second source. This evidence requires cadence/runtime tuning and a new exact-source acceptance run before claiming nominal streaming performance. The bounded queue is overload protection; increasing it would only allow more latency to accumulate and is not justified by these data.

## Follow-up tuning attempt

The first tuning attempt ran on `master` at source SHA `b1ba066414c703c093d28ab920c83162d1e5b737` with a five-second partial cadence and a 40-chunk Whisper queue. Installation and offline batch transcription passed, but the workflow correctly failed its new zero-nominal-drop assertion.

| Field | Value |
|---|---|
| Run / job / attempt | [37742220074](https://github.com/ekkus93/ai-talking-moose/actions/runs/37742220074) / `113195330780` / 1 (failed zero-drop validation) |
| Artifact ID / name | `11534766514` / `whisper-real-cpu-37742220074-1-b1ba066414c703c093d28ab920c83162d1e5b737` |
| Artifact ZIP SHA-256 | `429eb7a1887300b60ba5611eb5b3e591d997cf43959778731e9275fd1ceaf491` |
| CPU / platform | Intel Xeon Platinum 8573C / Linux x86_64 / 4 available workers |
| Nominal chunks dropped / processed audio | 3 / 10,700 ms of 11,000 ms |
| First partial / first final latency | 9,653 / 20,374 ms |
| Inference wall time / reported RTF | 14,132 ms / 1.3207968 |
| Pipeline CPU time / average utilization / high-water RSS | 56,022 ms / 274.48% / 708,050,944 bytes |
| Deliberate overload | 64 attempted; 40 accepted; 24 dropped |

The five-second cadence substantially improved audio retention over the original 300 ms cadence (3 drops instead of 87), but the 40-chunk queue remained slightly too small. The candidate then raised only Whisper's queue to 56 chunks (5.6 seconds); that intermediate result was unqualified until the final acceptance run recorded below.


## Final candidate acceptance (five-second partial interval)

The final candidate ran on current `master` at exact repository SHA `0628de3d0cd946d8d1c0c3fe89afce8f3253854c`. The workflow validated native/model/corpus provenance, offline transcription, two partial events followed by one final event, zero drops at nominal input cadence, and bounded-queue overload behavior.

| Field | Value |
|---|---|
| Run / job / attempt | [37744371559](https://github.com/ekkus93/ai-talking-moose/actions/runs/37744371559) / `113202252940` / 1 (success) |
| Artifact name / ID / size | `whisper-real-cpu-37744371559-1-0628de3d0cd946d8d1c0c3fe89afce8f3253854c` / `11535975002` / 7,059 bytes |
| Artifact ZIP SHA-256 | `63c24a708bd6b2c969cf96bdd80b6bd00b3855a6a95248d713f26a15fba1b32e` |
| CPU / platform | AMD EPYC 7763 64-Core Processor / Linux x86_64 / 4 available workers |
| Native source / model revision | `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774` / `5359861c739e955e79d9a303bcbc70fb988958b1` |
| Model SHA-256 / bytes | `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b` / `487601967` |
| Corpus SHA-256 / bytes | `59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e` / `352078` |
| Batch transcription wall / phase wall | 3,354 / 3,473 ms |
| Batch CPU time / utilization | 13,408 ms / 386.06% (process CPU / phase wall) |
| Batch baseline / current / high-water RSS | 25,612,288 / 688,041,984 / 688,041,984 bytes |
| Transcript | Expected JFK sentence; 11,000 ms; 1 segment; no-speech probability 0.000019276282 |
| Partial interval / endpoint / max utterance | 80,000 / 8,000 / 480,000 samples (5 s / 500 ms / 30 s) |
| Whisper ingress capacity | 56 chunks (5.6 seconds) |
| Partial / final events | 2 / 1 |
| First partial / first final latency | 8,219 / 16,986 ms |
| Processed audio / inference wall / RTF | 11,000 / 9,623 ms / 0.87482446 |
| Pipeline CPU time / average utilization / high-water RSS | 38,166 ms / 224.20% / 707,805,184 bytes |
| Nominal drops | 0 chunks at 100 ms input cadence |
| Deliberate overload | 64 attempted; 56 accepted; 8 dropped |

The final candidate preserved the complete corpus and met the workflow's zero-drop nominal-load criterion. First partial latency is about 8.2 seconds and final latency about 17 seconds on this runner; the result qualifies the measured hardware/corpus/configuration and does not establish the same latency on every supported CPU.
