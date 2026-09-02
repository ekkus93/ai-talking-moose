# Talking Moose AI 🫎

[![CI](https://github.com/ekkus93/ai-talking-moose/actions/workflows/ci.yml/badge.svg)](https://github.com/ekkus93/ai-talking-moose/actions/workflows/ci.yml)

> A modern reimagining of the classic 1986 Macintosh desktop character, built with Tauri 2, React, TypeScript, Rust, local Moonshine ASR, local llama.cpp text generation, and Google Gemini.

---

## Overview

**Talking Moose AI** lives directly on your desktop inside a deliberately retro-styled Macintosh window. Rather than being a corporate support chatbot, the Moose is a dry-witted, humorous desktop character.

- **Real-time spoken conversation:** Click the Moose to start a voice conversation.
- **Local or cloud speech recognition:** Moonshine Tiny/Small provide local ASR; Gemini Live provides an explicitly selected cloud-audio mode.
- **Instant barge-in:** Interrupt the Moose while he is talking and the active response is stopped and flushed.
- **Local-first text generation:** New profiles use the Local text provider by default. SmolLM2 360M is selected but is never downloaded until the user explicitly chooses **Download & Verify**. Google Gemini remains an optional text provider.
- **Ambient reactions:** When explicitly enabled, the Moose can react to permitted local computer events.
- **Retro visuals:** Low-resolution integer-scaled pixel art with amplitude-driven mouth animation.
- **Local control and privacy:** Settings, optional memories, and optional transcripts are stored locally. Privacy-sensitive features default conservatively and local ASR does not silently fall back to cloud microphone upload.

---

## Architecture

- **Frontend:** React 18, TypeScript, Tailwind CSS, Lucide icons, Zustand
- **Desktop shell/backend:** Tauri 2, Rust, Tokio, CPAL, Rusqlite, Tokio-Tungstenite
- **Speech recognition:** Local Moonshine streaming ASR plus optional Gemini Live cloud audio
- **Text generation:** Provider-neutral `TextModel` routing; Local uses pinned llama.cpp/ggml through Rust, while Google Gemini remains selectable through the REST API
- **Voice conversation:** Google Gemini Live over WebSockets; Local text selection does not replace the V1 voice provider
- **Speech output:** Rust-owned TTS/audio playback pipeline
- **Persistence:** SQLite for local application data and the platform secure credential store for the Google API key

Google model IDs and capabilities are centralized in the Rust Google configuration layer rather than duplicated throughout the frontend. Local GGUF identities, revisions, hashes, sizes, licenses, template hints, and runtime bounds are centralized in `src-tauri/src/ai/local/catalog.rs`. See `docs/LOCAL_LLM_ARCHITECTURE.md` and `docs/LOCAL_LLM_CATALOG.md`.

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
  clang \
  cmake \
  libasound2-dev \
  libayatana-appindicator3-dev \
  libclang-dev \
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

CMake is also required to compile the pinned Local LLM llama.cpp binding. Local native-ASR packaging additionally requires Git LFS and Python 3 to prepare the pinned Moonshine runtime. These are build-time tools; the finished application does not depend on a separately installed llama.cpp, Homebrew library path, or other developer-machine runtime path. Before a local Tauri bundle build, run:

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

## Local Text Quick Start

A fresh profile defaults to **Local** text generation with **SmolLM2 360M Instruct Q4_K_M** selected. Selection is only configuration: the application does **not** download model weights during startup, onboarding, provider selection, ordinary CI, or packaging.

1. Start the Tauri application with `npm run tauri dev` or open a packaged build.
2. Open **Settings → AI & Models**.
3. Keep **Local** selected for Text Generation, or select it explicitly.
4. For the selected model, choose **Download & Verify**.
5. Wait for download, exact-size verification, SHA-256 verification, and atomic installation to finish.
6. Use **Test Local Model** if you want a short bounded local inference smoke test.

Supported V1 Local text models:

| Model | Download bytes | Approx. decimal size | P12 CPU evidence |
| --- | ---: | ---: | --- |
| SmolLM2 360M Instruct Q4_K_M | 270,590,880 | 270.6 MB | ~297 MiB measured RSS delta; ~36.3 tok/s |
| Qwen3 0.6B Q4_K_M, non-thinking | 484,220,320 | 484.2 MB | ~713 MiB measured RSS delta; ~14.2 tok/s |

Those measurements came from the canonical Linux x86_64 real-CPU acceptance run on an AMD EPYC host and are **not performance guarantees** for another computer. They are runtime/usability measurements, not a semantic-quality benchmark. SmolLM2 remains the recommended Local default because it had materially lower disk/RAM/CPU cost in that acceptance run. See `docs/LOCAL_LLM_CPU_ACCEPTANCE_20260902.md`.

A Google AI Studio API key is **not required for Local text generation**. It is still required when you select Google text generation, for the V1 Gemini Live spoken-conversation provider, and for Google TTS. Local Moonshine ASR only makes speech recognition local; finalized Moonshine transcripts still go to Gemini Live for spoken conversation in this phase.

Local generation does not silently fall back to Google or Fake. A missing, corrupt, incompatible, or failed Local model produces an explicit failure.

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

### Frontend-only Vite preview

`npm run dev` is a **development-only UI preview**. When Tauri IPC is absent in a Vite development build, `src/lib/tauriBridge.ts` selects the explicitly named `browserPreviewBridge`. Its fabricated values are for UI development only and are not evidence that credentials, providers, persistence, model installation, audio, or conversation operations succeeded.

Production selection is fail-closed: packaged/native code prefers Tauri IPC whenever it is present, and a non-development build with missing or malformed Tauri IPC throws instead of falling through to the preview adapter. Preview selection is controlled by Vite's compile-time `import.meta.env.DEV`; it cannot be enabled by a query parameter, `localStorage`, or another browser runtime flag.

The preview surface is intentionally inventoried here so simulated effects do not become implicit production behavior:

- **Read-only/presentation (18):** `resizeWindow`, `getSettings`, `getOnboardingStatus`, `getGoogleModels`, `getGoogleTtsVoices`, `getAsrModels`, `getAsrDiagnostics`, `onAsrModelProgress`, `listAudioDevices`, `getMicrophonePermission`, `getToolAudit`, `getAudioDiagnostics`, `getCharacterState`, `getConversationLifecycle`, `isMuted`, `getMemories`, `getTranscripts`, `listenEvent`, plus local presentation defaults associated with these reads.
- **Simulated external state/effect (22):** `acknowledgeOnboarding`, `updateSettings`, `installAsrModel`, `deleteAsrModel`, `setGoogleApiKey`, `clearGoogleApiKey`, `hasGoogleApiKey`, `testAiConnection`, `requestMicrophoneAccess`, `testMicrophone`, `testAudioOutput`, `setCharacterState`, `triggerCannedReaction`, `auditionVoice`, `cancelStandaloneSpeech`, `startConversation`, `stopConversation`, `bargeIn`, `setMute`, `deleteMemory`, `forgetEverything`, and `sendTextMessage`.

The second category may report simulated success **only** through the explicit development preview adapter. Production-like tests use Tauri IPC fixtures instead; dedicated preview tests are kept separate from production contract coverage.

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

The Local LLM runtime is compiled through the pinned `llama-cpp-2` / `llama-cpp-sys-2` Rust dependencies; a separately installed system `llama.cpp` executable or library is not part of the build contract. GGUF model weights are user-data downloads and are intentionally absent from application bundles.

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

Default tests must not require a Google credential, contact live Google APIs, or download Local GGUF model weights. Ordinary Local-model unit coverage uses injected runtimes rather than third-party model files.

The Local LLM packaging-policy gate can also be run directly without downloading model weights:

```bash
python3 scripts/check_local_llm_packaging_policy.py
```

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
- Local LLM packaging-policy validation
- Local LLM compile/CPU-policy proofs on Linux x86_64, macOS arm64, and macOS x86_64
- production Node dependency audit
- RustSec dependency audit
- dependency/license inventory validation
- macOS Tauri application bundle smoke builds on Apple Silicon and Intel, including a no-embedded-GGUF check

`.github/workflows/release.yml` is separate from ordinary CI and runs only for semantic `v*.*.*` tags. It requires Developer ID + Apple notarization credentials, verifies hardened-runtime/signature/stapling/Gatekeeper state for both Apple Silicon and Intel artifacts, computes SHA-256 manifests, and creates a **draft** GitHub Release for final physical acceptance. See `docs/MACOS_RELEASE.md`.

Real Local-model acceptance is deliberately separate from ordinary CI:

- `.github/workflows/local-llm-real-cpu-acceptance.yml` is manual-only and downloads the selected pinned GGUF through the production installer, then runs CPU generation with network access denied after installation.
- `.github/workflows/local-llm-p13-packaging-acceptance.yml` is manual-only and compares the current macOS bundles with the fixed pre-llama baseline on arm64 and x86_64. It downloads no GGUF weights.

Do not move either heavyweight acceptance path into ordinary CI without revisiting the model-weight-free CI policy documented in `docs/LOCAL_LLM_ARCHITECTURE.md`.

The V1 deployment target is macOS 13.4 on both Intel and Apple Silicon, matching the pinned ONNX Runtime 1.23.2 runtime floor.

---

## License

MIT License.
