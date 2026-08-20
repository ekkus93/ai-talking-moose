# Moonshine → Gemini Live Text Handoff

Status: **V1R-ASR-011 implementation contract**
Recorded: 2026-08-20

## Purpose

Local Moonshine modes keep microphone PCM on-device while still using the active Gemini Live session for the Moose's conversational response. The provider boundary therefore accepts two distinct input operations:

- `send_audio_chunk` for **Gemini Live Cloud Audio** mode only;
- `send_text_turn` for a finalized Moonshine transcript.

No local-ASR path is permitted to call the provider audio-upload operation.

## Text-turn wire contract

`GoogleLiveSession::send_text_turn` sends exactly one Live API realtime text input message:

```json
{
  "realtimeInput": {
    "text": "finalized local transcript"
  }
}
```

`realtimeInput.text` is used instead of incremental `clientContent` because the configured model list includes `gemini-3.1-flash-live-preview`, whose current Live API contract limits `clientContent` to initial-history seeding after the first model turn. The response modality remains native audio, so the existing Gemini Live receive path can continue to emit transcript and `AudioData` events without a separate TTS round trip.

## Final-only routing

`ConversationManager::handle_local_asr_event` accepts provider-neutral `AsrEvent` values. It forwards only `FinalTranscript`:

- `PartialTranscript` remains local and is not persisted or sent as a Gemini turn;
- `SpeechStarted` and `SpeechEnded` remain lifecycle metadata;
- `Error` remains an ASR error, not model input;
- blank final text is rejected defensively.

ASR-010 remains responsible for duplicate-final suppression, so one accepted final utterance produces one provider text-turn call.

## Session attribution and stale callbacks

The handoff is fail-closed. Before a final transcript is accepted it must match all of these conditions:

1. a conversation is active;
2. the captured conversation generation is still current;
3. the local-ASR lifecycle still owns a resource for that generation;
4. the captured session ID still matches the active session ID;
5. the active ASR mode is Moonshine Tiny Streaming or Moonshine Small Streaming.

The user transcript callback receives that same active session ID and the `user` role. A stale callback therefore cannot write a transcript or send text into a newer Gemini session.

The final-turn operation is serialized with conversation start, stop, and barge-in. This prevents a callback admitted for one generation from racing teardown or an interrupt boundary and landing in a replacement Live session.

A valid finalized local utterance uses the same authoritative user-turn transition as a cloud transcript: it clears interrupted-response suppression, attributes the transcript to the active session as `user`, advances the conversation lifecycle to `Responding`, and sets the character to `Thinking` before the Gemini text turn is sent. This preserves ordering even if Gemini replies immediately.

## Microphone privacy boundary

`ConversationManager::forward_microphone_chunk` checks the active conversation's selected ASR mode before touching the provider session:

- `GeminiLiveAudio` may call `send_audio_chunk`;
- Moonshine Tiny and Moonshine Small return locally without calling the provider audio-upload API.

This guard sits inside the conversation manager rather than only in the Tauri command layer, so later ASR lifecycle wiring cannot bypass it accidentally.

## Lifecycle boundary

ASR-011 supplies the guarded provider handoff API but intentionally does **not** start or attach the real `LocalAsrPipeline`. V1R-ASR-012 owns that lifecycle integration, including wiring the pipeline callback to `handle_local_asr_event`, teardown, provider switching, and terminal error handling.

## Deterministic coverage

Tests prove that:

- the Google text-turn wire message contains only `realtimeInput.text` and no audio media payload;
- Moonshine Tiny and Small make zero provider `send_audio_chunk` calls;
- the same fake provider does receive an audio-upload call in Gemini Live Cloud Audio mode, proving the negative Moonshine assertion is meaningful;
- partial and speech lifecycle events make no provider text calls;
- one final transcript produces exactly one text-turn call;
- transcript attribution uses the expected active session ID and `user` role;
- a stale generation/session cannot write a transcript or reach the provider;
- a local final clears response suppression and enters `Responding`/`Thinking` before Gemini can reply;
- a local-final callback cannot race serialized conversation operation boundaries;
- Gemini Cloud Audio mode rejects the local-final routing path.
