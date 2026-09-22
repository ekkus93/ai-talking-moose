# WWR-900 source/privacy/security audit evidence — 2026-09-22

## Scope

This evidence records source-level Wake Word V1 privacy/security invariants enforced on current `master`. It does not claim final feature acceptance while real corpus/platform/performance/integrated-production gates remain open.

## Enforced source invariants

`scripts/check_wake_word_source_security_audit.mjs` and `.github/workflows/wake-word-source-security-audit.yml` fail closed on regressions to the following boundaries:

- exactly one `WakeWordRuntimeManager` definition;
- one authoritative `AppState::audio_capture` owner and one `WakeWordApplicationRuntime` owner;
- Wake capture composition routes through the shared capture owner and cannot construct a competing production `AudioCapture`;
- command handoff stops the authoritative capture owner before command-ASR ownership;
- repeated positive KWS frames cannot duplicate command activation, while a later phrase after resume can trigger again without a timer cooldown;
- disabled Wake does not feed KWS;
- Talking suspension, latest-setting terminal resolution, capture-error fail-closed handling, and shutdown boundaries remain present;
- `activate_wake_command_once` uses a single-use handoff, suspends Wake before command ASR, and has startup-failure recovery coverage;
- command ASR ingress remains provider-neutral and uses the normal `LocalAsrPipeline` rather than a cloud/network bypass;
- retained Wake PCM runtime/router/handoff paths contain no filesystem create/write API, preserving the memory-only retention policy;
- model and native-runtime identity verification and Linux x86_64/macOS arm64 architecture checks remain present;
- production KWS source contains no HTTP/WebSocket client dependency or Google/Gemini/OpenAI/full-transcription reference.

Separate privacy audit gates already cover diagnostics/error/log leakage for raw audio, credentials, and unnecessary absolute paths.

## Exact validation

PR #389 strengthened command activation/ASR-ingress audit coverage. Exact PR head `1feb49ad5735c9215653577fda7b6a71d5102bb6` passed ordinary CI `35779956547` and Wake Word source security audit `35779956632`. The merged master `a38f02788a9eb3d84d8a5575de8e25252c792d87` passed ordinary CI `35780153086` and source security audit `35780153046`.

PR #390 added the memory-only retained-PCM filesystem guard. Exact PR head `bfaa3f980c51895031be8d0decd2258001f340f4` passed ordinary CI `35780350090` and source security audit `35780350091`. The merged master `928b1d53549995552bb93517bbd639d6a79d5ef9` passed ordinary CI `35780463853` and source security audit `35780463857`.

## Remaining non-claims

This source audit does not establish that the durable native production listener is fully wired, that real positive/negative KWS fixtures pass on Linux and macOS, that integrated wake→ASR→Thinking→Talking→wake soak passes, or that performance acceptance passes. Those remain WWR-300/400/600/610/620/630/640 work and prevent WWR-900 final acceptance from being marked complete.

Final mandatory-review closure also remains open until those production acceptance tasks and WWR-910/950/960 reconciliation are complete.
