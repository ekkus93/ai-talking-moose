# Talking Moose AI — Audio Architecture & Lip Sync

## 1. Rust Audio Pipeline

Audio capture and playback are owned entirely by the Rust core via `cpal`:
- **Input Pipeline:** Device format -> Downmix to mono -> Resample to 16,000 Hz -> 16-bit signed PCM frames -> Gemini Live WebSocket.
- **Output Pipeline:** 24,000 Hz PCM chunks from Live/TTS -> Output ring buffer queue -> Speakers -> RMS Level analysis -> `MouthShape` event emission (`moose://mouth`).

## 2. Real-Time Lip Synchronization (Envelope Analysis)

Mouth animation uses RMS energy with hysteresis smoothing:
- `Closed`: RMS < 0.03
- `Small`: 0.03 <= RMS <= 0.12
- `Medium`: 0.12 < RMS <= 0.35
- `Wide`: RMS > 0.35

## 3. Instant Barge-In

When user speech is detected or the user clicks/speaks, `AudioPlayback::flush()` immediately drops all buffered output audio frames, silences playback, and switches the character state to `Interrupted -> Listening`.
