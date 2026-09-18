# WWR-510 — Privacy-Safe Wake Word Diagnostics Evidence

**Task:** WWR-510 — Complete privacy-safe diagnostics
**Merged implementation:** PR #205
**Merged `master` SHA:** `f180f3f2b9a73ffe143f6ff8bf95d86623f65cc9`
**Exact PR-head SHA:** `92248560749a1cc8f181aefd47dbd15e366b379e`
**Exact PR-head ordinary CI:** `35400266215`

## Implemented diagnostic surface

`src-tauri/src/asr/wake_word_diagnostics.rs` exposes a bounded `WakeWordDiagnostics` structure containing only lifecycle state, immutable identity metadata, fixed V1 policy values, bounded counters, and sanitized errors:

- enabled state;
- authoritative runtime phase;
- engine ID;
- model ID, model archive SHA-256, model license, and keyword SHA-256;
- sherpa runtime ID, runtime license, and platform C API library SHA-256 when the current platform is supported;
- OS and architecture;
- canonical sample rate and channel count;
- inference thread count;
- threshold and score;
- ring-buffer capacity in samples and milliseconds;
- current ring sample count and handoff pre-roll sample count;
- trigger count;
- last-trigger age in milliseconds;
- runtime initialization duration in milliseconds;
- Talking suspension state;
- sanitized last error.

## Privacy and safety properties

The diagnostics type intentionally cannot represent raw PCM buffers, transcripts, credentials, or filesystem paths. The merged tests serialize diagnostics and assert that PCM, transcript, credential, path, and raw-audio field names are absent.

`diagnostic_identity_constants_track_manifest_json` also binds diagnostic identity constants to `wake-word-artifacts.json`, preventing silent drift between diagnostics and the authoritative manifest.

## Scope note

WWR-510 asks for optional measured CPU/memory/inference/handoff timing fields as available. No CPU, memory, inference-latency, or handoff-latency measurements are available yet; those are explicitly covered by WWR-630. WWR-510 is complete because diagnostics do not fabricate unavailable measurements, while the available initialization timing is exposed.

## Deferred items still open elsewhere

This evidence does not close WWR-630 performance evidence, WWR-610/620 real-platform inference acceptance, WWR-640 lifecycle stability acceptance, or WWR-900 final audit.
