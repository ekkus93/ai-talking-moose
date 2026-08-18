# Talking Moose AI 🫎

[![CI](https://github.com/ekkus93/ai-talking-moose/actions/workflows/ci.yml/badge.svg)](https://github.com/ekkus93/ai-talking-moose/actions/workflows/ci.yml)

> A modern reimagining of the classic 1986 Macintosh desktop character, powered by Tauri 2, React, TypeScript, Rust, and Google Gemini Live.

---

## Overview

**Talking Moose AI** lives directly on your desktop inside a retro-styled Macintosh window. Rather than being a corporate support chatbot, the Moose is a dry-witted, humorous cartoon character:
- **Ambient Reactions:** Observes permitted local computer events (like rapid application switching) and occasionally makes witty, deadpan remarks.
- **Real-Time Spoken Conversation:** Click the Moose to engage in real-time voice conversations powered by **Google Gemini Live**.
- **Instant Barge-In:** Speak while the Moose is talking and he will immediately stop, react, and listen.
- **Retro Visuals:** Low-resolution integer-scaled pixel art sprite animation with amplitude-driven lip synchronization.
- **Local Control & Privacy:** All memories and settings are stored locally in SQLite with granular opt-in permissions and a complete "Forget Everything" action.

---

## Architecture

- **Frontend:** React 18, TypeScript, Tailwind CSS, Lucide icons, Zustand
- **Desktop Shell & Backend:** Tauri 2, Rust, Tokio, CPAL (Audio I/O), Rusqlite, Tokio-Tungstenite
- **AI Layer:** Google Gemini Live (WebSockets), Gemini Text (`gemini-2.0-flash-exp`), Google TTS / Puck Voice

---

## Development & Build Commands

### Prerequisites
- Node.js >= 18
- Rust & Cargo >= 1.75
- Tauri 2 dependencies (WebKitGTK on Linux, Xcode Command Line Tools on macOS)

### Quality & Verification Commands

```bash
# Run all frontend static checks (Typecheck, Lint, Format, Vitest)
npm run check:all

# Run Rust formatting, clippy, and unit/integration tests
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml

# Start Tauri development app
npm run tauri dev

# Production Build
npm run build
cargo build --manifest-path src-tauri/Cargo.toml --release
```

GitHub Actions runs the frontend, Rust, and macOS Tauri bundle checks on pushes to `master`, pull requests, tags, and manual runs. Build artifacts are retained only for tagged commits.

---

## License

MIT License.
