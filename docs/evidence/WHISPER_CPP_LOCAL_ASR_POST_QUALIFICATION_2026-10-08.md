# Whisper.cpp Local ASR Post-Qualification Evidence

**Result:** Passed
**Production source commit:** `29759a18d7c91c26113d33e16a55c6d7f80bdb0d`
**Acceptance checkout:** `cf3280ea1d4e91d298828e4935e3369f8cbd6be6` (`master`, documentation-only descendant of the production source commit)
**Ordinary CI:** [run 37755875840](https://github.com/ekkus93/ai-talking-moose/actions/runs/37755875840), successful at the acceptance checkout
**Real-CPU acceptance:** [run 37756211710](https://github.com/ekkus93/ai-talking-moose/actions/runs/37756211710), job `113241297405`, successful
**Artifact:** `whisper-real-cpu-37756211710-1-cf3280ea1d4e91d298828e4935e3369f8cbd6be6` (artifact ID `11540816318`, ZIP 7,053 bytes, SHA-256 `c225ae231d770c58ebaa58fa06859966fd32cb54f0bbd28b283ede766b3381ae`)
**Artifact retention:** through 2026-10-11 09:22:40 UTC

The artifact was downloaded and checked locally. Its evidence JSON, install/delete/reinstall reports, and transcript report agree on the tested repository SHA, model identity, source identity, model hash, and model byte count. The workflow's evidence validator and upload steps both passed.

## Provenance and install lifecycle

| Field | Verified value |
| --- | --- |
| Repository checkout | `cf3280ea1d4e91d298828e4935e3369f8cbd6be6` |
| whisper.cpp expected / actual source | `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774` / same |
| Model ID / revision | `whisper-small-ggml` / `5359861c739e955e79d9a303bcbc70fb988958b1` |
| Model SHA-256 | `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b` |
| Model bytes | `487601967` |
| Install / delete / reinstall | Both installs reported `installed`; delete reported `removed: true` |
| Runtime / network denial | Native runtime linked; network denial probe passed |
| Test corpus | `jfk.wav`, 352,078 bytes, SHA-256 `59dfb9a4acb36fe2a2affc14bacbee2920ff435cb13cc314a08c13f66ba7860e` |

## Measured Linux x86_64 CPU run

Runner CPU: AMD EPYC 7763, 4 available parallel workers. These measurements apply to this 11-second corpus and runner only; they are not general latency or throughput guarantees.

| Metric | Result |
| --- | ---: |
| Transcript | “And so my fellow Americans, ask not what your country can do for you, ask what you can do for your country.” |
| Partial / final events | 2 / 1 |
| First partial / final latency | 8,279 ms / 17,099 ms |
| Processed audio / inference wall time / RTF | 11,000 ms / 9,808 ms / 0.892 |
| Pipeline CPU time / average CPU utilization | 38,856 ms / 226.751% |
| Pipeline peak process RSS | 707,932,160 bytes |
| Nominal dropped chunks | 0 |
| Deliberate overload | 64 attempted, 56 accepted, 8 dropped; queue capacity 56 |

The harness used an 80,000-sample partial interval, 8,000-sample endpoint silence, and 480,000-sample maximum utterance. The accepted transcription completed with network access denied after installation.
