# WWR-510 — Privacy-safe Wake Word diagnostics evidence

Evidence head reviewed: `ed2a88c0811b6f650ebfa96f53378666c281c591`.

`src-tauri/src/asr/wake_word_diagnostics.rs` defines the serialized `WakeWordDiagnostics` boundary. It exposes bounded configuration, identity, lifecycle, timing, and counter data rather than audio content.

## Implemented fields

The production diagnostics include:

- enabled state and authoritative runtime phase;
- engine/model identity, model archive SHA-256, model license, and keyword SHA-256;
- runtime identity/license and platform-specific C-API SHA-256;
- platform and architecture;
- canonical sample rate/channels and one-thread policy;
- fixed threshold and score;
- ring-buffer capacity in samples/milliseconds and current retained sample count;
- handoff pre-roll retained sample count;
- trigger count and bounded last-trigger age;
- runtime initialization duration;
- Talking suspension state;
- sanitized last error.

The WWR-510 CPU/memory/inference/handoff timing item is explicitly optional in the remediation TODO (`as available`) and remains part of the separate WWR-630 measured-performance work rather than a prerequisite for privacy-safe diagnostics completion.

## Privacy and sanitization evidence

The diagnostics type cannot represent raw PCM, transcripts, credentials, or filesystem paths. Rust tests serialize representative disabled, triggered, suspended, and error diagnostics and assert that raw-audio/path/transcript markers are absent. Runtime errors exposed through diagnostics collapse to the sanitized message `The Wake Word runtime encountered an internal error.`

The specialized `Wake Word privacy audit` workflow additionally scans the diagnostics/source boundary and sanitizer/documentation evidence. On master `01a45ce95720eac25f7fa8f22b788eaadba1786a`, privacy audit run `35487357710` passed; ordinary CI run `35487357684` and documentation audit run `35487357681` also passed.

## Acceptance boundary

This evidence supports WWR-510 diagnostics completion: lifecycle/artifact state can be inspected without exposing audio or secrets. It does not claim WWR-630 measured performance acceptance or WWR-900 final whole-source audit completion.
