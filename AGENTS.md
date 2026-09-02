# Agent Developer Guide — Talking Moose AI

## Architecture & Project Structure

- **Desktop Shell:** Tauri 2 (`src-tauri/`) with custom retro window styling (`decorations: false`).
- **Frontend (`src/`):** React 18, TypeScript (strict), Tailwind CSS, Zustand (`src/stores/mooseStore.ts`), Lucide-React, Vitest.
- **Backend (`src-tauri/src/`):**
  - `ai/`: Provider traits (`ai/traits.rs`), typed provider/settings models, Google Gemini Live (`ai/google/live.rs`), Google REST text (`ai/google/text.rs`), Google TTS (`ai/google/tts.rs`), explicit development/test fake provider, and Local text generation (`ai/local/`).
  - `ai/local/`: App-owned Local-model catalog, verified installer, `LocalTextModel`, llama.cpp-backed runtime manager, compile proof, and opt-in real-model acceptance support. Binding-specific llama.cpp types stay inside this module tree.
  - `asr/`: Provider-neutral ASR types/lifecycle, bounded microphone-to-ASR pipeline, transcript state machine, and native Moonshine Tiny/Small streaming implementation (`asr/moonshine/`).
  - `audio/`: `cpal` mic capture/playback, 16kHz mono resampling (`audio/resample.rs`), RMS level tracking (`audio/levels.rs`), and synthesized-audio queueing/cancellation (`audio/speech.rs`).
  - `conversation/`: Generation-aware conversation/session lifecycle coordinating provider sessions, microphone capture, local ASR, playback, barge-in, and shutdown.
  - `character/`: State machine (`character/state.rs`), personality sliders (`character/personality.rs`), prompt builder (`character/prompt.rs`), cooldown & annoyance budget (`character/cooldown.rs`).
  - `desktop/`: Desktop observation/event runtime, including platform-specific macOS observers and the fail-closed non-macOS fallback branch in the same module.
  - `persistence/` & `memory/`: SQLite-backed settings, memories, transcripts, and related application metadata (`persistence/sqlite.rs`).
  - `tools/`: Built-in tools (`tools/builtin/`) dispatched through the bounded/privacy-gated `tools/router.rs` with metadata-only audit diagnostics.

Local Text V1 is intentionally separate from voice: typed messages and ambient remarks may use Local or Google text generation; Gemini Live remains the realtime spoken-conversation provider, including for finalized Moonshine transcripts. See `docs/LOCAL_LLM_ARCHITECTURE.md` before changing that boundary.

---

## Verification & Build Commands

Always run verification before concluding changes. Prefer the repository scripts so local verification stays aligned with CI:

```bash
# Full repository gate: frontend + Rust + generated backend contract
npm run check:all

# Equivalent split gates when isolating a failure
npm run check:frontend
npm run check:rust

# Index hygiene; runs without installed frontend dependencies
npm run check:generated-trees

# Cross-language IPC contract checks (after npm ci)
npm run check:tauri-command-contract
npm run check:frontend-contract-shapes

# Regenerate Rust-derived contract and fail on committed drift
npm run check:generated-backend-contract

# Local LLM packaging/CI policy; no GGUF download
python3 scripts/check_local_llm_packaging_policy.py

# Compile-only production checks
npm run build
cargo build --manifest-path src-tauri/Cargo.toml
```

`npm run check:generated-trees` fails if generated `node_modules/` or `dist/` content is ever committed. Both directories are intentionally ignored; use `npm ci` to reproduce frontend dependencies and `npm run build`/Tauri's `beforeBuildCommand` to regenerate `dist/`. The hygiene check inspects the Git index only, so it can run before `npm ci` and does not require either directory to exist.

`npm run check:tauri-command-contract` compares every command string invoked by `src/lib/tauriBridge.ts` with the commands registered in Rust's `tauri::generate_handler!`; registered backend-only commands are informational. `npm run check:frontend-contract-shapes` compares Rust-derived representative IPC objects in `src/generated/backendContract.json` with the interface keys and top-level JSON value categories declared in `src/types/moose.ts`. This intentionally does not prove numeric narrowing, complete enum variants, optionality semantics, primitive per-command argument signatures, or the per-command association between a command name and its parameter names/types/return type. The shape gate proves shared object shapes independently; it is not a generated full command-signature binding layer.

`npm run check:generated-backend-contract` regenerates `src/generated/backendContract.json` from Rust and fails if the committed copy is stale, then reruns the shape check. Run `npm run generate:frontend-contract` intentionally when Rust defaults, provider catalogs, or an exported IPC shape changes, review the diff, and commit the regenerated JSON. These contract checks require no live Google credential, third-party GGUF, audio device, or microphone permission; dependency/toolchain installation is the only setup requirement.

`npm run check:rust` is the source of truth for the Rust quality gate: it runs Rustfmt plus Clippy and tests with `--all-targets --all-features`. Do not replace it with weaker hand-written Clippy/test commands when validating a change. Release/bundle-specific checks are defined in `.github/workflows/ci.yml` and the scripts it invokes.

`scripts/check_local_llm_packaging_policy.py` is the fail-closed static gate for the Local LLM packaging boundary. It verifies the supported-target compile matrix, exact pinned llama binding versions/feature policy, ordinary-CI model-weight isolation, model-license documentation, notice resources, and absence of a developer-local/system llama.cpp dependency. If it fails, fix the policy drift; do not weaken the checker merely to make CI green.

`scripts/measure_local_llm_bundle_impact.py` is the deterministic measurement helper used by the manual P13 packaging workflow. Given baseline/current `.app` paths and exact SHAs, it records logical regular-file bytes, executable bytes, file counts, absolute/percentage delta, and GGUF absence. It is evidence plumbing, not an ordinary unit gate; a synthetic/local invocation is not a substitute for the canonical two-architecture workflow.

### Local LLM acceptance is separate from ordinary CI

Do **not** add real GGUF downloads to `npm run check:all`, ordinary `CI`, contract export, or normal bundle smoke jobs. Unit/integration tests use injected runtimes where model weights are not the subject of the test.

Two heavyweight workflows are intentionally manual:

- `.github/workflows/local-llm-real-cpu-acceptance.yml` — **Local LLM Real CPU Acceptance**. Downloads a selected pinned catalog model through the production installer, then performs CPU generation with network access denied after installation and uploads machine-readable evidence.
- `.github/workflows/local-llm-p13-packaging-acceptance.yml` — **Local LLM P13 Packaging Acceptance**. Builds current and fixed pre-llama macOS bundles on arm64/x86_64, measures size impact, and rejects embedded GGUFs. It does not download model weights.

Only dispatch real-model acceptance when the task genuinely changes model identity, installer behavior, runtime/template behavior, or requires fresh real-model evidence. A successful ordinary CI run is not a substitute for required real-model acceptance, and a successful real-model run is not a substitute for ordinary quality gates.

When accepting a model, record the exact workflow run, source SHA, artifact identity/hash/bytes, and relevant evidence in `docs/`. Never invent unavailable metrics; the P12 harness deliberately records null first-token latency when the runtime cannot expose it separately.

---

## Local LLM Change Checklist

For changes touching Local text generation, determine which surfaces are affected before editing:

1. **Provider/settings default or migration:** update Rust settings tests, generated backend contract, frontend tests, onboarding/settings disclosure, and migration coverage. Preserve legacy Google profiles unless a deliberate migration says otherwise.
2. **Catalog metadata/model identity:** update `src-tauri/src/ai/local/catalog.rs`, `docs/LOCAL_LLM_CATALOG.md`, and `docs/LOCAL_LLM_MODEL_LICENSES.md`; run catalog/contract checks and real CPU acceptance for the changed model.
3. **Installer/storage:** preserve explicit download, bounded staging, exact-size/SHA verification, atomic promotion, cancellation, symlink/path defenses, and no implicit provider/model substitution.
4. **Runtime/template:** preserve CPU-first bounded execution, serialized ownership semantics, sanitized errors, prompt/output-free logs, and explicit failure for unsupported templates. Real CPU acceptance is required before declaring a changed runtime/model combination supported.
5. **Packaging/native dependency:** run the packaging-policy and license gates; keep GGUFs out of Git/resources/bundles and preserve both supported macOS architectures plus Linux compile proof.
6. **Privacy/UX:** keep `README.md`, `docs/PRIVACY.md`, onboarding, Settings, and `docs/LOCAL_LLM_ARCHITECTURE.md` aligned about what is local, what downloads, and what still uses Google.

Production Local failures must never silently call Google or Fake. Deleting a selected Local model leaves it selected and not-installed; do not silently choose another model. Selecting Local or a Local model never grants permission to download it.

---

## Critical Toolchain & Runtime Quirks

1. **Rustls Crypto Provider:** Rustls 0.23 requires installing a default crypto provider on startup (`rustls::crypto::ring::default_provider().install_default()`) in `src-tauri/src/lib.rs` before establishing any TLS WebSockets.
2. **Tauri Async Tasks:** Use the existing Tauri runtime pattern (`tauri::async_runtime::spawn`) for Tauri-owned background tasks started from setup/runtime components; do not introduce a separate async runtime.
3. **Window Dragging:** Custom draggable regions in Tauri 2 require permissions defined in `src-tauri/capabilities/default.json` and `getCurrentWindow().startDragging()` triggered from `onMouseDown`.
4. **Audio Chunking:** Microphone capture via `cpal` downmixes to mono, resamples to 16kHz, and batches into 100ms frames (1,600 samples of 16-bit PCM = 3,200 bytes). `GeminiLiveAudio` sends those chunks through Gemini Live `realtimeInput`; local Moonshine modes route the same capture stream into the local ASR pipeline instead.
5. **Gemini Live Protocol:** Client messages are JSON WebSocket text frames. Server JSON may arrive as `Message::Text` or UTF-8 `Message::Binary`, so both are decoded as JSON. Model audio is base64 `inlineData` inside that JSON and is decoded to PCM before playback; binary WebSocket frames are not treated as raw audio.
6. **Gemini Model Identifiers:** Google model IDs are centralized in `src-tauri/src/ai/google/config.rs`; do not duplicate literals at call sites. The Google-provider defaults are Live Audio `gemini-3.1-flash-live-preview`, Google text `gemini-3.7-flash` (with `gemini-3.6-flash` also in the text catalog), and standalone TTS `gemini-2.5-flash-preview-tts`. The **new-profile text-provider default is Local**, not Google.
7. **Speech Output:** Non-Live synthesized utterances use `GoogleSpeechSynthesizer` (`src-tauri/src/ai/google/tts.rs`) and `audio/speech.rs` to queue PCM through Rust `AudioPlayback`/`cpal`. Gemini Live response audio is decoded from Live `inlineData` and queued through the same Rust playback layer. Browser speech synthesis and platform speech subprocesses such as `espeak`/`say` are intentionally not fallback paths.
8. **Local Model Root:** The global Local installer is initialized from Tauri app data at `<app-data>/models/llm`. The catalog owns model IDs, revisions, and filenames; do not derive arbitrary filesystem paths from frontend/user input.
9. **Local Runtime Ownership:** `AppState` owns `Arc<LocalRuntimeManager>`, but llama.cpp model/context/sampler types stay private to `ai/local/runtime/`. Keep generation/deletion/switch synchronization inside that ownership boundary.
10. **Local Model Weights:** `.gguf`/`.GGUF` are intentionally ignored and must not be committed or bundled. Supported models are explicit verified user downloads; ordinary CI must remain weight-free.
