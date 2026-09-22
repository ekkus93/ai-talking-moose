# WWR-900 — Wake Word source/security audit evidence

**Date:** 2026-09-21
**Scope:** Add a deterministic source gate for Wake Word ownership, lifecycle, artifact, architecture, and offline/provider-separation invariants.

## Gate

`node scripts/check_wake_word_source_security_audit.mjs` fails closed if the production source loses the single `WakeWordRuntimeManager`, the shared `AppState::audio_capture` ownership boundary, command-handoff capture stop, lifecycle suspension/reset boundaries, exact model/runtime verification calls, target native architecture checks, or the local KWS engine boundary.

The audit also rejects direct network-client/URL references and cloud/full-transcription provider references in the production Wake KWS engine source. This guards the V1 requirement that idle keyword spotting is local/offline and does not silently become full-time cloud ASR.

## Boundary

This source gate is not final WWR-900 acceptance by itself. Production microphone/handoff integration, real positive/negative KWS fixtures, platform acceptance, performance evidence, and final exact-head review remain separate requirements. The gate is intended to keep already-established source/security invariants from regressing while those remaining slices are completed.
