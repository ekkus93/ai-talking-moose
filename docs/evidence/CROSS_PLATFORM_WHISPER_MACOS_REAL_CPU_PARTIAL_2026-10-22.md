# macOS Whisper Seatbelt real CPU acceptance — partial (2026-10-22)

Manual workflow [run 39701495688](https://github.com/ekkus93/ai-talking-moose/actions/runs/39701495688) ran against exact `master` source `53604c421a38c35adf545f42f75206935ca2a5c8`. The workflow was dispatched directly through GitHub after Ralph Bridge returned an opaque error; this does not diagnose or repair the bridge. The run finished **failure** overall.

Both architecture jobs installed the pinned Whisper Small model through the production installer, performed genuine JFK corpus transcription under macOS Seatbelt egress denial, and uploaded JSON reports. The reports identify model SHA-256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`, 487,601,967 bytes, and whisper.cpp source commit `60c0be6ac8fa71b1a2ae2dd938a31a34a508e774`. Both reported `network_denied_required=true` and `network_denial_probe_passed=true`.

| Architecture | Job | Result | First / repeated RTF | Nominal dropped chunks | First / repeated final latency |
| --- | ---: | --- | ---: | ---: | ---: |
| arm64 | `119571539988` | Passed | 0.477 / 0.501 | 0 / 0 | 14,112 / 13,915 ms |
| x86_64 | `119571539711` | Failed final evidence assertion | 1.499 / 1.644 | 0 / 1 | 22,027 / 23,305 ms |

The Intel job's build, lifecycle regressions, model installation, both transcriptions, and upload passed. The final assertion failed because the repeated pipeline reported one nominal queue drop; the acceptance workflow requires zero. Intel used four inference threads. The RTF above 1 also leaves real-time performance unqualified. Do not close the enforced-offline WSP-200 checkbox or downstream gates on this partial run.

The uploaded JSON artifacts are archived in [`whisper-macos-real-cpu-39701495688/`](whisper-macos-real-cpu-39701495688/). GitHub artifact records:

- arm64 artifact `12393257285`, `whisper-macos-real-arm64-53604c421a38c35adf545f42f75206935ca2a5c8`, 3,946 bytes, SHA-256 `b486c0423d8b1d016c157d9421d1ff421510b6a1e31cbc9fd33b7ecf09a29704`.
- x86_64 artifact `12393657090`, `whisper-macos-real-x86_64-53604c421a38c35adf545f42f75206935ca2a5c8`, 3,993 bytes, SHA-256 `f4641ce861b5a6d68a5529a8e38c2b171162a725433a44edb23bcc49f59f12b4`.

Artifacts contain JSON evidence only, no model weights. Both runners also reported successful model deletion after inference. This run does not establish physical microphone/TTS, sustained thermal performance, packaged-app release acceptance, or a Ralph Bridge fix.
