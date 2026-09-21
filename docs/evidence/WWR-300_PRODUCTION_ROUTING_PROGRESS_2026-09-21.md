# WWR-300 / WWR-310 production routing progress

Date: 2026-09-21
Baseline: `d7f8bea6c018588d22720c1bbc104ae9984e2263`

## Objective source evidence now present

The post-audit implementation has advanced materially beyond the original WWR-300 ownership audit.

- `AppState` owns exactly one `Arc<Mutex<AudioCapture>>` and one `WakeWordApplicationRuntime`.
- `WakeWordApplicationRuntime` owns no capture device. It shares the one authoritative `WakeWordRuntimeManager` state across clones.
- `start_authoritative_wake_capture` starts Wake listening through the existing `AudioCapture` owner at the canonical 16 kHz target rate. `AudioCapture::start` first stops/replaces the stream held by that same owner; no Wake-specific `AudioCapture` exists.
- `WakeCapturePcmConsumer` only decodes PCM16-LE emitted by `AudioCapture`; it does not resample or open hardware.
- `CanonicalWakePcmRouter` validates one canonical frame and routes the exact same sample slice, serially, to the pre-roll ring and KWS engine.
- On accepted detection the runtime atomically snapshots chronological pre-roll and enters `Triggered`. The router then retains subsequent live canonical chunks without re-feeding KWS.
- The wake handoff transfer is single-use and preserves pre-roll followed by post-trigger live samples in exact order.
- `WakeCommandAsrHandoff` adds a second single-use activation guard so one accepted wake payload cannot prime command ASR twice.
- `LocalAsrPipeline` implements `WakeCommandAsrIngress`, so wake handoff audio primes the existing bounded Moonshine command-ASR ingress instead of creating another ASR pipeline or microphone path.
- Command-interaction lifecycle helpers suspend Wake before command ASR/TTS ownership and clear retained Wake audio. Terminal interaction resolution honors the latest enabled setting.
- Router return-to-listening clears stale handoff state and resets KWS before accepting later wake PCM.

## Deterministic coverage already on master

Current unit coverage proves the following component-level invariants without microphone hardware:

- restarting Wake capture replaces the stream on the same `AudioCapture` owner;
- capture bytes are decoded and routed chronologically;
- invalid capture bytes do not mutate Wake state;
- ring retention and KWS receive the same canonical chunks in order;
- trigger + post-trigger live chunks produce one ordered command payload;
- handoff transfer is single-use;
- invalid post-trigger input does not mutate live handoff state;
- failed command-ASR startup consumes stale handoff and returns Wake to a recoverable state;
- repeated activation attempts do not duplicate command-ASR ingress;
- return to Wake resets KWS and permits a later phrase.

## Remaining production gap

This evidence does **not** close WWR-300 or WWR-310. The remaining mandatory gap is application orchestration: startup must construct the verified native KWS session, start/consume the authoritative capture stream when Wake is enabled and lifecycle permits, invoke the existing command interaction exactly once on detection, and keep the same capture timeline attached through command ASR without reopening a competing stream. Device disconnect/reconnect and cancellation must be exercised through that integrated owner.

The TODO checkboxes should remain conservative until that orchestration is merged and exact-head qualified. This file exists to prevent already-implemented routing/handoff invariants from being rediscovered or accidentally reimplemented as a second stack.
