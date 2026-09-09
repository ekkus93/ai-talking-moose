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

## Post-review production semantics — 2026-09-09

The 2026-09-05 source review found several places where the implementation guarantees were stronger or narrower than the original V1 prose implied. The remediation tracker `docs/TODO(20260905-141500).md` is authoritative for those corrections. The resulting production semantics are:

### Request-scoped settings ownership

Typed and ambient text generation capture one immutable request settings snapshot before provider selection. Provider identity, provider-specific model ID, memory inclusion, prompt/personality inputs, and typed transcript-retention policy are derived from that snapshot. A concurrent settings write affects the next request, not half of the current request. Ambient delivery retains the separate post-generation current-state privacy suppression gate.

### Runtime diagnostics path

`LocalRuntimeManager::diagnostics()` is the authoritative runtime telemetry source. Production Tauri command `get_local_llm_diagnostics` composes installer and runtime diagnostics, carries them through the Rust-generated frontend contract and typed TypeScript bridge, and feeds the Local LLM Settings diagnostics surface. Diagnostics expose identity/state/performance and safe error categories only; prompt, output, memory, transcript, credential, filesystem-path, and raw native-error payloads are excluded.

### Installer phase and cancellation semantics

An authoritative in-flight installer record owns both the cancellation token and truthful phase. Active phases include `downloading`, `verifying`, and `promoting`. Large SHA-256 verification runs on a blocking worker and cooperatively checks cancellation per bounded chunk. Cancellation is checked again after verification and before promotion. A promoted artifact is not considered installed until the install marker is committed; accepted cancellation before that commit leaves no valid marker and cannot later turn the same operation into `installed`.

Every redirect target must remain HTTPS and the redirect count is bounded. Installer `last_error` chronology is explicit rather than inferred from unordered map iteration.

### Runtime-use artifact admission

Fast model-list/status refreshes use marker/shape validity and do not hash hundreds of megabytes. Before first runtime load in a process, the current GGUF bytes are rehashed against the pinned catalog SHA-256. Successful verification is cached only while a conservative file identity/fingerprint remains unchanged. A changed fingerprint forces rehash, and a mismatch fails closed before llama.cpp receives the artifact path. There is no fallback to Google, Fake, or another Local model.

### Persisted settings compatibility

`AppSettings::from_persisted_json()` inspects `settings_version` before destructive normalization. A persisted version newer than the application understands fails closed with a typed safe compatibility error. Unknown future fields are not normalized away and the rejected document is not overwritten with defaults or a downgraded schema.

### Frontend settings writes

Settings components submit patch intent rather than reconstructing complete `AppSettings` objects from render-time snapshots. The Zustand store owns optimistic/reconciled state and constructs the complete object sent to the Tauri persistence boundary. This prevents a stale component callback from overwriting unrelated newer settings while preserving ordered persistence, continuous-control coalescing, and rejected-write reconciliation.

### Generation cancellation ownership

The provider-neutral `TextModel::generate()` API has no application cancellation handle. Normal `LocalTextModel::generate()` requests create a private token used by the runtime's cooperative decode checks; typed and ambient callers cannot cancel that token. Model switch/delete are serialized with generation, and application shutdown rejects new work and waits only within the existing bounded teardown policy. This is deliberately not described as user-visible/native forced cancellation.

### Chat-template ownership

The application—not generic llama.cpp chat-template application—owns the supported SmolLM2 and Qwen ChatML rendering. The embedded GGUF template is a compatibility input used for deterministic family/signature validation. SmolLM2 system behavior and Qwen non-thinking framing are application-owned, and malformed/unsupported template signatures fail closed. P6 changed the ownership documentation and compatibility discriminator without changing rendered prompt framing, so no P12 real-model rerun was required.

### Remaining V1 limits

The Local runtime remains CPU-only by policy. Cooperative native generation cancellation remains limited to the safe high-level binding surface; shutdown is bounded rather than a forced native abort; first-token latency is still unavailable from the pinned runtime API and is recorded as unavailable rather than synthesized. Fully local voice remains deferred. Signed/notarized P13 release execution, physical Mac audio/TCC acceptance, and human voice audition remain owner-deferred and are not implied by ordinary CI.
