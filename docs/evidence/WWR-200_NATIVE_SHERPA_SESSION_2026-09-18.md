# WWR-200 native sherpa KWS session evidence

Date: 2026-09-18
Baseline master: `daf397937c7a09fc4a15e99af725de337d8bc123`

## Implemented production boundary

`src-tauri/src/app/wake_word_engine.rs` now contains the real `NativeKwsSession` and lazily loaded `NativeKwsRuntime` production adapter. Session construction validates the frozen V1 KWS configuration and verifies every required model and native-runtime artifact before native inference is reachable.

The native adapter loads only the pinned sherpa-onnx shared C API and resolves the bounded KWS symbol set. It constructs the keyword spotter with the verified encoder, decoder, joiner, `tokens.txt`, `bpe.model`, and generated keyword file; CPU provider; one inference thread; 16 kHz / 80-dimension features; score 1.0; and threshold 0.25.

Streaming PCM16 is validated as non-empty 16-kHz input before conversion to normalized float samples. The adapter feeds the native keyword stream, decodes while ready, reads only keyword-presence from the result, emits the bounded `WakeWordDetection` shape, and resets the native stream after a detection. It does not expose transcription results or raw PCM in the detection event.

Shutdown is idempotent at the session boundary. Native resources are owned by `NativeKwsRuntime` and destroyed through `Drop`. Native-load and artifact errors are mapped to sanitized Wake Word errors without filesystem paths. Normal inference performs no network operation.

## Deterministic coverage already present

Unit coverage in `wake_word_engine.rs` verifies the frozen policy, artifact contract, invalid PCM ordering, missing/corrupt artifact rejection, architecture rejection, sanitized native-load failure, KWS-only C API contract, shared-C-API rather than JNI loading, frozen native configuration, bounded keyword-result handling, fake-engine feed/reset/shutdown behavior, and idempotent native-session shutdown.

The repository also has `.github/workflows/wwr-kws-c-api-probe.yml`, which validates the pinned native C API surface independently of the Rust unit tests.

## WWR-200 items that must remain open

WWR-200 must not be marked complete yet. Its real positive and negative fixture requirements have not been objectively satisfied on this baseline. Those acceptance requirements depend on WWR-600 (deterministic corpus/harness) and WWR-610/WWR-620 (real Linux x86_64 and macOS arm64 inference acceptance).

The production-composition acceptance also remains open: later integration tasks must prove that application composition instantiates this real native session and never substitutes the fake test engine.

Accordingly, this evidence records the implemented native adapter without converting fixture-dependent or production-composition-dependent checkboxes into false completion claims.
