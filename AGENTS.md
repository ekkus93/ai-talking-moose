# Agent Developer Guide — Talking Moose AI

## Architecture & Project Structure

- **Desktop Shell:** Tauri 2 (`src-tauri/`) with custom retro window styling (`decorations: false`).
- **Frontend (`src/`):** React 18, TypeScript (strict), Tailwind CSS, Zustand (`src/stores/mooseStore.ts`), Lucide-React, Vitest.
- **Backend (`src-tauri/src/`):**
  - `ai/`: Gemini Live Bidi WebSocket client (`ai/google/live.rs`), REST text client (`ai/google/text.rs`), trait interfaces (`ai/traits.rs`), and fake provider (`ai/fake.rs`).
  - `audio/`: `cpal` mic capture/playback, 16kHz mono resampling (`audio/resample.rs`), RMS envelope lip sync (`audio/levels.rs`), system speech (`audio/speech.rs`).
  - `character/`: State machine (`character/state.rs`), personality sliders (`character/personality.rs`), prompt builder (`character/prompt.rs`), cooldown & annoyance budget (`character/cooldown.rs`).
  - `persistence/` & `memory/`: SQLite tables for settings, memories, and transcripts (`persistence/sqlite.rs`).
  - `tools/`: Built-in safe tools (`tools/builtin/`) dispatched through `tools/router.rs`.

---

## Verification & Build Commands

Always run verification before concluding changes:

```bash
# Frontend: Typecheck, Lint, Prettier check, Vitest
npm run check:all

# Backend: Rustfmt, Clippy (zero warnings), Unit & Integration tests
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml

# Production Build
npm run build
cargo build --manifest-path src-tauri/Cargo.toml
```

---

## Critical Toolchain & Runtime Quirks

1. **Rustls Crypto Provider:** Rustls 0.23 requires installing a default crypto provider on startup (`rustls::crypto::ring::default_provider().install_default()`) in `src-tauri/src/lib.rs` before establishing any TLS WebSockets.
2. **Tauri Async Tasks:** Use `tauri::async_runtime::spawn` for background tasks inside Tauri command handlers and setup hooks.
3. **Window Dragging:** Custom draggable regions in Tauri 2 require permissions defined in `src-tauri/capabilities/default.json` and `getCurrentWindow().startDragging()` triggered from `onMouseDown`.
4. **Audio Chunking:** Microphone capture via `cpal` downmixes to mono, resamples to 16kHz, and batches into 100ms frames (1,600 samples of 16-bit PCM = 3,200 bytes) before sending `realtimeInput` frames to Gemini Live.
5. **Gemini Live Protocol:** Google's Gemini Live Bidi endpoint transmits responses as `Message::Binary` frames containing JSON and raw audio. Responses must be parsed for both `Message::Binary` and `Message::Text`.
6. **Gemini Model Identifiers:**
   - Real-time Live Audio: `models/gemini-2.5-flash-native-audio-latest` (or `models/gemini-3.1-flash-live-preview`).
   - Text & Remarks: `gemini-2.5-flash` or `gemini-flash-latest`.
7. **Speech Output:** Conversational spoken responses use `espeak` on Linux and `say` on macOS (`src-tauri/src/audio/speech.rs`) with synchronized mouth frame animation in `SpeechBubble` / `MooseSprite`.
