# WSP-400 — Whisper Small missing-OpenAI-key TTS isolation

**Recorded:** 2026-10-17 UTC
**Source SHA:** `d19e2ca4432591c04fdb6c8665bb3a7c93017b8f`
**Exact-head CI:** [Main CI 39045338764](https://github.com/ekkus93/ai-talking-moose/actions/runs/39045338764), **completed success**.

## What was qualified

- The existing `missing_openai_key_in_non_live_talk_does_not_fall_back` regression in `src-tauri/src/app/state/tests/talk_routing.rs` now checks both `TtsProvider::Local` and `TtsProvider::Google` with `AsrMode::WhisperSmall` and selected `TextProvider::Openai`.
- The non-Live conversation connects without silently invoking Gemini Live; with no configured OpenAI key, a text turn returns a typed `Auth` error. The error must not disclose the turn's test text.
- The event channel is checked for no subsequent emitted event, particularly no `AudioData` or `TurnComplete`, regardless of the selected standalone TTS.
- The source change is test-only; it does not change production routing, Moonshine, KittenTTS/ONNX Runtime, model installation, or release packaging.

## Verification

Main CI at exactly `d19e2ca4432591c04fdb6c8665bb3a7c93017b8f`: run `39045338764`. `Rust tests`, `Rust quality` (including Clippy), `Generated backend contract`, and `Classify CI scope` all concluded **success**. Frontend, macOS packaging, and canonical `npm run check:all` jobs were skipped in this scope and must not be counted as successes.

## Explicit limitations

This is an automated negative-path/fake-provider regression. It is **not** evidence for a real OpenAI API call, actual Whisper weight-backed microphone inference, genuine TTS synthesis/playback, macOS System Voices audition, enforced offline egress, a signed application, or full WSP-400/600/700 acceptance. The authoritative TODO's incomplete checkboxes remain open. Retain the Moonshine cutover safety gate and KittenTTS's independent ONNX Runtime.
