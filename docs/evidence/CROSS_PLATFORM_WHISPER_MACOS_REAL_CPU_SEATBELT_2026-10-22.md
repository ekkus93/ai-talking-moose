# macOS Whisper Seatbelt Real CPU Acceptance — Pass with Intel Performance Limit

Manual workflow [run 39860766151](https://github.com/ekkus93/ai-talking-moose/actions/runs/39860766151) passed at exact `master` SHA `6009d63c249db22b73f35af535d1d21ae879b084`. Both architecture jobs passed pinned Whisper Small installation, clean reinstall, missing/corrupt model rejection, CPU-only real JFK transcription under macOS Seatbelt egress denial, repeated cold-start inference, pipeline lifecycle regressions, evidence assertions, and model deletion. Job IDs: arm64 `120110572862`; x86_64 `120110573140`.

Both reports confirm network-denial enforcement and meaningful transcription using the pinned 487,601,967-byte model (SHA-256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`) and whisper.cpp source `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`. The workflow and JSON artifacts are retained below; GitHub artifact hashes cover the uploaded ZIPs, while JSON files are individually archived here.

| Architecture | Inference threads | Run | First partial / final (ms) | RTF | Queue peak / capacity | Nominal drops |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| arm64 | 3 | first | 7,066 / 13,965 | 0.486 | 17 / 56 | 0 |
| arm64 | 3 | repeated | 7,298 / 14,124 | 0.509 | 19 / 56 | 0 |
| x86_64 | 4 | first | 9,605 / 20,359 | 1.267 | 40 / 56 | 0 |
| x86_64 | 4 | repeated | 9,994 / 19,649 | 1.249 | 44 / 56 | 0 |

The new nominal queue diagnostics show no dropped chunks on Intel in either run, with no first-drop timestamp; its queue nevertheless reached 44 of 56 chunks. Intel RTF remains above 1 in both cold starts, so it did not process the 11-second audio corpus in real time. This clears the enforced-egress inference subtask but not WSP-200's performance, sustained/thermal, app lifecycle, physical microphone, packaged-app, or full installer matrix. Keep Moonshine removal gated.

GitHub artifact records:

- arm64 artifact `12461756874`, 3,990 bytes, SHA-256 `13d7c51d569662663e2f6818d26aaf293752155dda03a8c81938a47f19e6a75b`.
- x86_64 artifact `12461788794`, 4,041 bytes, SHA-256 `26b11ea3e835e9207c137462434460df7913dbb5459c2564343f2b3aa91a04e7`.

This run does not establish sustained thermal behavior, live microphone/TTS, a packaged signed/notarized app, or a fix for Ralph Bridge dispatch.
