# Moonshine Local-ASR Regression Suite

Recorded: 2026-08-21

This document maps V1R-ASR-017 to deterministic tests that run without model downloads, microphone hardware, or live Google traffic. Real native-model acceptance remains a separate supported-macOS exercise.

## Coverage matrix

| ASR-017 requirement | Deterministic coverage |
| --- | --- |
| Fake model-manager tests | `asr/moonshine/installer_tests.rs` uses `FakeTransport` and fake disk probes for atomic install, repair, integrity failure, cancellation, concurrency, deletion, and progress. |
| Fake native ABI/wrapper tests | `asr/moonshine/runtime.rs` uses `FakeApi` to exercise runtime compatibility, typed native errors, stream ownership/drop order, and Rust-owned transcript conversion without linking the native library. |
| Partial/final transcript tests | `asr/transcript_state.rs`, `asr/moonshine/engine_tests.rs`, and `asr/pipeline_tests.rs` cover replacement, duplicate suppression, final-once semantics, speech lifecycle, stale updates, and provider-neutral worker emission. |
| Provider switching tests | `conversation/session/tests.rs` proves a replacement provider tears down prior local ASR before connecting and stale callbacks cannot cross generations; local-final routing is also serialized against Start/Stop boundaries. |
| Missing/corrupt/incompatible model tests | Session tests fail missing/corrupt installs before provider connection or microphone startup; manifest/runtime tests reject header/runtime incompatibility; engine tests reject missing/corrupt/native-load failures without fallback. |
| Queue overload/cancellation tests | `asr/pipeline_tests.rs` covers hard queue bounds, drop-newest behavior, queue-depth recovery, cancellation, stop/join, and queued-audio discard; engine/installer tests cover cancellation idempotence and partial cleanup. |
| No cloud microphone upload in local mode | `conversation/session/event_loop/asr_handoff_tests/privacy.rs::moonshine_mode_never_calls_provider_audio_upload_api` asserts Tiny and Small local microphone frames cause zero provider audio-upload calls while Gemini cloud-audio mode does call the provider API. |
| Ordinary tests stay offline/hardware independent | Installer tests inject fake transport/filesystem dependencies; pipeline tests use fake engines and mock capture; native wrapper tests inject `FakeApi`; ASR-015 real native CPU benchmarks are `#[ignore]` and require explicit environment opt-in. |

## Privacy invariant

For `MoonshineTinyStreaming` and `MoonshineSmallStreaming`, `ConversationManager::forward_microphone_chunk` returns without invoking the provider audio API. Only `GeminiLiveAudio` may call `LiveSession::send_audio_chunk`. Final local transcripts cross the provider boundary as text turns after generation, session-ID, lifecycle, and active-mode checks.

Local-ASR startup failures are fail-closed: model verification occurs before provider connection and microphone capture. Missing or corrupt local models therefore leave capture stopped and report that no microphone audio was sent to Google.

## Barge-in invariant

Barge-in flushes stale response playback and interrupts the provider response, but it does not tear down the current local recognizer. The local recognizer remains generation-bound and can continue receiving the next user turn. Full Stop, Mute, Dismiss, provider loss, application shutdown, or provider/mode replacement use the centralized teardown path and stop local ASR.

## Deliberately non-ordinary acceptance

The following are not ordinary regression tests and remain open until explicitly run on supported macOS hardware:

- real Tiny native model load/stream/transcription;
- real Small native model load/stream/transcription;
- ASR-015 representative Tiny/Small CPU benchmark measurements and minimum-supported-CPU decision.

The packaged native runtime load itself is covered separately by the ASR-016 macOS bundle smoke jobs for arm64 and x86_64.
