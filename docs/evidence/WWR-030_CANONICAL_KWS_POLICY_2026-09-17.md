# WWR-030 Canonical Wake Word V1 KWS policy evidence

WWR-030 freezes the Wake Word V1 acoustic/runtime policy in one crate-level source of truth: `src-tauri/src/wake_word_policy.rs`.

The canonical policy fixes:

- phrase: `Hey, Moose`;
- keyword source: `HEY MOOSE`;
- sample rate: 16,000 Hz;
- channels: mono (1);
- feature dimension: 80;
- inference threads: 1;
- keyword score: 1.0;
- threshold: 0.25;
- pre-roll: 2 seconds / 32,000 samples.

`app::wake_word_settings`, `app::wake_word_engine`, `audio::pcm_ring_buffer`, and `asr::wake_word_diagnostics` consume or alias those canonical values rather than maintaining independent policy literals. The authoritative Wake Word facade exposes the policy alongside the selected runtime/engine components.

The engine's V1 configuration validation fails closed on sample-rate, channel-count, feature-dimension, thread-count, keyword, score, or threshold drift. Regression tests explicitly exercise threshold, score, thread, channel, and non-canonical PCM failures.

The authoritative engine also exposes the exact model input contract used by the manifest: encoder ONNX, decoder ONNX, joiner ONNX, `tokens.txt`, and `bpe.model`. Architecture tests compare that engine contract with `SHERPA_KWS_REQUIRED_FILES`, making schema drift executable rather than documentary.

Production model identities remain deliberately fail-closed and are not claimed by WWR-030; exact upstream bytes, hashes, provenance, and license closure belong to WWR-100.

Exact-head ordinary CI on `6efd987429053ff10d06d04776d9ad6aed9d3ba5` passed as run `35280465472`. A separately triggered KittenTTS acceptance run had an unrelated Linux one-thread sweep inference failure while normal Linux inference and the full macOS acceptance passed; WWR-030 does not change Local TTS code or policy. This evidence commit intentionally creates a fresh exact head so repository-required checks can run again without bypassing the merge gate.
