# Talking Moose AI

Talking Moose AI is a desktop application that recreates the classic Talking Moose experience with modern AI capabilities.

## Features

- Character-driven desktop companion UI
- Google Gemini-backed conversation and speech features
- Local and cloud model support
- Configurable audio input/output
- Native macOS/Linux desktop packaging
- Offline-capable Local TTS support with KittenTTS Mini 0.8

## Development

### Prerequisites

- Node.js
- Rust toolchain
- Tauri system dependencies for your platform

### Install

```bash
npm install
```

### Run

```bash
npm run tauri dev
```

### Validation

```bash
npm run check:all
```

The repository also contains path-scoped CI and explicit native/runtime acceptance workflows for expensive platform-specific validation.

## License

Apache-2.0
