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
# Complete ordinary quality gate: TypeScript, ESLint, Prettier, Vitest,
# frontend production build, rustfmt, Clippy (-D warnings), and Rust tests.
# Default tests do not require Google credentials or live APIs.
npm run check:all

# Run only one side when iterating locally.
npm run check:frontend
npm run check:rust

# Start Tauri development app.
npm run tauri dev

# Production build.
npm run build
cargo build --manifest-path src-tauri/Cargo.toml --release
```

The real Gemini Live integration test is intentionally excluded from ordinary test runs. To run it manually, opt in twice and provide a dedicated test credential:

```bash
TALKING_MOOSE_ALLOW_LIVE_API=1 \
TALKING_MOOSE_GOOGLE_API_KEY='...' \
cargo test --manifest-path src-tauri/Cargo.toml --test test_gemini_live_asr -- --ignored
```

GitHub Actions runs the ordinary frontend and Rust gates, dependency audits, and a macOS Tauri smoke build. Build artifacts are retained only for tag-triggered workflows.

---

## License

MIT License.
