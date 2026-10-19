# Ralph Bridge write/validation incidents — 2026-10-19 (secret-free)

Connected runtime at observation: semantic version `0.6.2`, exact build `0acea4b23362e2ccecab8f6c9db4418b162426cf`, server instance `ecf828a193874460b2f7b3088fab8c5b`, auth mode `app`. No tokens, personal keys, model files, or complete source bodies are included.

## Incident 1 — acknowledged remote write despite generic tool error

- Error observed **2026-10-19T10:25:20.582Z UTC**, Ralph Bridge method `commit_files`.
- Sanitized shape: `repo=ekkus93/ai-talking-moose`, `branch=master`, `expected_head_sha=04ba233418b593888a24c176c46a0f0c6c12de8f`, two UTF-8 upserts at `src-tauri/src/asr/whisper/acceptance.rs` and `.github/workflows/whisper-macos-real-cpu-acceptance.yml`; no complete bodies logged.
- Returned error: `INVALID_ARGUMENT: Error code: INVALID_ARGUMENT; Error: RuntimeException: Error calling MCP tool: [TextContent(type='text', text='Error executing tool commit_files', annotations=None, meta=None)]`.
- No operation ID or accepted result was surfaced to the caller. **Reconciliation, with no retry:** fresh Ralph Bridge `get_repo_state(master)` returned `6df93609488bb20965e5bc81a57d65db58d69332` with parent exactly `04ba233418b593888a24c176c46a0f0c6c12de8f` and expected commit message. Ralph `get_diff` and paginated file reads confirmed the intended two-file correction had landed. Treat as an **ambiguous post-commit tool response**, not confirmed denied permission, stale head, platform safety denial or GitHub API cause.
- Correlated CI at `6df9360`: [39237342725](https://github.com/ekkus93/ai-talking-moose/actions/runs/39237342725) (CI, later superseded/cancelled), [39237342854](https://github.com/ekkus93/ai-talking-moose/actions/runs/39237342854) (native), [39237342842](https://github.com/ekkus93/ai-talking-moose/actions/runs/39237342842) (unsigned app), [39237342838](https://github.com/ekkus93/ai-talking-moose/actions/runs/39237342838) (pre-cutover smoke). The separate legacy [ASR-015 39237342823](https://github.com/ekkus93/ai-talking-moose/actions/runs/39237342823) failed Moonshine dependency preparation; this does not identify the cause of the Bridge error.
- Actionable investigation: verify whether GitHub write acceptance and the MCP response/error boundary can diverge after an accepted operation, and ensure operation ID plus accepted head/commit receipt remain accessible on post-write inspection failure. No unsafe duplicate write was attempted.

## Incident 2 — manual acceptance dispatch not confirmed

- Error observed **2026-10-19T10:30:24.968Z UTC**, Ralph Bridge method `dispatch_validation`.
- Sanitized shape: `repo=ekkus93/ai-talking-moose`, exact ref `3979a5ffaccafbdedf68f83ba38b4ff6baac0c7f`, requested configured profile name `whisper_macos_real_cpu`.
- Returned error: `INVALID_ARGUMENT: Error code: INVALID_ARGUMENT; Error: RuntimeException: Error calling MCP tool: [TextContent(type='text', text='Error executing tool dispatch_validation', annotations=None, meta=None)]`.
- No operation ID, workflow run ID, or accepted response was returned. Fresh Ralph Bridge reads confirmed `master` still at `3979a5...` then, and `get_ci_runs` showed only [Main CI 39237846328](https://github.com/ekkus93/ai-talking-moose/actions/runs/39237846328) from a push, **no confirmed `workflow_dispatch` run** at the exact target SHA. No alternate profile guesses or repeated dispatch attempts were submitted.
- Cause unresolved: the error alone does **not** establish a missing profile, permission defect, safety denial or GitHub API failure. Actionable follow-up: expose allowed validation profile names (secret-free) and structured rejection reasons/operation IDs to callers; then safely dispatch the configured manual Mac real-CPU profile, or have the owner run it explicitly.

Neither incident establishes a platform-level safety denial. Reconcile remote state before considering any later write/dispatch. Preserve no-model-upload and exact-head integrity requirements.

## Incident 3 — manual Seatbelt acceptance dispatch still not confirmed after reconnect

- UTC observation: `2026-10-19T20:06:10.937Z`. Connected Ralph runtime `0.6.2`, build `0acea4b23362e2ccecab8f6c9db4418b162426cf`, instance `5c4e195cb21747e5ab7fdec92490e86f`, auth mode `app`; `ralph_ping` returned `pong`.
- Method: `dispatch_validation`. Sanitized shape: `repo=ekkus93/ai-talking-moose`, `ref=867945a37129ae736fa9573ea480a375672f0d3d` (observed master), `profile=whisper_macos_real_cpu`. No source bodies, credentials or model contents transmitted.
- Exact error: `INVALID_ARGUMENT: Error code: INVALID_ARGUMENT; Error: RuntimeException: Error calling MCP tool: [TextContent(type='text', text='Error executing tool dispatch_validation', annotations=None, meta=None)]`. No operation ID or CI run ID surfaced.
- Immediate read-only reconciliation: `get_repo_state(master)` remained `867945a37129ae736fa9573ea480a375672f0d3d`; `get_ci_runs` for that SHA returned only push CI `39245728475` (success), **no confirmed manual workflow-dispatch run**. This is an ambiguous Ralph/MCP tool failure, not evidence of a platform safety denial, missing authorization, stale-head write, GitHub rejection or an ASR test failure. No duplicate dispatch was attempted.
- Related exact-source CI: Mac native `39244418942`, unsigned Whisper app `39244418993`, pre-cutover app `39244419064` all passed at parent source `e36be4fc829b9a2f6659f1eea42779bea8da61b2`; Moonshine legacy ASR-015 `39244418953` failed at native runtime preparation. No weight-backed Seatbelt result exists from this attempt.
- Actionable investigation: expose permitted validation profiles and structured preflight/dispatch errors, and preserve operation IDs/run receipts on ambiguous dispatch failure. Do not infer the underlying failure layer from this generic message. Retry only after verifying profile authorization/configuration and remote run state.
