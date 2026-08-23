# Talking Moose AI 🫎

[![CI](https://github.com/ekkus93/ai-talking-moose/actions/workflows/ci.yml/badge.svg)](https://github.com/ekkus93/ai-talking-moose/actions/workflows/ci.yml)

> A modern reimagining of the classic 1986 Macintosh desktop character, built with Tauri 2, React, TypeScript, Rust, local Moonshine ASR, and Google Gemini.

---

## Overview

**Talking Moose AI** lives directly on your desktop inside a deliberately retro-styled Macintosh window. Rather than being a corporate support chatbot, the Moose is a dry-witted, humorous desktop character.

- **Real-time spoken conversation:** Click the Moose to start a voice conversation.
- **Local or cloud speech recognition:** Moonshine Tiny/Small provide local ASR; Gemini Live provides an explicitly selected cloud-audio mode.
- **Instant barge-in:** Interrupt the Moose while he is talking and the active response is stopped and flushed.
- **Ambient reactions:** When explicitly enabled, the Moose can react to permitted local computer events.
- **Retro visuals:** Low-resolution integer-scaled pixel art with amplitude-driven mouth animation.
- **Local control and privacy:** Settings, optional memories, and optional transcripts are stored locally. Privacy-sensitive features default conservatively and local ASR does not silently fall back to cloud microphone upload.

---

## Architecture

- **Frontend:** React 18, TypeScript, Tailwind CSS, Lucide icons, Zustand
- **Desktop shell/backend:** Tauri 2, Rust, Tokio, CPAL, Rusqlite, Tokio-Tungstenite
- **Speech recognition:** Local Moonshine streaming ASR plus optional Gemini Live cloud audio
- **AI:** Google Gemini Live over WebSockets and Gemini text generation through the REST API
- **Speech output:** Rust-owned TTS/audio playback pipeline
- **Persistence:** SQLite for local application data and the platform secure credential store for the Google API key

Gemini model IDs and capabilities are centralized in the Rust Google configuration layer rather than duplicated throughout the frontend.

---

## Development Environment

CI is the reference environment. For the closest local match, use:

- **Node.js 22**
- **npm** with the checked-in `package-lock.json`
- **Rust stable** with the `rustfmt` and `clippy` components
- **Tauri 2 system dependencies** for your operating system

### Install Node.js

Use any Node version manager you prefer, but select Node 22 before installing dependencies.

Verify:

```bash
node --version
npm --version
```

### Install Rust

Using `rustup`:

```bash
rustup toolchain install stable --component rustfmt --component clippy
rustup default stable
```

Verify:

```bash
rustc --version
cargo --version
cargo fmt --version
cargo clippy --version
```

### Linux system packages

The GitHub Actions Rust job uses the following packages on Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  build-essential \
  libasound2-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libssl-dev \
  libwebkit2gtk-4.1-dev \
  libxdo-dev
```

### macOS prerequisites

Install the Xcode Command Line Tools:

```bash
xcode-select --install
```

Local native-ASR packaging also requires CMake, Git LFS, and Python 3 as build tools. They are used only to prepare the pinned Moonshine runtime; the finished application does not depend on Homebrew or another developer-machine library path. Before a local Tauri bundle build, run:

```bash
bash scripts/prepare_moonshine_macos.sh "$(uname -m)"
```

The script verifies the pinned Moonshine source revision and architecture-specific ONNX Runtime checksum, then stages the two dylibs that Tauri embeds under the application `Contents/Frameworks` directory. The macOS bundle jobs in CI run this preparation and the clean-environment native-load smoke check on both Apple Silicon and Intel runners.

---

## Clone and Install

```bash
git clone https://github.com/ekkus93/ai-talking-moose.git
cd ai-talking-moose
npm ci
```

`npm ci` is preferred over `npm install` for validation because CI uses the lockfile exactly.

Cargo dependencies are resolved automatically by Cargo when running the Rust build/test commands.

---

## Run the Application

Start the complete Tauri desktop application in development mode:

```bash
npm run tauri dev
```

For frontend-only development with Vite:

```bash
npm run dev
```

---

## Build

### Frontend production build

```bash
npm run build
```

This runs TypeScript compilation followed by the Vite production build.

### Rust release build

```bash
cargo build --manifest-path src-tauri/Cargo.toml --release
```

### Tauri application bundle

On macOS, generate the deterministic release icons and prepare the pinned native Moonshine runtime before invoking Tauri:

```bash
python3 scripts/generate_app_icons.py
bash scripts/prepare_moonshine_macos.sh "$(uname -m)"
```

Then build the normal packaged application, matching the non-tagged macOS CI smoke build:

```bash
npm run tauri build -- --bundles app
```

For a local unsigned macOS application plus DMG:

```bash
npm run tauri build -- --bundles app,dmg
```

Unsigned local/CI bundles are smoke-test artifacts only. Public distribution uses the dedicated Developer ID signing/notarization workflow documented in `docs/MACOS_RELEASE.md`.

---

## Quality, Linting, Formatting, and Tests

### Complete ordinary quality gate

Run the same aggregate frontend/Rust validation intended for normal development:

```bash
npm run check:all
```

Default tests must not require a Google credential or contact live Google APIs.

### Frontend quality gate

```bash
npm run check:frontend
```

Equivalent individual commands:

```bash
# TypeScript
npm run typecheck

# ESLint
npm run lint

# Prettier verification
npm run format:check

# Write Prettier formatting
npm run format:write

# Vitest
npm test

# Watch-mode Vitest
npm run test:watch

# Production frontend build
npm run build
```

### Rust quality gate

```bash
npm run check:rust
```

Equivalent individual commands, matching CI:

```bash
# rustfmt
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check

# Clippy; warnings are errors
cargo clippy \
  --manifest-path src-tauri/Cargo.toml \
  --all-targets \
  --all-features \
  -- \
  -D warnings

# Rust tests
cargo test \
  --manifest-path src-tauri/Cargo.toml \
  --all-targets \
  --all-features
```

To apply Rust formatting before re-running the check:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
```

---

## Dependency Audits

CI treats production dependency vulnerabilities as blocking.

### Node production dependencies

```bash
npm audit --omit=dev --audit-level=high
```

Development-tool advisories are currently reported separately in CI:

```bash
npm audit --include=dev --audit-level=high
```

### Rust dependencies

CI runs RustSec using `rustsec/audit-check`. Locally, if `cargo-audit` is installed:

```bash
cargo install cargo-audit
cargo audit --file src-tauri/Cargo.lock
```

---

## Optional Live Gemini Integration Test

The real Gemini Live integration test is intentionally excluded from ordinary test runs. It must be explicitly enabled and supplied with a dedicated test credential:

```bash
TALKING_MOOSE_ALLOW_LIVE_API=1 \
TALKING_MOOSE_GOOGLE_API_KEY='...' \
cargo test \
  --manifest-path src-tauri/Cargo.toml \
  --test test_gemini_live_asr \
  -- \
  --ignored
```

Do not put a Google API key in source code, committed configuration, SQLite settings, ordinary test fixtures, or shell history intended for sharing.

---

## Continuous Integration

`.github/workflows/ci.yml` runs on pushes to `master`, pull requests targeting `master`, tags, and manual dispatches.

The normal CI gates include:

- frontend type checking
- ESLint
- Prettier verification
- Vitest
- frontend production build
- Rust formatting
- Clippy with `-D warnings`
- Rust tests
- production Node dependency audit
- RustSec dependency audit
- macOS Tauri application bundle smoke build

`.github/workflows/release.yml` is separate from ordinary CI and runs only for semantic `v*.*.*` tags. It requires Developer ID + Apple notarization credentials, verifies hardened-runtime/signature/stapling/Gatekeeper state for both Apple Silicon and Intel artifacts, computes SHA-256 manifests, and creates a **draft** GitHub Release for final physical acceptance. See `docs/MACOS_RELEASE.md`.

The V1 deployment target is macOS 10.15 on Intel; Apple Silicon hardware requires macOS 11 or later.

---

## License

MIT License.
