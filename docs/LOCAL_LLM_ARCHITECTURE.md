# Local LLM V1 Architecture

## Scope

Local LLM V1 adds CPU-friendly **text generation** alongside Google Gemini. It covers:

- typed `send_text_message` replies;
- ambient remarks;
- explicit Local-model self-test generation.

It does **not** replace the V1 spoken-conversation provider. Gemini Live remains the realtime voice provider, including for finalized transcripts produced by local Moonshine ASR. Google TTS also remains a separate cloud speech-output path when selected.

The design deliberately separates text-provider choice from ASR, realtime voice, and TTS so a Local text selection cannot silently redefine those privacy boundaries.

## Provider selection

`AppSettings.text_provider` is the authoritative text-provider choice:

- `local` — `LocalTextModel` backed by the app-owned Local runtime;
- `google` — `GoogleTextModel` backed by the configured Gemini text model.

Fresh profiles default to **Local** with `smollm2-360m-instruct-q4-k-m` selected. Selection does not install or download the model. Profiles created before the text-provider selector continue to migrate explicitly to Google so an upgrade does not silently change an existing user's text provider.

Ambient and typed-message call sites continue to depend on the provider-neutral `TextModel` trait. They do not contain provider-specific fallback branches.

```text
Typed message / ambient event
            |
            v
      TextModel selection
        /           \
       /             \
   Local              Google
     |                  |
LocalTextModel      GoogleTextModel
     |                  |
LocalRuntimeManager    Gemini REST
     |
 pinned llama.cpp / ggml
```

There is no Local -> Google -> Fake fallback chain. If the selected provider cannot satisfy the request, the request fails with a sanitized explicit error.

## Local model catalog and installer

The executable catalog is `src-tauri/src/ai/local/catalog.rs`. Every supported artifact is app-owned metadata containing:

- stable app model ID;
- display name and family;
- parameter scale and quantization;
- exact artifact filename;
- HTTPS source URL containing an immutable 40-hex revision;
- exact expected byte count;
- SHA-256;
- license metadata;
- context and output bounds;
- runtime chat-template hint.

The Local model root is initialized from Tauri's application-data directory at:

```text
<app-data>/models/llm/
```

Installed artifacts are revision-scoped beneath the app-owned model ID. The installer validates catalog-owned paths and rejects unsafe/symlinked storage layouts rather than following them outside the model root.

A model download requires an explicit user action. The production installer:

1. creates a unique staging file;
2. streams the HTTPS response with a hard byte bound;
3. supports cancellation and reports progress;
4. verifies exact expected bytes;
5. verifies the pinned SHA-256;
6. atomically promotes the artifact into its revision-scoped directory;
7. writes an install marker tying the installed file to the catalog identity.

A failed, cancelled, corrupt, or incomplete install does not become `installed` and does not authorize another provider to take over.

## Runtime manager

`AppState` owns an `Arc<LocalRuntimeManager>`. Binding-specific llama.cpp model/context/sampler types remain inside `src-tauri/src/ai/local/runtime/`; they do not cross into Tauri commands, frontend IPC types, or general application state.

The runtime manager is responsible for:

- lazy loading the selected verified artifact;
- CPU-only model/context configuration;
- bounded context and output policy;
- conservative thread selection;
- serialized generation unless concurrency is explicitly proven safe;
- model identity tracking across requests;
- unload/reload when the selected model changes;
- coordinating deletion with active inference;
- bounded application shutdown;
- prompt/output-free diagnostics such as loaded model ID, duration, token counts, and sanitized error category.

`LocalTextModel` adapts the provider-neutral `TextRequest` into a local runtime request and returns a provider-neutral `TextResponse`. Chat control tokens are not assembled at ambient or conversation call sites. Supported families use the app's validated family/template policy; incompatible template metadata fails explicitly.

## Network and privacy boundaries

There are two distinct Local-model network phases:

| Operation | Network behavior |
| --- | --- |
| Select Local provider/model | No download |
| Inspect installed status | Local filesystem only |
| **Download & Verify** | Explicit HTTPS request to the pinned catalog artifact URL |
| Local generation after installation | No network required |
| Delete/unload/test installed model | Local-only, except the test naturally uses the already installed model |

Real CPU acceptance proves generation after installation while the process is in a denied-network namespace. The Local generation path does not construct the Google REST client, Gemini Live WebSocket, or installer request.

Local text generation does **not** imply that the whole application is offline:

- Gemini Live voice remains cloud-based in V1;
- Moonshine Tiny/Small keep microphone PCM local, but finalized transcript text is still handed to Gemini Live for spoken conversation;
- Google TTS receives text when Google TTS is used;
- Google text generation is available when the user explicitly selects the Google text provider.

These boundaries are documented in `docs/PRIVACY.md` and must stay synchronized with Settings/onboarding copy.

## Voice architecture and deferred seam

Current V1 voice path:

```text
Microphone
   |
   +--> Gemini Live Cloud Audio -------------------------+
   |                                                     |
   +--> Moonshine local ASR --> finalized transcript ----+--> Gemini Live
                                                          |
                                                          v
                                                   response audio
                                                          |
                                                          v
                                                   Rust playback
```

Local text selection is intentionally absent from that path.

A later fully local voice phase may introduce an explicit seam such as:

```text
Microphone -> Moonshine ASR -> Local TextModel -> speech synthesizer -> playback
```

That is **deferred work**, not a hidden fallback or partially enabled V1 path. It will require an explicit conversation-provider/lifecycle design so interruption, streaming, tool calls, memory, TTS privacy, and error semantics remain coherent.

## Packaging and licensing

The application builds the pinned `llama-cpp-2` / `llama-cpp-sys-2` `0.1.154` dependencies with default features disabled. No developer-local/system llama.cpp installation is part of the supported build contract.

GGUF weights are not Git content and are not application-bundle resources. The macOS bundle verifier fails if a `.gguf` is embedded. Runtime/binding license notices are packaged separately from model license metadata; see:

- `docs/THIRD_PARTY_NOTICES.md`;
- `docs/LOCAL_LLM_MODEL_LICENSES.md`;
- `src-tauri/native/macos/notices/LocalLlmRuntime/`.

## Verification boundaries

Ordinary CI is intentionally model-weight-free. It proves source quality, contracts, tests, packaging policy, supported-target native compilation, license inventory, and macOS bundle construction without downloading third-party GGUFs.

Heavy acceptance is manual and evidence-producing:

- **Local LLM Real CPU Acceptance** — `.github/workflows/local-llm-real-cpu-acceptance.yml`; downloads through the production installer, verifies the pinned artifact, then runs generation with network denied.
- **Local LLM P13 Packaging Acceptance** — `.github/workflows/local-llm-p13-packaging-acceptance.yml`; compares current and fixed pre-llama macOS bundles on arm64 and x86_64 and rejects embedded GGUFs.

Canonical P12 and P13 evidence is recorded in:

- `docs/LOCAL_LLM_CPU_ACCEPTANCE_20260902.md`;
- `docs/RECONCILIATION_LOCAL_LLM_P13_20260902.md`.
