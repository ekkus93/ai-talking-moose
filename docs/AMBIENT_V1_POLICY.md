# Ambient behavior policy — V1

## Classifier decision (V1R-074)

V1 deliberately does **not** add a secondary model-based "should Moose speak?" classifier.

The deterministic Rust policy is authoritative. It already evaluates privacy permissions, mute state, active conversation state, the unsolicited-comments toggle, quiet hours, cooldown, annoyance/dismissal state, duplicate-event fingerprints, the hard hourly cap, and event importance before any generation call is made. A second model call would add latency, cost, privacy surface, and another failure mode while providing no safety property that the local gate cannot enforce directly.

If a classifier is revisited later, it must run only after the local allow result, receive separately bounded/minimized context, use a strict typed output schema, and never be able to override a local deny. V1 remains safe when no classifier exists or when model providers are unavailable.

## Safe ambient generation (V1R-075)

Ambient generation is bounded and fail-closed:

- Event summaries are bounded by `MAX_AMBIENT_EVENT_CHARS` before prompt assembly.
- The complete ambient prompt is bounded by `MAX_AMBIENT_PROMPT_CHARS`.
- Memory facts are read only when the persisted memory setting is enabled.
- Application and window-title observations are accepted only when their corresponding privacy settings are enabled; unknown event categories are denied.
- Tool results and transcript history have no ambient `PromptBuilder` input surface.
- The generated ambient text is trimmed and hard-capped locally at 320 Unicode scalar values even if a provider ignores the requested token limit.
- Empty/whitespace-only model output is dropped.
- A provider-generation error or TTS error returns an error and clears transient ambient presentation state. No canned/fake/browser/subprocess fallback remark is fabricated.
- The speech bubble is emitted only after TTS has successfully queued audio, so TTS failure cannot display a remark that was never successfully queued for playback.

## Ambient appearance lifecycle (V1R-076)

For an ambient remark that begins while Moose is hidden/dismissed and is locally allowed:

1. presentation state transitions through `Appearing` to `Idle` and `Thinking`;
2. successful bounded generation/TTS transitions to `Talking`;
3. `Talking` is retained for the calculated queued-audio duration, hard-bounded to 10 seconds;
4. the bubble is cleared and state returns to `Idle`;
5. after the persisted `hide_delay_seconds` interval, Moose returns to `Hidden` if no user action changed the state.

If Moose was already visible and idle before the ambient event, the lifecycle returns to `Idle` and does not hide the user-visible character.

Ambient appearance is state/event driven only. The ambient command path intentionally does not call native window `show`, `set_focus`, `set_always_on_top`, or equivalent APIs, so the ambient lifecycle does not request focus or raise the window.

Explicit conversation start, mute, dismissal, barge-in, and application shutdown interrupt the central scheduler. Dismissal also records the dismissal cooldown/annoyance budget, preventing immediate ambient reappearance.
