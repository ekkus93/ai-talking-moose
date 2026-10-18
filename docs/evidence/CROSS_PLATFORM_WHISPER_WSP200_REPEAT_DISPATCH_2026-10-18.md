# WSP-200: repeated real-CPU macOS transcription — workflow and dispatch evidence

**Observed:** 2026-10-18T22:12:15.767Z (UTC)  
**Repository:** `ekkus93/ai-talking-moose`  
**Implemented source:** `de8b2ae89b5fef250e68797aaddb21a96e4d3f7a` (`test(whisper): measure repeat cold-start genuine Mac transcription`).  
**Changed workflow:** `.github/workflows/whisper-macos-real-cpu-acceptance.yml`.

## Change and qualification boundary

The opt-in/manual macOS matrix workflow now runs **two separate real-model transcription processes** per Apple Silicon and Intel job, using the same explicitly verified model installation. Both must prove CPU-only native diagnostics, pinned source/model, meaningful fixed-corpus text, and valid JSON pipeline observations. The second run records model-load and inference wall time, pipeline RTF, and final-event and nominal-queue-drop assertions. Weight files remain in runner-local user/test data and are not uploaded; artifacts remain JSON-only. This is repeated **cold-start** evidence, not sustained same-session inference or a live microphone test.

At `de8b2ae89b5fef250e68797aaddb21a96e4d3f7a`, [main CI 39170995361](https://github.com/ekkus93/ai-talking-moose/actions/runs/39170995361) and [Wake Word audit 39170995403](https://github.com/ekkus93/ai-talking-moose/actions/runs/39170995403) both completed **success**. Those push gates do **not** execute this manual 488 MB workflow and cannot qualify the new assertions or real-model performance.

## Ralph Bridge dispatch incident (no successful workflow dispatch)

- Runtime: `ralph-bridge 0.6.2`, build `0acea4b23362e2ccecab8f6c9db4418b162426cf`, instance `ab7dc9b1f568483d9f4dd5619f985b84`, started `2026-10-18T22:05:30.667826Z`; auth mode `app` (secret-free).
- Method: `dispatch_validation`; sanitized arguments: repo `ekkus93/ai-talking-moose`, ref exact `de8b2ae89b5fef250e68797aaddb21a96e4d3f7a`, configured-profile candidate `whisper-macos-real-cpu-acceptance`. No source body, credentials, user model or environment secrets included.
- Exact returned error: `INVALID_ARGUMENT: Error code: INVALID_ARGUMENT; Error: RuntimeException: Error calling MCP tool: [TextContent(type='text', text='Error executing tool dispatch_validation', annotations=None, meta=None)]`.
- Reconciliation: current head was re-read and remained `de8b2ae89b5fef250e68797aaddb21a96e4d3f7a`; no workflow dispatch ID or correlated CI run was returned or claimed.
- Historical correlation: the same profile and `whisper-macos-real-cpu` previously returned generic `INVALID_ARGUMENT` on 2026-10-15; see `docs/evidence/CROSS_PLATFORM_WHISPER_WSP200_DISPATCH_2026-10-15.md`.
- Classification: Ralph Bridge dispatch **request failure with undetermined internal cause**. The generic response does not establish whether the profile is unavailable, GitHub App authorization is insufficient, GitHub rejected the request, or the bridge failed internally. It is **not** evidence of an OpenAI platform safety denial, a failed transcription, a stale-head write, or rate limiting. No duplicate dispatch was attempted in this run.

## Required independent action

Have the operator inspect secret-free Ralph Bridge `dispatch_validation` profile configuration, the method's server-side error/log correlation, and App permission for Actions workflow dispatch. Alternatively an authorized human may manually start **Whisper macOS Real CPU Acceptance** on `master` from the GitHub Actions UI **while master still points to the intended source**. Capture the actual triggered SHA and run ID; accept a run only when both architectures finish successfully with genuine-model artifacts. This action is **not yet completed** and needs operator/human help.

WSP-200 network-denied macOS acceptance, same-session sustained Intel performance, clean physical microphone/TTS checks, WSP-500 Moonshine removal, signing/notarization and WSP-700 remain open. Preserve KittenTTS's separate ONNX Runtime and all Moonshine safety gates.
