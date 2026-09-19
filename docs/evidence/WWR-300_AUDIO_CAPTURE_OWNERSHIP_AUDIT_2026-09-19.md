# WWR-300 — Audio capture ownership audit

Date: 2026-09-19
Baseline: `e3bcfd64f272c6916f5aae751d7463710b346dbb`

## Current production ownership

`AppState::audio_capture` is the existing application-owned `Arc<Mutex<AudioCapture>>` used by normal conversation startup. `AudioCapture::start` first stops any prior stream, then opens one CPAL input stream and emits bounded 16-bit mono PCM chunks at the caller-selected target sample rate. The capture processor performs channel downmix and sample-rate conversion before enqueueing PCM.

The Wake Word application runtime deliberately owns no microphone stream. `WakeWordApplicationRuntime` owns only lifecycle/runtime state; its composition documentation explicitly reserves microphone ownership to `AppState::audio_capture` for WWR-300 routing. This is the correct invariant and must remain true.

The current conversation path passes the same `AppState::audio_capture` object into `ConversationStartRequest`. Therefore adding a second Wake Word-specific `AudioCapture` or directly opening CPAL from the KWS engine would create competing ownership and is prohibited.

## Final V1 routing strategy

Use exactly one application-level capture stream rooted at `AppState::audio_capture`. While Wake Word is enabled and lifecycle permits listening, that stream produces the canonical 16 kHz mono PCM timeline. A routing layer above `AudioCapture` must fan each canonical chunk, in chronological order, to the Wake Word ring buffer and native KWS session from the same chunk. It must not resample independently for those two consumers.

On accepted wake detection, the router must atomically change logical ownership rather than opening another microphone stream: freeze/take the chronological pre-roll snapshot, begin retaining subsequent live canonical chunks, start the existing command interaction, replay pre-roll, then drain live retained chunks and continue forwarding the same capture timeline to command ASR. The boundary must prove no gap, duplicate range, or inversion.

During command ASR, Thinking, and Talking, Wake Word KWS must not consume microphone chunks. Talking entry clears retained Wake Word audio. When the interaction terminates, the same capture owner returns to Wake Word listening only if the latest setting remains enabled; otherwise it remains disabled. Cancellation and recoverable errors must follow the same ownership-return rule.

## Error and device policy

A CPAL runtime stream failure already marks `AudioCapture` inactive and records a bounded diagnostic error. WWR-300 integration must propagate that single-owner failure into Wake Word lifecycle state without opening retry streams in parallel. Reconnect or device changes must serialize replacement of the sole `AudioCapture` stream. Unavailable-device and permission failures must leave deterministic ownership and preserve manual interaction behavior.

## Prohibited designs

- A dedicated continuous CPAL stream inside `NativeKwsSession` or Wake Word runtime.
- Simultaneous Wake Word and command-ASR capture opens.
- Separate resampling pipelines for Wake Word ring retention and KWS inference.
- Restarting capture at the wake→ASR boundary when the existing canonical stream can be routed instead.
- Resuming KWS before command/TTS ownership has terminally released the interaction.

## Implementation consequences

The next WWR-300/310 production slice should introduce a bounded canonical PCM router around the existing `AppState::audio_capture` owner, not another capture implementation. Tests must count physical capture starts/active streams across repeated wake→ASR→wake cycles and assert the count never grows beyond one active stream. The handoff tests must label synthetic sample ranges so continuity, non-duplication, and ordering are objective.

This audit closes only the WWR-300 re-audit and strategy-selection work. It does **not** claim that production Wake Word microphone routing, native KWS composition, or wake→ASR handoff is already complete.