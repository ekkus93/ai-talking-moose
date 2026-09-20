# WWR-510 — Privacy-safe Wake Word diagnostics evidence

Date: 2026-09-20
Baseline: `facd54e82a3ee75c0a9ad5812e9f84726edea5a9`

## Source evidence

`src-tauri/src/asr/wake_word_diagnostics.rs` defines the bounded `WakeWordDiagnostics` serialization surface. It exposes enabled/runtime phase, exact model archive and keyword identities, pinned sherpa runtime identity and platform-specific C API hash, platform/architecture, one-thread policy, canonical sample rate/channels, threshold/score, ring capacity/current retained sample count, handoff pre-roll sample count, trigger count, bounded last-trigger age, initialization duration, Talking suspension, and sanitized last error.

The diagnostics type cannot represent raw PCM, transcripts, credentials, or filesystem paths. Unit coverage serializes snapshots and explicitly rejects `pcm`, `transcript`, `credential`, `path`, `model_path`, `runtime_path`, `pcm_samples`, `audio_samples`, and `raw_audio` tokens. Runtime errors are reduced to the fixed sanitized message `The Wake Word runtime encountered an internal error.`

`src-tauri/src/commands/wake_word_diagnostics.rs` reads the authoritative AppState-owned `WakeWordApplicationRuntime` and converts its snapshot to the privacy-safe diagnostics representation. Command-level tests cover disabled/audio-free serialization and sanitized runtime errors.

## WWR-510 coverage

The current source objectively implements all required non-optional WWR-510 diagnostics fields: enabled state, authoritative runtime state, model/runtime identity, platform/architecture, one-thread policy, canonical sample format, ring capacity/duration, threshold/score, trigger count, last-trigger age, initialization duration, Talking suspension, and sanitized last error. Raw PCM is structurally absent from the serialized type.

The optional CPU/memory/inference/handoff timing fields remain optional by specification and are not claimed here. Broader repository-wide log/error audits remain part of WWR-900 and final qualification; this evidence does not claim those audits complete.
