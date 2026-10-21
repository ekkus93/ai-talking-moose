# WSP-200/600 — Manual-only macOS Whisper real-weight acceptance policy

**Recorded:** 2026-10-21 UTC
**Repository:** `ekkus93/ai-talking-moose`
**Specification:** `docs/CROSS_PLATFORM_WHISPER_ASR_SPEC_2026-10-15.md`
**Checklist:** `docs/CROSS_PLATFORM_WHISPER_ASR_TODO_2026-10-15.md`

## Implemented and qualified (static policy only)

At exact master source `d45a450a32a4d7b57efa9dc01f958e4096334c3e`, the workflow checker in `scripts/check_whisper_macos_acceptance_workflow.py` requires the 488 MB, real-model macOS acceptance workflow's top-level `on:` mapping to contain exactly one bare `workflow_dispatch:` trigger. This rejects additional or alternative events (including `issue_comment`, `workflow_call`, `release`), inline/list event forms, duplicate top-level `on:` mappings and unreviewed manual-trigger inputs. The accompanying mutation suite adds the corresponding fail-closed cases.

- Initial source `85ee1d53d0f8decfb7f0e9799be7a6376e86caaa` had escaped Python source/fixture mistakes; [CI 39577913454](https://github.com/ekkus93/ai-talking-moose/actions/runs/39577913454) **failed** and is not acceptance evidence.
- Corrected source `d45a450a32a4d7b57efa9dc01f958e4096334c3e`, [CI 39578057618](https://github.com/ekkus93/ai-talking-moose/actions/runs/39578057618) **success**, `CI plumbing` job `119148514796`: log confirms `scripts/check_whisper_macos_acceptance_workflow.py` passed and `python3 -m unittest scripts.test_check_whisper_macos_acceptance_workflow` ran **20 tests**, all passing.
- The successful workflow classified this change as **CI plumbing**: Rust quality/tests, generated backend contract, packaging, actual model download, and real inference were skipped at this exact head. It proves only the static/manual-trigger policy.
- Previously pending native macOS build [39570481312](https://github.com/ekkus93/ai-talking-moose/actions/runs/39570481312) at earlier exact source `bd8a209454dcb7a0563f565d9dd0dbff70928716` completed **success** for both ARM64 and Intel. Earlier unsigned app smoke [39570481032](https://github.com/ekkus93/ai-talking-moose/actions/runs/39570481032) and pre-cutover app smoke [39570481133](https://github.com/ekkus93/ai-talking-moose/actions/runs/39570481133) also passed at that earlier source. None substitutes for a latest-head real-model run or physical app acceptance.

## Unresolved Ralph Bridge manual dispatch

One manual `dispatch_validation` call at **2026-10-21T09:27:37.773Z** returned:

```text
INVALID_ARGUMENT: Error code: INVALID_ARGUMENT; Error: RuntimeException: Error calling MCP tool: [TextContent(type='text', text='Error executing tool dispatch_validation', annotations=None, meta=None)]
```

- Secret-free Ralph runtime: semantic version `0.6.2`, build `0acea4b23362e2ccecab8f6c9db4418b162426cf`, instance `c75f6fc5b7794249adb6bf78a7d3605b`, auth mode `app`.
- Method/shape: `dispatch_validation(repo=ekkus93/ai-talking-moose, ref=12d744301fc18065c8ca2f7465dfd51e5e635def, profile=whisper_macos_real_cpu)`.
- The expected and observed master were `12d744301fc18065c8ca2f7465dfd51e5e635def` at dispatch. No operation ID, GitHub HTTP status, or workflow-run receipt was returned. Subsequent Ralph Bridge exact-ref run discovery showed only push CI `39575270206`, with no confirmed manual `workflow_dispatch`.
- **Classification: unresolved generic Ralph/MCP validation-dispatch error.** It is not evidence of a failed transcription, GitHub permission failure, stale head, or a platform safety denial. No blind retry or alternate profile guess occurred this run.
- Bridge operator diagnostic requested: allowed validation profiles, rejection stage, operation ID, and underlying GitHub API status/error category where available; never disclose tokens or model data.

## Remaining gates

Do **not** check the full WSP-200 offline-inference/performance gate or WSP-500 Moonshine removal. The real pinned `ggml-small.bin` must still transcribe under enforced macOS Seatbelt network denial on **both** Mac architectures, with actual reports; sustained Intel performance, real microphone, wake/capture, audible TTS, System Voices, KittenTTS-specific acceptance and signed/release hardware evidence remain distinct. Preserve Moonshine until cutover prerequisites are objectively met, and preserve KittenTTS's separately owned ONNX Runtime. No weights, credentials, transcripts, or user models are part of this report.
