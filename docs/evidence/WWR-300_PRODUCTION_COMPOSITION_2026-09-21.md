# WWR-300 production composition evidence

Date: 2026-09-21
Audited base: `a2ffb738d60d8cb3ae254d54a590ffd4c2fabe8a`

## Scope

This evidence narrows WWR-300 to the still-missing audio-routing work. It verifies two implementation facts that are already present on production `master`: the authoritative Wake Word manager is wired into application composition, and that owner does not create a competing microphone capture stream.

## Production wiring

`src-tauri/src/app/state.rs` has exactly one application-level Wake Word owner field, `AppState::wake_word_runtime: WakeWordApplicationRuntime`, alongside the existing authoritative `AppState::audio_capture: Arc<Mutex<AudioCapture>>`.

`AppState::new_with_secret_store` constructs the single `AudioCapture` owner and separately constructs `WakeWordApplicationRuntime::from_settings(&settings.read())`. The Wake Word owner is therefore initialized from the normalized persisted settings and participates in normal production `AppState` composition.

`src-tauri/src/app/wake_word_composition.rs` owns only `WakeWordRuntimeManager`; it has no `AudioCapture`, device name, stream handle, CPAL host/device, or capture-open operation. Its capture-error boundary explicitly fails Wake Word closed and does not reopen or replace `AudioCapture`.

The composition clone regression proves cloned application Wake owners share the same underlying manager state rather than creating an independent manager.

## Existing production command boundary

`src-tauri/src/commands/conversation/core.rs` passes `state.audio_capture.clone()` into the existing normal `ConversationStartRequest`. Wake lifecycle guarding uses `state.wake_word_runtime.clone()` independently and does not construct another capture object. This preserves the one authoritative capture owner while command interaction lifecycle work proceeds.

## Checklist evidence

The source above objectively supports these WWR-300 items:

- Wire Wake Word manager into production application state/composition.
- Ensure Wake Word does not open a competing continuous microphone stream.

It does **not** claim completion of the remaining WWR-300 routing items. In particular, canonical microphone PCM is not yet shown here feeding both ring buffer and native KWS through one production route, and this evidence does not claim device reconnect/permission acceptance or wake-to-command ownership transfer is complete.

## Next implementation boundary

The next WWR-300 implementation must route the one existing capture owner's canonical PCM into the Wake runtime/KWS path without introducing a second capture open. That routing must then supply the post-trigger live handoff used by WWR-310 and retain deterministic ownership on device/cancellation errors.
