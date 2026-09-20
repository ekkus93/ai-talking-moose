# WWR-510 — Privacy-safe Wake Word diagnostics evidence

Evidence head reviewed: `ed2a88c0811b6f650ebfa96f53378666c281c591`.

## Authoritative diagnostics surface

`src-tauri/src/asr/wake_word_diagnostics.rs` defines `WakeWordDiagnostics` as the privacy-safe serialized runtime diagnostics surface. The type intentionally exposes bounded counters, immutable artifact identities, fixed configuration, and lifecycle state only.

The diagnostics surface exposes the WWR-510 required fields:

- Wake Word enabled state: `enabled`.
- Authoritative runtime state: `runtime_phase`.
- Exact model identity: `engine_id`, `model_id`, `model_archive_sha256`, `model_license`, and `keyword_sha256`.
- Exact runtime identity: `runtime_id`, `runtime_license`, and platform-specific `runtime_c_api_sha256`.
- Platform and architecture: `platform` and `architecture`.
- One-thread policy: `inference_threads`.
- Canonical sample policy: `canonical_sample_rate_hz` and `canonical_channels`.
- Ring duration/capacity: `ring_buffer_capacity_samples`, `ring_buffer_capacity_ms`, `ring_buffer_samples`, and `handoff_pre_roll_samples`.
- Threshold and score: `threshold` and `score`.
- Trigger count and last-trigger age: `trigger_count` and `last_trigger_age_ms`.
- Initialization duration: `runtime_initialization_ms`.
- Talking suspension: `talking_suspended`.
- Sanitized last error: `last_error`.

Optional CPU/memory/inference/handoff timing metrics are not yet represented beyond `runtime_initialization_ms`, so the optional measured timing task remains open until measured performance evidence is added.

## Privacy and serialization bounds

The diagnostics type documentation states that raw PCM, transcripts, credentials, and filesystem paths are not representable in `WakeWordDiagnostics`.

Focused Rust diagnostics tests verify:

- disabled diagnostics are fail-closed and serialize without `pcm`, `transcript`, `credential`, or `path` substrings;
- model/runtime/keyword identities match the manifest rather than drift silently;
- initialization duration is observable without serializing model/runtime paths;
- trigger count, last-trigger age, and Talking suspension are observable without raw audio content;
- Talking suspension clears retained ring/pre-roll samples;
- runtime errors expose only the fixed sanitized message `The Wake Word runtime encountered an internal error.`

The dedicated Wake Word privacy audit workflow additionally checks that:

- `WakeWordDiagnostics` does not contain raw PCM/transcript/credential/path-like serialized fields;
- the required privacy-safe fields remain present;
- engine-side sanitizer evidence for path and token-like redaction remains present;
- documentation continues to state that diagnostics do not expose raw PCM, transcripts, credentials, or private audio content;
- the corpus manifest continues to forbid private room audio and remains pending real fixture calibration.

## Qualification evidence

Current merged master `ed2a88c0811b6f650ebfa96f53378666c281c591` passed ordinary CI run `35487447538`.

The latest exact-master Wake Word privacy audit evidence before this review was run `35487357710` on `01a45ce95720eac25f7fa8f22b788eaadba1786a`, and the diagnostics source reviewed here is unchanged in the WWR-500 evidence-only merge to `ed2a88c0811b6f650ebfa96f53378666c281c591`.

This evidence closes the concrete WWR-510 diagnostics field/privacy requirements except optional measured CPU/memory/inference/handoff timing fields, which remain dependent on WWR-630 performance evidence.
