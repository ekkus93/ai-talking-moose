# Whisper macOS real-CPU dispatch and write reconciliation — 2026-10-20

**Scope:** Ralph Bridge-only inspection and exact-head source operations for `ekkus93/ai-talking-moose`. Secret-free operational evidence; no credentials, user model files, complete source bodies, or raw transcripts are included.

## Confirmed source correction and CI scope

- Connected runtime: Ralph Bridge **0.6.2**, build `0acea4b23362e2ccecab8f6c9db4418b162426cf`, instance `cfe09172c19941b1a9d8ba5a73f57aa0`, GitHub App authentication.
- The previous hourly run reported a **platform-level safety denial** at **2026-10-20 14:05:32 UTC** for a one-file `commit_files` formatting-only workflow correction, expected master `4b008c16a30408707302782c698eeae85d41250d`. Its reported remote reconciliation found no accepted write. Do not conflate that explicit safety denial with the validation-dispatch error below.
- On the **next scheduled run**, the observed master was still `4b008c16a30408707302782c698eeae85d41250d`. A new exact-head `commit_files` operation `c7eb07f9e3574b80bba363139a290dcb` **succeeded** and verified the single changed path `.github/workflows/whisper-macos-real-cpu-acceptance.yml`, producing `8e4aa53d61ab79f46c4211ad89b855638bca987c`. This corrected literal `\\n` characters in the Seatbelt explanatory YAML comment and added the missing **first-run** `nominal_dropped_chunks == 0` assertion (the repeat report already had that assertion). No model pin, Seatbelt enforcement, KittenTTS/ORT dependency or Moonshine shipping behavior was changed.
- At this exact source, [CI 39445995996](https://github.com/ekkus93/ai-talking-moose/actions/runs/39445995996) **passed CI plumbing only**; Rust, frontend, native macOS build and full packaging jobs were skipped. [Wake Word required-gates audit 39445996369](https://github.com/ekkus93/ai-talking-moose/actions/runs/39445996369) **passed**. Neither run executed model-backed transcription or physical acceptance. A successful later write does not explain the earlier platform safety denial.

## Repeated manual-dispatch failure, reconciled without duplicate submission

- UTC observation: **2026-10-20T15:10:47.871Z**.
- Method: Ralph Bridge `dispatch_validation`.
- Sanitized argument shape: `{repo:"ekkus93/ai-talking-moose",ref:"master",profile:"whisper_macos_real_cpu"}`; no custom workflow inputs, shell commands, credentials or model files supplied.
- Expected and immediately observed master: `8e4aa53d61ab79f46c4211ad89b855638bca987c`.
- Exact returned error: `INVALID_ARGUMENT: Error code: INVALID_ARGUMENT; Error: RuntimeException: Error calling MCP tool: [TextContent(type='text', text='Error executing tool dispatch_validation', annotations=None, meta=None)]`.
- Operation ID: **not returned**. GitHub Actions workflow-run ID: **not returned**. A fresh exact-SHA `get_ci_runs` reconciliation showed only push CI `39445995996` and Wake Word audit `39445996369`, with **no observed workflow_dispatch run**. One dispatch attempt only; no alternate-profile guessing or same-run retry.
- Classification: **unresolved generic Ralph/MCP validation-dispatch error**. No evidence proves an OpenAI safety denial, a GitHub permission or API rejection, invalid configured profile, stale-head write, or an actual inference/test failure. The repeated pattern across prior observations (see `docs/evidence/RALPH_BRIDGE_INCIDENTS_2026-10-19.md`) warrants a Bridge-side diagnostic rather than repeated blind attempts.

## Actionable Bridge diagnostics request

Expose a **secret-free** configured validation-profile inventory and per-call structured stage (profile lookup, policy preflight, GitHub workflow resolution, dispatch request, or response parsing); retain an operation/correlation ID and underlying GitHub HTTP status/error category if one exists. For accepted but ambiguous dispatches, expose a durable workflow-run receipt. Do not reveal installation tokens, model files or full source payloads. Until dispatch is verified and real weight-backed transcription runs under macOS Seatbelt on **both** architectures, keep WSP-200 enforced-offline acceptance and WSP-500 Moonshine removal **OPEN**. Intel sustained performance, physical microphone, System TTS and signed release remain separate gates.
