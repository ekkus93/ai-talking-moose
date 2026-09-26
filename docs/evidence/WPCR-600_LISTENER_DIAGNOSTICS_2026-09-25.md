# WPCR-600 Listener Diagnostics Evidence

**Status:** implementation evidence for listener ownership/status truthfulness.
**Remediation item:** WPCR-600 — diagnostics and Settings UI truthfulness.

## Change summary

This slice separates the Wake runtime phase from physical native-listener ownership in the privacy-safe diagnostics payload. `WakeWordDiagnostics` now reports `listener_status`, `listener_active`, and `listening`, so Settings can distinguish a persisted/enabled runtime from a listener that is actually active on the local microphone.

The Settings panel renders both runtime phase and listener ownership. It exposes stopped, starting, active, pending-until-idle, suspended-for-command, failed-closed, and shutting-down states without raw paths, credentials, audio content, or transcripts.

## Acceptance coverage

- Backend diagnostics tests prove listener ownership is serialized separately from runtime phase.
- Listener-state tests cover startup-without-active-listener and unsupported-ASR fail-closed classification.
- Frontend tests cover stopped, active, pending, and failed-closed Settings states.
- The privacy audit requires the new listener fields while preserving the diagnostic ban on raw PCM, transcripts, credentials, and paths.

## Evidence boundary

This evidence closes the WPCR-600 diagnostics/UI truthfulness boundary only. It does not claim final WPCR-950/960 closeout, real native KWS acceptance, or complete post-closeout remediation by itself.
