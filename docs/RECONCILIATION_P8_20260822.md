# Phase P8 reconciliation — 2026-08-22

This overlay reconciles the legacy monolithic TODO without rewriting the large tracker file through the GitHub contents API.

## Current closure candidate: V1R-080 through V1R-086

These rows are **implemented in the source candidate but pending GitHub Actions acceptance**. Do not treat P8 as accepted complete until the implementation commit is reported green.

### V1R-080 — Observer result model

- [x] Common typed `available/denied/unavailable/unsupported/error` observer contract.
- [x] Placeholder production values are removed; unsupported/unavailable/error sources fail closed.
- [x] Diagnostics contain only observer kind/status/fixed error code and never observation values.

### V1R-081 — Real idle time

- [x] macOS idle duration uses CoreGraphics rather than a constant.
- [x] Non-macOS builds return `unsupported`; invalid macOS values return a typed error.
- [x] Conversion/minimization tests cover the five-minute bucket and 24-hour retention bound.

### V1R-082 — Sleep/wake events

- [x] macOS sleep/wake uses `IORegisterForSystemPower` with required power-change acknowledgements.
- [x] A single desktop runtime owns the observer and routes power events through the P7 ambient scheduler.
- [x] Shutdown cancels the desktop runtime before stopping the ambient scheduler and tears down the IOKit registration.
- [x] Wake submits only a local ambient event and never opens microphone capture.

### V1R-083 — Battery/power state

- [x] macOS battery state uses IOPowerSources with type-checked capacity/charging values.
- [x] Machines without an internal battery return `unavailable`; other platforms return `unsupported`.
- [x] Fabricated `85% charging` placeholder behavior is removed from both ambient observation and the built-in battery tool.
- [x] Deterministic summarizer tests cover threshold behavior.

### V1R-084 — Active application

- [x] macOS active application identity uses AppKit `NSWorkspace`.
- [x] The OS query is skipped entirely when active-app observation is not opted in.
- [x] Observer diagnostics never contain the application identity.
- [x] The built-in active-application tool uses the same opt-in-aware observer contract and never fabricates `Unknown`.
- [x] Only `available` values reach the summarizer; P7 re-checks the privacy setting before generation and delivery.
- [x] Denied/unavailable/unsupported/error states fail closed.

### V1R-085 — Window-title decision

- [x] V1 explicitly does not implement window-title observation.
- [x] The backend capability reports `unsupported` even if a legacy settings field is true.
- [x] P7 hard-denies `WindowTitle`, so no stale compatibility setting can expose title data.

### V1R-086 — Local event summarizer

- [x] Only typed `available` observations are accepted by the runtime bridge.
- [x] Application identity is bounded; historical switch summaries retain counts/timestamps rather than names.
- [x] Last-application state is retained as a SHA-256 fingerprint rather than raw identity.
- [x] Idle/battery/app-switch retention and event frequency are explicitly bounded.
- [x] Summarized events enter the existing P7 scheduler, inheriting dedup/cooldown/annoyance/privacy policy.

See `docs/DESKTOP_OBSERVATION_V1.md` for the V1 source/permission/retention contract.
