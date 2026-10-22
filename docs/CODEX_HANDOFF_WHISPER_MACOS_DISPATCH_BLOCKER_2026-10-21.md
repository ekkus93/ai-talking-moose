# Codex handoff — unblock Ralph Bridge macOS Whisper acceptance dispatch

**Prepared:** 2026-10-21 (America/Los_Angeles; some GitHub observations fall on 2026-10-22 UTC)  
**Canonical handoff location:** `ekkus93/ai-talking-moose/docs/CODEX_HANDOFF_WHISPER_MACOS_DISPATCH_BLOCKER_2026-10-21.md` (copy corrected from earlier placement in the Ralph Bridge repository).  
**State:** Action required in Ralph Bridge; caller-visible failure reproduced across restart. **Root cause is not established.**  
**Bridge repository:** `ekkus93/chatgpt_ralph_loop_tools` — write directly to `master`, no per-task branches/PRs.  
**Consuming repository:** `ekkus93/ai-talking-moose` — `docs/CROSS_PLATFORM_WHISPER_ASR_TODO_2026-10-15.md`.  
**Primary incident report (in Ralph Bridge repository):** [`docs/BUG_REPORT_WHISPER_MACOS_DISPATCH_INVALID_ARGUMENT_2026-10-21.md`](https://github.com/ekkus93/chatgpt_ralph_loop_tools/blob/master/docs/BUG_REPORT_WHISPER_MACOS_DISPATCH_INVALID_ARGUMENT_2026-10-21.md). Read it before changing code.

## 1. Why this is being handed off

The assistant cannot complete the remaining macOS Whisper acceptance gate because calling the trusted Phase 2 workflow-dispatch tool repeatedly results in a generic MCP error, without structured refusal/receipt:

```text
INVALID_ARGUMENT: Error code: INVALID_ARGUMENT; Error: RuntimeException:
Error calling MCP tool: [TextContent(type='text', text='Error executing tool dispatch_validation', annotations=None, meta=None)]
```

The profile being requested is `whisper_macos_real_cpu` for `ekkus93/ai-talking-moose`. It should launch the explicitly authorized, manual-only macOS real-model acceptance workflow (macOS arm64 and x86_64) that performs pinned-weight inference under enforced Seatbelt egress denial. Push-triggered CI, compilation, and unsigned-app tests cannot substitute for that evidence.

**The last retry occurred after the operator restarted the bridge.** The connection, authenticated reads, and Phase 4 source writes work. This is a *Phase 2 validation dispatch* symptom, not a demonstrated global GitHub/tool outage, platform safety denial, Whisper engine defect, or proof of a specific permission issue. Do not simply restart again and call the incident resolved.

## 2. Exact observed state (verified through Ralph Bridge)

### Ralph Bridge server

- Semantic version: `0.6.2`; build SHA `0acea4b23362e2ccecab8f6c9db4418b162426cf`; GitHub auth mode `app`.
- Previous server instance `c75f6fc5b7794249adb6bf78a7d3605b` (started `2026-10-21T09:25:23.395951Z`).
- Restarted server instance `ea90f0213d5d4ea790583f2f94dc483d` (started `2026-10-21T23:58:57.374980Z`).
- The MCP ping returned `pong` on both. The restarted server could read both repositories and resolve refs, and `get_ci_runs` and `get_ci_status` worked.
- The original bug report was committed on bridge repository `master` at `311fa96e6831966a0b579b96fe1679c98954ae3b`; the earlier handoff was then committed there at `4df34bc0ac4dad7185aac475a2a5a81ca5bb7d0d`. These documentation-only commits did not change dispatch code or production config.
- Bridge documentation-commit CI: [run `39697701892`](https://github.com/ekkus93/chatgpt_ralph_loop_tools/actions/runs/39697701892) was **queued** at last check. Recheck rather than assuming a pass.

### AI Talking Moose project

- Exact observed `master`: `500b3dfa8b9f100917d6efde39e0a1554f508ce9`.
- Recently landed directly on `master`: `20615147aba272b403dd1810672442e17c33ae7d` adds a fixture regression for a cancelled Whisper model installer waiting behind another active download. `500b3dfa8b9f100917d6efde39e0a1554f508ce9` fixes its Rust formatting.
- **Qualified at the exact latest project SHA:** [Main CI `39693531744`](https://github.com/ekkus93/ai-talking-moose/actions/runs/39693531744), [native macOS compile `39693531695`](https://github.com/ekkus93/ai-talking-moose/actions/runs/39693531695), [pre-cutover macOS app smoke `39693531643`](https://github.com/ekkus93/ai-talking-moose/actions/runs/39693531643), and [Whisper-only macOS app smoke `39693531671`](https://github.com/ekkus93/ai-talking-moose/actions/runs/39693531671) **all completed successfully**. Native compile passed both ARM64 and x86_64.
- The independent legacy [ASR-015 workflow `39693531666`](https://github.com/ekkus93/ai-talking-moose/actions/runs/39693531666) **failed in pinned Moonshine runtime preparation**, an ONNX Runtime archive-size mismatch; it is not a Whisper macOS manual-dispatch failure.
- Earlier real pinned-model two-architecture CPU transcription was achieved: see project TODO evidence for `38886320925`, `39091829528`, and `39174236096`. Those do **not** qualify enforced macOS offline/Seatbelt isolation. Intel sustained performance remained questionable (repeated RTF > 1 in historical evidence).
- The **manual two-architecture Seatbelt egress-denied transcription** remains unqualified; WSP-200/400/500/600/700 checkboxes must not be closed just because builds pass.

## 3. Minimal reproduction and reconciliation

First verify the configured operator policy and current exact target head. The last observed intended call was:

```json
{
  "repo": "ekkus93/ai-talking-moose",
  "ref": "500b3dfa8b9f100917d6efde39e0a1554f508ce9",
  "profile": "whisper_macos_real_cpu"
}
```

A historical attempt with `ref="master"` and an earlier attempt with the prior exact head also failed with the same generic error. On the restarted instance, the exact-SHA request failed. A subsequent `get_ci_runs(repo="ekkus93/ai-talking-moose", ref_or_sha="<same SHA>")` showed **only five push runs and no manual `workflow_dispatch` event**, with no operation ID or workflow-run ID returned by the failed call.

**Caution:** A missing run in one bounded read is not conclusive proof that a write never reached GitHub. Do not blindly retry a write after an ambiguous acknowledgment. Inspect Ralph server audit records and reconcile against actual workflow runs first. Reproduce against a disposable/authorized workflow when possible, then perform a single explicitly authorized production acceptance dispatch.

## 4. Precise code and configuration entry points

Investigate these files on **`ekkus93/chatgpt_ralph_loop_tools` `master`** (source locations verified before this handoff; these paths are not in `ai-talking-moose`):

| File | Role |
| --- | --- |
| `src/ralph_bridge/mcp/phase2_tools.py` | Exposes `dispatch_validation(repo, ref, profile)`; calls service through `call_async_safely`. |
| `src/ralph_bridge/services/dispatch_validation.py` | Profile resolution, Phase 2 write guard, workflow/ref preflight, dispatch, receipt correlation, `WriteAuditEvent`. |
| `src/ralph_bridge/validation_profiles.py` | Trusted profile resolution and configured inputs. |
| `src/ralph_bridge/github/dispatch.py` | GitHub Actions dispatch integration, workflow and ref lookup. |
| `src/ralph_bridge/errors.py` and MCP safe-call/error adapter | Domain-to-MCP error translation and sanitization; trace why caller only sees generic `INVALID_ARGUMENT`. |
| `config/ralph.example.yaml` | Example of `phase2.enabled`, `phase2.write_repositories`, `phase2.validation_profiles`, per-profile workflow and run correlation. **Not the actual private deployed config.** |
| `tests/unit/test_phase2_dispatch_validation_service.py` | Existing hermetic Phase 2 service test suite. |
| `tests/unit/test_phase2_dispatch_validation_edges.py` | Ref, workflow and API error edge tests. |
| `tests/unit/test_phase2_foundation.py` | Write permission/error mapping and safe logging regressions. |
| `docs/PHASE2_REGRESSION_ACCEPTANCE.md` | Contract that errors such as `VALIDATION_PROFILE_NOT_FOUND`, `REF_NOT_DISPATCHABLE`, `WORKFLOW_NOT_FOUND` and `WRITE_PERMISSION_REQUIRED` are typed and actionable. |

The service's current order is: `parse_repository` -> `config.require_write_repository` -> `resolve_github_validation_profile` -> `actions.get_workflow` -> `actions.resolve_dispatch_ref` -> re-verify workflow/ref -> `trusted_profile.build_inputs` -> `actions.dispatch_workflow` -> verify/correlate run -> emit audit event. Normal domain failures are expected to be `RalphToolError`; unexpected exceptions are audited as `INTERNAL_ERROR`. Inspect exactly which stage fails in server logs rather than guessing based on the outer MCP wrapper.

The README documents independent Phase 2 write authorization and **GitHub App Actions: read/write** permission. An installation may require separate approval even after updating the GitHub App's registration. Successful Phase 4 Contents writes **do not** establish Actions write access.

## 5. Codex: prioritized investigation and implementation plan

1. **Reconcile HEAD and deployment first.** Read this file in `ai-talking-moose` and the primary incident in `chatgpt_ralph_loop_tools`. Inspect the current `master` heads of **both repositories**, existing work and tests; avoid new branches/PRs and concurrent writes. Identify which executable/config the running MCP tunnel really launched. Compare its build/instance to current source. Do not include private config or tokens in commits.
2. **Inspect private server-side logs/audit.** Search by the restarted instance ID, `dispatch_validation`, `ekkus93/ai-talking-moose`, and profile name. Obtain the exception class and *stage* as well as any GitHub HTTP status, `precondition`, `github_result`, and operation/correlation ID. Do not expose private credentials or full workflow inputs.
3. **Validate actual Phase 2 settings, safely.** Confirm `phase2.enabled`, exact `phase2.write_repositories` membership, presence of the `whisper_macos_real_cpu` profile, intended workflow filename/inputs, whether workflow supports `workflow_dispatch`, and ref dispatchability. Confirm **installed** App Actions permission, not just registration. Do not make a broad allowlist/permission change as a workaround.
4. **Repair the specific cause.** If configuration is wrong, document and correct only the operator-authorized setting. If it is transport/error-boundary code, preserve domain errors through service -> MCP -> tunnel, including stable error code, bounded message, safe details, and retryability. If an accepted GitHub POST could have lost its response, maintain an explicit **unknown state** and audit correlation rather than reporting rejection or resubmitting blindly.
5. **Add focused negative/positive tests** around the diagnosed failure plus adjacent typed-error propagation, API 403/404/422/429/5xx/timeout, uncorrelated accepted dispatch, and secret redaction. Assert denial paths issue **no unauthorized POST**. Existing tests cover some of these: extend only meaningful gaps, rather than replacing prior coverage.
6. **Run local quality checks.** Project uses Python >=3.11, `uv` and the `dev` dependency group; suitable local commands include `uv sync --group dev`, `uv run pytest -q tests/unit/test_phase2_dispatch_validation_service.py tests/unit/test_phase2_dispatch_validation_edges.py`, `uv run ruff check .`, `uv run ruff format --check .`; follow existing repository CI for full checks and mypy. Live write tests must remain explicitly opt-in and run only with operator approval.
7. **Commit any Ralph Bridge fix directly to `chatgpt_ralph_loop_tools/master`; verify exact-head CI** and deploy/restart the updated bridge **only via the operator's approved procedure**. Check `get_runtime_info` confirms the *new build/instance*. A source commit by itself does not update the running MCP process.
8. **Prove the actual integration path** with a controlled configured workflow, then a single authorized `whisper_macos_real_cpu` dispatch at a verified `ai-talking-moose/master` head. Report accepted vs rejected/unknown unambiguously, provide any real GitHub run ID, and re-read the run via Ralph Bridge. Fix/refine anything ordinary that fails; preserve all safety gates.
9. **Handoff back with a concise receipt:** root cause; exact changed paths and new commit; tests and exact-head CI run URLs/conclusions; deployed build/instance; redacted policy/permission outcome; dispatch response/run ID or precise remaining blocker. Do not claim WSP acceptance from a merely queued, cancelled, mock, or non-isolated run.

### Helpful local diagnostic commands (for Codex on the operator's machine)

```bash
git status --short
git rev-parse HEAD
rg -n 'dispatch_validation|call_async_safely|VALIDATION_PROFILE_NOT_FOUND|REF_NOT_DISPATCHABLE' src tests docs
uv run pytest -q tests/unit/test_phase2_dispatch_validation_service.py tests/unit/test_phase2_dispatch_validation_edges.py
```

These are examples of **local**, non-GitHub-write investigation commands; the user's rule for this ChatGPT workflow was Ralph Bridge exclusively for remote GitHub/CI. Codex may use the operator's normal local Git/CI environment as authorized. Inspect server environment and config in place; do not paste secrets into reports, chats, or committed files.

## 6. What's blocked in AI Talking Moose, beyond the bridge defect

**Immediate bridge-unblocked task:** run the manual macOS genuine pinned-model acceptance with enforced Seatbelt egress denial on both architectures and record real logs/reports at an exact SHA. The earlier macOS real-CPU tests established two-arch inference, but not the new enforced offline requirement.

**Still open even after successful dispatch:**
- WSP-200: offline inference evidence, representative lifecycle/performance/thermal and acceptable Intel RTF, installer ownership under real workloads.
- WSP-300/400: remaining full settings/provider/audio capture/wake/mic/TTS/app-session matrix and negative/physical tests.
- WSP-500: gated deletion of Moonshine native runtime, UI, workflows, packaging paths **only after its migration prerequisites**; retain independently required KittenTTS ONNX Runtime.
- WSP-600: full exact-head release/packaging policy, provenance, no stray dynamic linkage, signing/notarization and docs/ELSV reconciliation.
- WSP-700: Apple Silicon and Intel physical mic, System TTS audition, privacy, upgrade/clean install and release acceptance. These cannot be inferred from CI.

Use this repository's `docs/CROSS_PLATFORM_WHISPER_ASR_SPEC_2026-10-15.md` and full `docs/CROSS_PLATFORM_WHISPER_ASR_TODO_2026-10-15.md` as authority. Preserve unchecked acceptance items without evidence. Do **not** delete Moonshine simply to make its legacy ASR-015 gate green: replacement of that workflow is gated by verified functionality and shipping requirements.

**Automation note:** The earlier hourly AI Talking Moose development task was disabled after a source-write safety interruption; subsequent exact-head source commits succeeded, but the dispatch blocker persists. Do not assume the task is enabled or that a live automated loop will independently resume. After a fix, tell the user what to re-enable.

## 7. Completion criterion for returning this to ChatGPT

The critical path is unblocked when the deployed, verified Ralph Bridge can (a) explain any expected dispatch refusal with a typed actionable error **and** (b) submit a policy-authorized configured `whisper_macos_real_cpu` run, yielding an auditable accepted/unknown/rejected outcome and read-side workflow evidence.

Do not conflate that bridge-level resolution with passing the Mac offline inference itself. Report the latter separately with both architecture job results, Seatbelt denial attestations, pinned source/model hashes and run/artifact IDs. All other product gates remain independently open until supported by evidence.
