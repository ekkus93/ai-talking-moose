# WSP-200 — macOS Seatbelt egress-denied inference qualification (2026-10-19)

## Implemented, without model-backed acceptance claims

- The optional manual `.github/workflows/whisper-macos-real-cpu-acceptance.yml` installs the pinned Whisper weight *before* creating the macOS Seatbelt sandbox. It executes the two successful transcript commands under `sandbox-exec` with `(allow default)` and `(deny network*)` and `--require-network-denied`.
- The feature-gated acceptance CLI on macOS checks both a kernel Seatbelt `sandbox_check` denial for `network-outbound` and an OS `PermissionDenied` response from an actual loopback connection. A refused connection, timeout or DNS failure alone cannot meet the requirement. An unavailable `sandbox-exec` fails the manual acceptance rather than silently claiming offline inference.
- The JSON contract requires `network_denied_required=true` and `network_denial_probe_passed=true` for **each** architecture and both successful real transcriptions. The static policy verifier requires the exact Seatbelt wrappers, preflight, JSON checks, and rejects Linux namespace substitutions and model artifact uploads.
- Six hermetic negative policy tests were added and run by the CI plumbing gate. No downloaded weights, live secrets, physical microphone or audible TTS are part of these tests.

## Exact-commit evidence

| SHA | Evidence | Outcome / boundary |
| --- | --- | --- |
| `04ba233418b593888a24c176c46a0f0c6c12de8f` | Seatbelt probe + optional manual workflow | Initial draft; subsequently corrected operation terminator/YAML indentation. Do not count its invalid workflow as accepted. |
| `6df93609488bb20965e5bc81a57d65db58d69332` | Corrected macOS probe and workflow | Post-write read verified contents; CI plumbing subsequently flagged an existing policy checker that disallowed all Mac network-denial flags. |
| `3979a5ffaccafbdedf68f83ba38b4ff6baac0c7f` | Static policy checker updated without weakening constraints | [Main CI 39237846328](https://github.com/ekkus93/ai-talking-moose/actions/runs/39237846328) success. |
| `7ffe4b2c9930ddb89ab74d09f2a9cde6753165d2` | Fail-closed mutation tests for unsupported or unprotected Mac offline claims | [Main CI 39238837588](https://github.com/ekkus93/ai-talking-moose/actions/runs/39238837588) success: CI plumbing, unit mutation tests, Rust tests/quality, contract. Wake Word audit [39238837862](https://github.com/ekkus93/ai-talking-moose/actions/runs/39238837862) success. |

The prior genuine-model repeated-process Mac results remain on older sources (e.g., [39174236096](https://github.com/ekkus93/ai-talking-moose/actions/runs/39174236096)) and explicitly had `network_denied_required=false`. They are **not** retrospective Seatbelt passes. No manual Seatbelt real-model run was obtained at this new exact source. The attempted Ralph Bridge dispatch profile was not confirmed, and remote read found no run. Do not check WSP-200 enforced offline egress or declare release readiness.

Mac native compile/unsigned-app qualification for the changed source must be assessed separately at its actual workflow SHAs. An unrelated, legacy Moonshine ASR-015 dependency-preparation failure does not count as a Whisper inference regression. Keep Moonshine removal gated and preserve KittenTTS's independent ONNX Runtime.
