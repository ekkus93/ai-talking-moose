# Talking Moose AI — Google Gemini Integration

## 1. Provider Interfaces

The backend defines clean trait abstractions in `src-tauri/src/ai/traits.rs`:
- `RealtimeConversationProvider` / `LiveSession`: Bidirectional WebSocket streaming for voice conversations with barge-in.
- `TextModel`: Short remarks and ambient reaction classification (`gemini-2.0-flash-exp`).
- `SpeechSynthesizer`: Standalone TTS for ambient comments with voice selection (e.g. `Puck`).

## 2. Gemini Live Protocol (WebSockets)

- **Endpoint:** `wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1alpha.GenerativeService.BidiGenerateContent?key={API_KEY}`
- **Setup Frame:** Configures model, response modalities (`["AUDIO"]`), voice configuration, and Moose character system instructions.
- **Audio Framing:** Input audio stream sends 16-bit PCM at 16kHz mono base64 chunks.
- **Output Streaming:** Model audio stream receives 24kHz PCM chunks played directly through Rust CPAL output queues.
