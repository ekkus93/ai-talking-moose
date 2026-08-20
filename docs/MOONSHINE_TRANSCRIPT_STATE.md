# Moonshine Transcript State Machine

Status: **V1R-ASR-010 implementation contract**  
Recorded: 2026-08-20

## Purpose

The Moonshine native runtime exposes mutable streaming transcript lines. The application must not infer utterance identity from text because partial text changes over time. The Tiny engine therefore preserves Moonshine's stable native line ID, and the ASR-010 state machine maps that ID to a provider-neutral utterance lifecycle.

The bounded ASR pipeline exposes only `AsrEvent` above this adapter boundary:

- `SpeechStarted`;
- `PartialTranscript`;
- `FinalTranscript`;
- `SpeechEnded`;
- typed `Error`.

Moonshine-specific transcript-update types do not cross the pipeline callback boundary.

## Stable accumulator

`TranscriptStateMachine` owns at most one active utterance and a set of segment IDs that have already been closed. For the active segment it retains the latest partial text.

For one stable segment ID:

1. the first non-empty partial emits `SpeechStarted` followed by `PartialTranscript`;
2. a changed partial replaces the prior partial and emits one new `PartialTranscript` event;
3. an unchanged partial is suppressed;
4. the first final emits `FinalTranscript` followed by `SpeechEnded`;
5. the segment is then permanently closed for that state-machine instance;
6. duplicate finals, changed-after-final updates, and late partials for that segment are suppressed.

A final that arrives without a preceding partial still receives a complete provider-neutral lifecycle: `SpeechStarted`, `FinalTranscript`, `SpeechEnded`.

## Segment changes before final

Only one utterance may be active. If a new stable segment ID appears while an older segment has only partial text, the old segment is closed with `SpeechEnded` **without fabricating a final transcript**. The new segment then starts normally.

The abandoned segment ID remains closed, so a stale later final cannot resurrect it after a newer utterance has started. This is intentionally fail-closed: incomplete local text is not silently converted into a user turn.

## Empty text

Whitespace-only partial or final updates are ignored. They do not create speech lifecycle events and are never retained.

## Retention contract

Only `AsrEvent::FinalTranscript` is eligible for transcript retention or for the ASR-011 Gemini text-turn handoff. `PartialTranscript` is transient local/UI state only. Speech lifecycle and error events are not transcript records.

No ASR-010 code writes persistence directly. The eventual caller must gate final-record persistence through the existing `save_transcripts` setting.

## Timing metadata

The current Moonshine adapter does not provide application monotonic speech-boundary timestamps, so ASR-010 emits `SpeechStarted`/`SpeechEnded` with `monotonic_ms: None`. The provider-neutral event shape intentionally allows a later VAD/capture integration to supply timestamps without changing transcript semantics.

## Deterministic coverage

The state-machine tests use synthetic stable segment IDs and line sequences and cover:

- first partial + replacement;
- unchanged partial suppression;
- partial-to-final completion;
- duplicate/changed-after-final suppression;
- final-without-partial lifecycle;
- segment replacement before final;
- stale late partial/final suppression;
- blank updates;
- final-only retention eligibility.

Pipeline integration tests additionally prove that Moonshine-specific worker updates are translated into provider-neutral partial/final speech lifecycle events before crossing the worker callback boundary.
