# WWR-300 AppState capture composition — 2026-09-22

## Scope

This evidence records the production boundary that composes Wake Word routing around the authoritative application microphone owner.

## Source evidence

`capture_owner_from_app_state` constructs `AuthoritativeWakeCaptureOwner` from `AppState::audio_capture.clone()`. The helper does not allocate or hide a second `AudioCapture`; Wake routing must use the same capture owner that manual command listening uses.

`runtime_from_app_state` continues to expose the single `WakeWordApplicationRuntime` stored directly in `AppState`. The Wake runtime and Wake capture owner are therefore composed from authoritative application state rather than independent Wake-owned globals.

## Regression coverage

`capture_owner_from_app_state_uses_exact_app_capture` starts Wake capture through the helper against a mock `AppState::audio_capture`, verifies that the same application capture owner becomes active at the V1 sample rate, then disables Wake and verifies that the same capture owner stops and the shared Wake runtime transitions to `Disabled`.

## Non-claims

This does not complete real microphone-device acceptance, device disconnect/reconnect handling, real KWS fixture acceptance, or wake→ASR command handoff acceptance. It narrows the production composition boundary so those later acceptance tests exercise one shared capture owner instead of an independent Wake-owned microphone path.
