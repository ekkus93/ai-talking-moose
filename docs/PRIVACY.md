# Talking Moose AI — Privacy & Security Architecture

## 1. Principles

1. **Local-first and data minimization.** Screen contents, OCR, keystrokes, clipboard contents, browser history, and arbitrary files are not collected by V1.
2. **Conservative fresh-profile defaults.** Active-application observation, cross-conversation memory, and transcript retention start **Off**. Users may explicitly enable each feature later.
3. **No silent cloud escalation.** A selected local capability never authorizes an implicit cloud substitute. Moonshine ASR failures do not upload microphone audio to Google, and Local text failures do not fall back to Google or Fake text generation.
4. **Explicit network actions.** Selecting a Local ASR/text model is configuration only. Missing model weights are downloaded only after an explicit user install action and integrity verification.
5. **Secure credentials.** On macOS, the Google API key is stored in the user's Keychain. It is not stored in SQLite, frontend state, or normal logs.
6. **Complete private-data reset.** Users can inspect and delete saved memories and can use **Forget Everything** to purge persisted memory/transcript data and reset transient derived observation state. Ordinary preferences, separately installed model files, and the separately managed Keychain credential are preserved.
7. **Microphone lifecycle is explicit.** The microphone is permitted only for a user-initiated conversation or an explicit diagnostics action. Mute, dismiss, stop, provider failure, and app shutdown must terminate capture; these lifecycle guarantees are release-blocking requirements.

## 2. Speech-recognition privacy

Talking Moose supports three ASR choices:

| ASR choice | Microphone audio | Transcript text |
| --- | --- | --- |
| Moonshine Tiny Streaming | Processed locally | Finalized user text may be sent to Gemini Live for the Moose's conversational response |
| Moonshine Small Streaming | Processed locally | Finalized user text may be sent to Gemini Live for the Moose's conversational response |
| Google Gemini Live Audio | Streamed to Google during the active conversation | Google processes the corresponding conversation content |

Moonshine therefore makes **speech recognition local**; it does not make the realtime conversation LLM or Google-generated voice response local. The application must communicate that distinction clearly anywhere the ASR mode is selected.

Moonshine Tiny Streaming is the default for a new profile. The model is downloaded only after an explicit user action; selecting a missing model is not permission to download it or switch to cloud ASR automatically.

## 3. Text-generation privacy

V1 has an independent text-provider setting for typed replies and ambient remarks:

| Text provider | Prompt/derived text | Model execution | Network during generation |
| --- | --- | --- | --- |
| Local | Remains on the computer for text generation | Pinned GGUF through the app-owned llama.cpp runtime | None required after installation |
| Google Gemini | Sent to Google as required for the selected Gemini text request | Google service | HTTPS request to Google |

A fresh profile defaults to **Local** text generation with SmolLM2 360M selected. The selection is not an installed-state claim and does not trigger a download. If the selected Local model is absent, corrupt, incompatible, or fails at runtime, text generation fails explicitly. It does not call Google and does not manufacture Fake success.

Local text generation has a separate, explicit installation network phase. **Download & Verify** makes an HTTPS request to the pinned catalog artifact URL, writes to staging, enforces the exact expected byte count, verifies SHA-256, and atomically promotes the artifact. After installation, the Local generation path does not need network I/O; the real CPU acceptance harness proves generation with network denied.

Local text does **not** make the entire Moose offline:

- Gemini Live remains the V1 spoken-conversation provider even when Text Generation is Local.
- Moonshine keeps microphone PCM local, but finalized Moonshine transcript text is still sent to Gemini Live for spoken conversation in this phase.
- Google TTS receives text when Google TTS is used.
- Selecting the Google text provider sends text-generation requests to Google.

A Google AI Studio key is therefore optional for Local typed/ambient text generation but still required for Google text, Gemini Live voice, and Google TTS.

## 4. Fresh-profile defaults

The V1 settings baseline for a newly created profile is:

- Text generation: **Local**
- Selected Local text model: **SmolLM2 360M Instruct Q4_K_M**
- Local text model automatically downloaded: **No**
- Active application observation: **Off**
- Window-title observation: **Off**
- Cross-conversation memory: **Off**
- Local transcript retention: **Off**
- ASR: **Moonshine Tiny Streaming**
- Raw-audio retention: **Never enabled in V1**

An upgraded profile keeps explicit pre-existing choices where migration can do so safely. Profiles created before the text-provider selector migrate explicitly to Google text generation so an upgrade does not silently reinterpret their provider. Profiles created before the ASR selector preserve the former Gemini Live audio behavior until the user explicitly changes ASR mode.

## 5. Logging policy

Normal application logs may contain operational metadata such as component names, model identifiers, revisions/quantization, byte counts, queue depths, durations, state transitions, token counts, throughput, and sanitized error categories. They must not contain:

- Google API credentials or credential-bearing URLs;
- user or Moose transcript/text-generation content;
- system prompts or full provider setup frames;
- raw Gemini Live frames;
- microphone PCM or base64-encoded audio;
- saved memory text;
- ambient desktop summaries or window titles;
- Local model prompt/output sentinels;
- sensitive tool arguments or tool results;
- raw native/provider errors containing private paths or request content.

Provider and Local-runtime failures cross application boundaries as structured/provider-neutral or otherwise fixed user-safe messages. Raw provider payloads, credential-bearing URLs, prompt/output text, local artifact paths, and native runtime error strings are not forwarded to the frontend or normal logs. Tool logging is limited to registered tool name and success/failure metadata rather than arguments, results, or raw error payloads.

## 6. Local persistence and model files

SQLite stores ordinary settings and, only when the user has opted in, semantic memory and transcript data. V1 desktop observations are transient and are never persisted; schema version 4 removes the legacy `observations` table. The generic settings repository rejects `google_api_key` so future callers cannot accidentally reintroduce plaintext credential persistence.

Local LLM weights are not stored in SQLite. They live under the application-data model root in an app-owned ID/revision-scoped layout. Installed-state markers bind the artifact to its catalog identity. Deleting a Local model removes model-owned data only and does not silently select or download a replacement.

When upgrading an old database that contains a plaintext `google_api_key`, startup migrates the key into secure storage, verifies the secure read-back, and only then deletes the SQLite row. A failed secure-store write leaves the old row in place so migration does not destroy the user's only copy; the application treats the migration failure as an error rather than pretending it succeeded.

## 7. Packaging and third-party model boundaries

GGUF model weights are user-initiated downloads and are not committed to Git or embedded in application bundles. Ordinary CI and release/package jobs remain model-weight-free. macOS bundle verification fails if a `.gguf` is embedded.

The native llama.cpp/binding licenses are shipped as runtime/dependency notices. Downloadable model license/source metadata is recorded separately in `docs/LOCAL_LLM_MODEL_LICENSES.md` because those weights are not redistributed in the application bundle.

## 8. User controls

The Settings privacy/AI surfaces must explain which features are local, which actions perform a download, which data can leave the device, and which persistence controls are Off by default. Onboarding must make the same boundaries clear before asking the user to enable observation, memory, transcript retention, cloud microphone processing, or Google-backed text/voice/TTS.

Privacy-sensitive settings are runtime policy, not cosmetic preferences. A disabled setting or unavailable selected provider must cause the corresponding backend operation to fail closed.

See `docs/LOCAL_LLM_ARCHITECTURE.md` for the Local text runtime/provider boundary and the explicitly deferred fully local voice seam.
