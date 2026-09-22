# WWR-300 capture disconnect/reconnect evidence — 2026-09-22

## Scope

This evidence records deterministic source and regression coverage for Wake Word microphone disconnect/reconnect handling through the single authoritative `AppState::audio_capture` owner. It does not claim real hardware hot-unplug acceptance.

## Disconnect behavior

`src-tauri/src/app/wake_word_capture_orchestrator.rs` treats closure of the authoritative capture queue as a terminal event for the current listening epoch. `route_next` clears pending handoff audio, moves the Wake runtime to `Error`, and returns `CaptureClosed`; it does not open a replacement stream or spin in a retry loop.

The regression `closed_capture_queue_fails_wake_runtime_closed_without_reopening_capture` proves a closed queue leaves the runtime in `Error` with zero retained ring-buffer and handoff samples.

## Reconnect behavior

`restart_after_capture_error` reopens capture only through the same supplied `AudioCapture` owner. Before returning to Listening it clears stale handoff state, resets the KWS stream, and performs the explicit Loading -> Listening runtime transition. Failed capture open or KWS/runtime reset remains fail-closed; an opened stream is stopped again if reset/re-entry fails.

`AuthoritativeWakeCaptureOwner::restart_wake` serializes that reconnect operation around the same `Arc<Mutex<AudioCapture>>` held by `AppState`.

The regressions `reconnect_reuses_same_capture_owner_and_returns_error_runtime_to_listening` and `failed_command_return_leaves_capture_stopped_and_wake_recoverable` cover successful recovery after a closed/error epoch and prove recovery does not introduce another capture owner.

## Validation baseline

Exact merged-master ordinary CI for `e2cd3ef9100749219e738ed1e4585aa25f70a1f0` passed in run `35767670958`.

## Non-claims

This evidence supports deterministic disconnect/reconnect semantics and unit-level acceptance only. Real device hot-unplug/replug acceptance and repeated integrated wake→ASR→Thinking→Talking→wake soak remain part of WWR-640/final acceptance and are not inferred here.
