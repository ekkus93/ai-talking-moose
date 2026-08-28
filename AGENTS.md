# Agent Developer Guide — Talking Moose AI

## Architecture & Project Structure

- **Desktop Shell:** Tauri 2 (`src-tauri/`) with custom retro window styling (`decorations: false`).
- **Frontend (`src/`):** React 18, TypeScript (strict), Tailwind CSS, Zustand (`src/stores/mooseStore.ts`), Lucide-React, Vitest.
- **Backend (`src-tauri/src/`):**
  - `ai/`: Gemini Live Bidi WebSocket client (`ai/google/live.rs`), REST text client (`ai/google/text.rs`), standalone Gemini TTS client (`ai/google/tts.rs`), provider traits (`ai/traits.rs`), and fake provider (`ai/fake.rs`).
  - `asr/`: Provider-neutral ASR types/lifecycle, bounded microphone-to-ASR pipeline, transcript state machine, and native Moonshine Tiny/Small streaming implementation (`asr/moonshine/`).
  - `audio/`: `cpal` mic capture/playback, 16kHz mono resampling (`audio/resample.rs`), RMS level tracking (`audio/levels.rs`), and synthesized-audio queueing/cancellation (`audio/speech.rs`).
  - `conversation/`: Generation-aware conversation/session lifecycle coordinating provider sessions, microphone capture, local ASR, playback, barge-in, and shutdown.
  - `character/`: State machine (`character/state.rs`), personality sliders (`character/personality.rs`), prompt builder (`character/prompt.rs`), cooldown & annoyance budget (`character/cooldown.rs`).
  - `desktop/`: Desktop observation/event runtime, including platform-specific macOS observers and the fail-closed non-macOS fallback branch in the same module.
  - `persistence/` & `memory/`: SQLite-backed settings, memories, transcripts, and related application metadata (`persistence/sqlite.rs`).
  - `tools/`: Built-in tools (`tools/builtin/`) dispatched through the bounded/privacy-gated `tools/router.rs` with metadata-only audit diagnostics.

---

## Verification & Build Commands

Always run verification before concluding changes. Prefer the repository scripts so local verification stays aligned with CI:

```bash
# Full repository gate: frontend + Rust
npm run check:all

# Equivalent split gates when isolating a failure
npm run check:frontend
npm run check:rust

# Compile-only production checks
npm run build
cargo build --manifest-path src-tauri/Cargo.toml
```

`npm run check:rust` is the source of truth for the Rust quality gate: it runs Rustfmt plus Clippy and tests with `--all-targets --all-features`. Do not replace it with weaker hand-written Clippy/test commands when validating a change. Release/bundle-specific checks are defined in `.github/workflows/ci.yml` and the scripts it invokes.

---

## Critical Toolchain & Runtime Quirks

1. **Rustls Crypto Provider:** Rustls 0.23 requires installing a default crypto provider on startup (`rustls::crypto::ring::default_provider().install_default()`) in `src-tauri/src/lib.rs` before establishing any TLS WebSockets.
2. **Tauri Async Tasks:** Use the existing Tauri runtime pattern (`tauri::async_runtime::spawn`) for Tauri-owned background tasks started from setup/runtime components; do not introduce a separate async runtime.
3. **Window Dragging:** Custom draggable regions in Tauri 2 require permissions defined in `src-tauri/capabilities/default.json` and `getCurrentWindow().startDragging()` triggered from `onMouseDown`.
4. **Audio Chunking:** Microphone capture via `cpal` downmixes to mono, resamples to 16kHz, and batches into 100ms frames (1,600 samples of 16-bit PCM = 3,200 bytes). `GeminiLiveAudio` sends those chunks through Gemini Live `realtimeInput`; local Moonshine modes route the same capture stream into the local ASR pipeline instead.
5. **Gemini Live Protocol:** Client messages are JSON WebSocket text frames. Server JSON may arrive as `Message::Text` or UTF-8 `Message::Binary`, so both are decoded as JSON. Model audio is base64 `inlineData` inside that JSON and is decoded to PCM before playback; binary WebSocket frames are not treated as raw audio.
6. **Gemini Model Identifiers:** Model ids are centralized in `src-tauri/src/ai/google/config.rs`; do not duplicate literals at call sites. The current defaults are Live Audio `gemini-3.1-flash-live-preview`, text/remarks `gemini-3.7-flash` (with `gemini-3.6-flash` also in the text catalog), and standalone TTS `gemini-2.5-flash-preview-tts`.
7. **Speech Output:** Non-Live synthesized utterances use `GoogleSpeechSynthesizer` (`src-tauri/src/ai/google/tts.rs`) and `audio/speech.rs` to queue PCM through Rust `AudioPlayback`/`cpal`. Gemini Live response audio is decoded from Live `inlineData` and queued through the same Rust playback layer. Browser speech synthesis and platform speech subprocesses such as `espeak`/`say` are intentionally not fallback paths.
