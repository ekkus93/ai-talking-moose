# Talking Moose AI — Google Gemini Integration

Verified against the Google Gemini API documentation on 2026-08-21.

## 1. Provider interfaces

The backend defines provider-neutral traits in `src-tauri/src/ai/traits.rs`:

- `RealtimeConversationProvider` / `LiveSession`: bidirectional Live API sessions.
- `TextModel`: provider-neutral short remarks and typed/ambient text generation. When the **Google text provider** is selected, its default model is `gemini-3.7-flash`; fresh profiles now default to the separate Local text provider.
- `SpeechSynthesizer`: standalone speech synthesis for non-Live remarks.

Google model IDs, capability metadata, and the Live WebSocket endpoint are centralized in
`src-tauri/src/ai/google/config.rs`. Production settings expose model choices from that typed
catalog rather than maintaining a second UI-only list. Persisted stale or capability-incompatible
model IDs normalize to the current defaults, and settings updates reject a model selected for the
wrong capability.

The Google text-model catalog/default is not the same thing as the application's `text_provider` default. Local Text V1 routes typed replies and ambient remarks through `LocalTextModel` when `text_provider = local`; it does not construct a Google text request and does not fall back to Google on Local failure. Gemini Live and Google TTS remain independent Google-backed capabilities. See `LOCAL_LLM_ARCHITECTURE.md` and `PRIVACY.md`.

## 2. Current Gemini Live configuration

- **Live model:** `gemini-3.1-flash-live-preview`
- **WebSocket endpoint:** `wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent`
- **Authentication:** the configured API key is appended only when opening the socket; it is not
  exposed through the model catalog or normal logs.
- **Setup frame:** configures the selected model, `AUDIO` response modality, voice, Moose system
  instruction, transcription, tool declarations, session resumption, and context-window
  compression.
- **Audio framing:** Gemini Live Cloud Audio mode sends 16-bit PCM at 16 kHz mono as base64 audio
  frames. Moonshine modes never upload microphone PCM.
- **Local-ASR text turns:** final Moonshine transcripts are sent as text turns; partial transcripts
  remain local UI state. See `MOONSHINE_GEMINI_HANDOFF.md`.
- **Output streaming:** returned audio is 24 kHz PCM and is queued through Rust CPAL playback.

## 3. Setup acknowledgement is the readiness gate

Opening a WebSocket is not considered a ready conversation. `GoogleLiveProvider::connect` sends the
setup frame and then waits for an explicit `setupComplete` server message. The setup phase has a
bounded timeout and maps timeout, rejection, malformed setup traffic, and premature closure into
structured provider errors.

The conversation manager stays in `Connecting` while `provider.connect(...)` is pending. Playback,
microphone capture, the active session ID, and the `Listening` lifecycle state are established only
after the provider returns a setup-acknowledged session. A setup failure therefore cannot produce a
false-ready UI state or start cloud microphone upload.

Ordinary tests exercise the setup gate with fake WebSocket frame streams; they do not contact
Google or require a credential.
