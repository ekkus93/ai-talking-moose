# WWR-200 production native capture consumer — 2026-09-22

## Scope

This evidence records the production-composition boundary added after the real native sherpa KWS session implementation.

## Source evidence

`WakeWordApplicationRuntime::native_capture_consumer` constructs a `NativeKwsSession` from explicit verified model/runtime roots before returning the capture consumer used by the authoritative Wake Word runtime owner.

The constructor is intentionally fail-closed: `NativeKwsSession::new` verifies the frozen KWS policy, model artifacts, runtime artifacts, and platform/runtime identities before a capture consumer can be built. Missing or corrupt artifacts therefore fail before microphone capture can start through this production constructor.

The deterministic fake-session seam remains available through `WakeWordApplicationRuntime::capture_consumer(engine)` for unit tests and routed-PCM verification, but production composition now has a distinct real-session constructor instead of requiring callers to inject a fake or arbitrary engine.

## Regression coverage

`native_capture_consumer_uses_real_session_and_fails_closed_without_verified_artifacts` proves the production constructor attempts to build the real verified native session and returns the sanitized missing-artifact error without mutating an already-listening runtime when the required artifacts are absent.

## Non-claims

This does not complete real positive/negative fixture acceptance. It also does not complete Linux x86_64 or macOS arm64 real KWS acceptance, production microphone ownership wiring, or wake→ASR handoff acceptance.
