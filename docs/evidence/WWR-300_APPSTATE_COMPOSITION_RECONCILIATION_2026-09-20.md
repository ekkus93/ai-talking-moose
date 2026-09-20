# WWR-300 AppState composition reconciliation — 2026-09-20

Baseline: `master` `2afd5b70eac8ce18e4b04c64705e495526b70d53`.

This evidence is deliberately bounded to the two WWR-300 production-composition claims that are objectively present on the baseline. It does **not** claim that continuous microphone→KWS routing or wake→ASR handoff is production-complete.

## Wake Word is wired into authoritative application composition

`src-tauri/src/app/state.rs` contains exactly one Wake Word application owner field, `AppState::wake_word_runtime: WakeWordApplicationRuntime`, adjacent to the authoritative `AppState::audio_capture: Arc<Mutex<AudioCapture>>`.

`AppState::new_with_secret_store` constructs the single `AudioCapture` first, then constructs `WakeWordApplicationRuntime::from_settings(&settings.read())`, and stores both objects in the same returned `AppState`. The Wake owner is therefore part of production application composition rather than a test-only/global side channel.

This supports the WWR-300 item **“Wire Wake Word manager into production application state/composition.”**

## Wake Word composition cannot open a competing microphone stream

`src-tauri/src/app/wake_word_composition.rs` owns only `WakeWordRuntimeManager`. It contains no `AudioCapture`, CPAL device/stream, microphone handle, or capture-open API. Its capture-error boundary explicitly records an error on the runtime without reopening or replacing `AudioCapture`.

The production physical microphone owner remains `AppState::audio_capture`. The Wake application owner can therefore share runtime state without itself opening a second continuous microphone stream.

This supports the structural WWR-300 item **“Ensure Wake Word does not open a competing continuous microphone stream.”** It does not yet prove the final end-to-end acceptance claim “No simultaneous competing capture opens occur,” because production continuous Wake routing is not wired yet.

## Remaining production boundary

`src-tauri/src/app/wake_word_pcm_router.rs` already implements the component-level canonical timeline contract: validate one 16-kHz mono chunk, retain the same chunk in pre-roll, feed that same chunk to KWS, atomically start handoff on accepted detection, preserve post-trigger live samples, transfer handoff once, clear stale handoff state, and reset KWS before returning to listening.

However, current `AppState` does not yet own a production `CanonicalWakePcmRouter`/native KWS session or a capture-consumer task that feeds it. Likewise, the router's `transfer_handoff_to_asr` output is not yet connected to `start_conversation`/the selected command-ASR graph. Therefore the following remain open and must not be inferred from this reconciliation:

- continuous authoritative microphone→Wake router feed;
- native KWS production composition/loading;
- one-trigger→one normal command activation;
- chronological pre-roll/live injection into command ASR;
- deterministic capture ownership transfer/return under real production routing;
- device disconnect/reconnect/error acceptance;
- repeated-cycle no-stream-multiplication acceptance.

## Qualification policy

This is a source reconciliation only. It may be merged after exact-head ordinary CI. It does not satisfy the specialized real-KWS, corpus, lifecycle, performance, or final WWR-950/960 gates.
