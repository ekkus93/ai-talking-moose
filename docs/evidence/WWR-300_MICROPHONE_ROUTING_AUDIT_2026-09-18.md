# WWR-300 microphone routing audit and selected strategy

Date: 2026-09-18
Baseline master: `8cb6d9f6d2c912c6b71109dd1fb354d3f753c52e`

## Current AudioCapture ownership audit

The current production microphone owner is `AppState.audio_capture: Arc<Mutex<AudioCapture>>` in `src-tauri/src/app/state.rs`. It is shared by normal command conversation startup, shutdown, diagnostics, and microphone test paths.

`AudioCapture::start` in `src-tauri/src/audio/capture.rs` always calls `self.stop()` before opening a new stream, resets capture diagnostics, and stores the resulting CPAL stream in a single `_stream` slot. `AudioCapture::stop` clears the running flag and drops the stored stream. This means the current low-level capture object cannot intentionally hold two active CPAL streams at once.

The command conversation path in `src-tauri/src/conversation/session.rs` owns runtime capture activation today. During `ConversationManager::start_session`, it prepares any local ASR pipeline, opens the shared capture exactly once for either local Moonshine ASR or Gemini Live audio, forwards level updates, and closes capture through `begin_shutdown_locked` on stop, failed startup, provider-send failure, or app shutdown.

`test_microphone` in `src-tauri/src/commands/settings.rs` uses `DiagnosticCaptureLease`, which also starts the same shared `AudioCapture` and stops it on drop. It is guarded against active conversations before opening the microphone.

## Selected WWR-300 one-stream strategy

Wake Word V1 must not introduce a second continuous microphone owner. The final production strategy is:

1. Keep `AppState.audio_capture` as the only authoritative low-level microphone owner.
2. Route Wake Word listening, ring-buffer retention, and KWS feed from the same chronological canonical stream that will later be transferred into command ASR.
3. Do not let Wake Word open an independent CPAL stream while command ASR, microphone diagnostics, or Gemini Live audio owns capture.
4. When Wake Word is disabled, preserve current manual listen behavior and do not keep an extra wake stream alive.
5. On a wake trigger, transfer ownership deterministically from wake listening into the existing command interaction path rather than starting a parallel capture path.
6. On command completion, cancellation, recoverable failure, or TTS completion, return ownership to Wake Word only if settings still enable it and lifecycle state permits.
7. Treat device disconnect, unavailable device, permission failure, capture callback error, and cancellation as ownership-state transitions, not as reasons to spin up another stream.

## Implementation implications

The next code slice should add a single Wake Word runtime owner to `AppState`, then wire lifecycle transitions around the existing shared capture boundary. The Wake Word runtime manager should observe settings/lifecycle state and should not own an additional `AudioCapture` instance.

The native KWS session from WWR-200 remains available for the eventual feed path, but WWR-300 should first preserve the one-stream invariant before connecting real microphone frames to the native session.

## Items objectively covered by this evidence

- Re-audited current `AudioCapture` ownership on post-consolidation source.
- Selected and documented the final one-stream/routing strategy.
- Confirmed that current manual conversation and diagnostic paths already serialize through one shared `AudioCapture` slot.

## Items that remain open

- Production `AppState` still needs to own the Wake Word runtime manager.
- Wake Word still needs to be wired into production composition and lifecycle.
- Ring buffer and KWS do not yet share a production microphone stream.
- Wake-to-ASR ownership transfer and return are not yet implemented.
- Device disconnect/reconnect/unavailable-device lifecycle behavior still needs dedicated code and tests.
