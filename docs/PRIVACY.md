# Talking Moose AI — Privacy & Security Architecture

## 1. Principles

1. **Local-first and data minimization.** Screen contents, OCR, keystrokes, clipboard contents, browser history, and arbitrary files are not collected by V1.
2. **Conservative fresh-profile defaults.** Active-application observation, cross-conversation memory, and transcript retention start **Off**. Users may explicitly enable each feature later.
3. **No silent cloud escalation.** A selected local capability never authorizes an implicit cloud substitute. Moonshine ASR failures do not upload microphone audio to Google, Local text failures do not fall back to Google or Fake text generation, and Local TTS failures do not fall back to Google TTS.
4. **Explicit network actions.** Selecting a Local ASR/text/TTS model is configuration only. Missing model weights or Local TTS runtime artifacts are downloaded only after an explicit user install action and integrity verification.
5. **Independent credentials.** On macOS, Google and OpenAI API keys use separate Keychain entries. On Linux, both keys are session-only. Neither key belongs in SQLite, persisted settings, browser storage, or normal logs.
6. **Complete private-data reset.** Users can inspect and delete saved memories and can use **Forget Everything** to purge persisted memory/transcript data and reset transient derived observation state. Ordinary preferences, separately installed model files, and the separately managed Keychain credential are preserved.
7. **Microphone lifecycle is explicit.** The microphone is permitted only for a user-initiated conversation or an explicit diagnostics action. Mute, dismiss, stop, provider failure, and app shutdown must terminate capture; these lifecycle guarantees are release-blocking requirements.

## 2. Speech-recognition privacy

Talking Moose supports four ASR choices:

| ASR choice | Microphone audio | Transcript text |
| --- | --- | --- |
| Moonshine Tiny Streaming | Processed locally | Finalized text goes only to the explicitly selected text provider (Local, Google, OpenAI, Ollama, or llama-server) |
| Moonshine Small Streaming | Processed locally | Finalized text goes only to the explicitly selected text provider (Local, Google, OpenAI, Ollama, or llama-server) |
| Local Whisper.cpp Small | Processed locally on the host | Finalized text goes only to the explicitly selected text provider (Local, Google, OpenAI, Ollama, or llama-server) |
| Google Gemini Live Audio | Streamed to Google during the active conversation | Google processes the corresponding conversation content |

Moonshine and local Whisper.cpp keep speech recognition local. Local-first Talk also keeps finalized transcript text, generation, and generated speech local after selected models are installed. Selecting Google or OpenAI text generation explicitly sends finalized transcript text to that cloud provider; selecting Ollama or llama-server sends it to the configured loopback server. Only explicit Gemini Live audio ASR streams microphone audio to Google. Local partial transcript updates stay on the host.

Local Whisper.cpp uses a pinned whisper.cpp source commit and a user-downloaded, SHA-verified `ggml-small` model file. Whisper microphone audio remains local. Whisper installation is explicit and separate from conversation start; an active Whisper installation can be canceled from Settings.

If the selected Whisper model is absent, corrupt, unsupported, or fails at runtime, local ASR fails explicitly. It does not fall back to Gemini Live, Moonshine, Fake, or any other cloud provider.

The platform's local ASR mode is the default for a new profile. Models are downloaded only after an explicit user action; selecting a missing model is not permission to download it or switch to cloud ASR automatically.

## 3. Text-generation privacy

Typed replies and ambient remarks use an explicit text-provider selection. The OpenAI, Ollama, and llama-server adapters are implemented; macOS System Voices are not yet available, and full platform acceptance is still being recorded. A failed request never silently switches to another text provider.

| Text provider | Prompt/derived text | Model execution | Network during generation |
| --- | --- | --- | --- |
| Local | Remains on this computer for generation | App-owned pinned GGUF through llama.cpp | None required after installation |
| Google Gemini | Sent to Google for the selected text request | Google service | HTTPS to Google |
| OpenAI API | Sent to OpenAI using the separately saved OpenAI key | OpenAI service | HTTPS to OpenAI |
| Ollama | Sent to the user-managed loopback server | Independently managed local process | HTTP to an explicitly configured loopback address |
| llama-server | Sent to the user-managed loopback server | Independently managed local process | HTTP to an explicitly configured loopback address |

Loopback providers accept only explicit HTTP localhost/127.0.0.1/::1 addresses with a port. They do not launch servers or download model weights. Those independent servers may log or retain prompts according to their own configuration. A failed request must not retry through another provider, silently select a different model, or forward content to Google or OpenAI.

A fresh profile defaults to **Local** text generation with SmolLM2 360M and **Local** TTS with KittenTTS selected. These selections are not installed-state claims and do not trigger downloads. If a selected Local model is absent, corrupt, incompatible, or fails at runtime, the operation fails explicitly; it does not call Google or manufacture Fake success. Talk uses local ASR, Local text generation, and KittenTTS by default. Gemini Live is used only when Gemini Live audio ASR is explicitly selected. Choosing Google text generation with local ASR sends finalized transcript text to Google for non-Live Talk, without streaming microphone audio through Gemini Live.

Local text generation has a separate, explicit installation network phase. **Download & Verify** makes an HTTPS request to the pinned catalog artifact URL, writes to staging, enforces the exact expected byte count, verifies SHA-256, and atomically promotes the artifact. After installation, the Local generation path does not need network I/O; the real CPU acceptance harness proves generation with network denied.

Local text does **not** make the entire Moose offline:

- Local-first Talk uses local ASR, the selected Local text model, and KittenTTS.
- Gemini Live is used only when Gemini Live audio ASR is explicitly selected. Choosing Google text generation with local ASR sends finalized transcript text to Google for non-Live Talk, without streaming microphone audio through Gemini Live.
- Google TTS receives text when Google TTS is used.
- Selecting the Google text provider sends text-generation requests to Google.

A Google AI Studio key is optional for Local, OpenAI, and loopback typed/ambient text generation, but required for Google text, Gemini Live voice, and Google TTS. An OpenAI key is required only when OpenAI text is selected. Google and OpenAI credentials are independent and must never be sent to loopback servers.

### Idle Banter prompt and provider privacy

Idle Banter is unsolicited ambient text generation with an explicit user-configured schedule. It uses the currently selected text provider and never silently falls back to another provider. The built-in or user-edited Idle Banter seed topic selected for an occurrence is prompt input: with Local text generation it remains on the computer after model installation; with Google text generation it is sent to Google as part of that request.

Idle Banter V1 does **not** automatically inject recent conversation transcript text merely to make remarks topical. If the shared ambient prompt path includes saved memory context, the existing `memory_enabled` setting remains authoritative; disabling memory prevents retained memories from entering the Idle Banter prompt. Idle Banter event metadata itself is limited to bounded Moose-interaction inactivity duration and the configured seed topic rather than window titles or application names.

To reduce repetition, the runtime keeps at most the 12 most recent successfully delivered Idle Banter lines in memory for the current app session. That generated-text history is not persisted to SQLite and is cleared on restart. Normal logs must not contain Idle Banter prompt bodies, seed-topic contents, recent generated lines, user transcript text, or memory content.

Idle Banter can be disabled independently in Behavior settings and is also disabled whenever the global unsolicited-comments master switch is off. Existing mute, active-conversation, quiet-hours, annoyance, dismissal, cooldown, and hourly-rate gates remain authoritative.

## 4. Standalone speech-output privacy

Standalone TTS for typed replies, ambient remarks, canned reactions, and auditions is separate from Gemini Live voice conversations. macOS System Voices are planned but not yet implemented or qualified; selecting System currently fails closed rather than silently playing another voice.

| Standalone speech provider | Text sent for synthesis | Model execution | Network during synthesis | Fallback behavior |
| --- | --- | --- | --- | --- |
| Local KittenTTS | Remains on the computer for synthesis | Pinned KittenTTS artifacts through the app-owned CPU ONNX Runtime path | None required after installation | Fails explicitly; does not call Google TTS |
| Google Gemini TTS | Sent to Google as required for the selected Gemini TTS request | Google service | HTTPS request to Google | Fails explicitly; does not call Local TTS |
| macOS System Voices (not yet implemented) | Intended to remain on the Mac | Planned AVSpeechSynthesizer buffer path | No generation network request intended | Currently unavailable; no subprocess or other-provider fallback |

Local KittenTTS has a separate installation network phase. **Download & Verify** fetches the pinned KittenTTS model/runtime artifacts, writes to staging, verifies expected byte counts and SHA-256, and promotes only verified assets. After installation, Local TTS synthesis does not require network I/O; the real KittenTTS CPU acceptance workflow proves synthesis with network denied.

Local TTS diagnostics may expose operational metadata such as provider, selected model ID, selected voice ID, install state, expected/installed bytes, runtime phase, sample rate, inference thread count, sanitized error category, timings, and real-time factor. Diagnostics must not expose utterance text, raw PCM/audio bytes, credentials, credential-bearing URLs, arbitrary filesystem paths, or raw native/provider error payloads.

Local Talk synthesizes replies with KittenTTS and does not require network access after its models are installed. Gemini Live remains a separate opt-in cloud conversation path.

## 5. Fresh-profile defaults

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

An upgraded profile keeps explicit pre-existing choices where migration can do so safely. Profiles without a saved text-provider selector migrate to Local text generation; a saved explicit Google selection remains Google. Profiles created before the ASR selector preserve the former Gemini Live audio behavior until the user explicitly changes ASR mode.

## 6. Logging policy

Normal application logs may contain operational metadata such as component names, model identifiers, revisions/quantization, byte counts, queue depths, durations, state transitions, token counts, throughput, and sanitized error categories. They must not contain:

- Google or OpenAI API credentials or credential-bearing URLs;
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

## 7. Local persistence and model files

SQLite stores ordinary settings and, only when the user has opted in, semantic memory and transcript data. V1 desktop observations are transient and are never persisted; schema version 4 removes the legacy `observations` table. The generic settings repository rejects `google_api_key` so future callers cannot accidentally reintroduce plaintext credential persistence.

Local LLM weights are not stored in SQLite. They live under the application-data model root in an app-owned ID/revision-scoped layout. Installed-state markers bind the artifact to its catalog identity. Deleting a Local model removes model-owned data only and does not silently select or download a replacement.

When upgrading an old database that contains a plaintext `google_api_key`, startup migrates the key into secure storage, verifies the secure read-back, and only then deletes the SQLite row. A failed secure-store write leaves the old row in place so migration does not destroy the user's only copy; the application treats the migration failure as an error rather than pretending it succeeded.

## 8. Packaging and third-party model boundaries

GGUF model weights are user-initiated downloads and are not committed to Git or embedded in application bundles. Ordinary CI and release/package jobs remain model-weight-free. macOS bundle verification fails if a `.gguf` is embedded.

The native llama.cpp/binding licenses are shipped as runtime/dependency notices. Downloadable model license/source metadata is recorded separately in `docs/LOCAL_LLM_MODEL_LICENSES.md` because those weights are not redistributed in the application bundle.

## 9. User controls

The Settings privacy/AI surfaces must explain which features are local, which actions perform a download, which data can leave the device, and which persistence controls are Off by default. Onboarding must make the same boundaries clear before asking the user to enable observation, memory, transcript retention, cloud microphone processing, or Google-backed text/voice/TTS.

Privacy-sensitive settings are runtime policy, not cosmetic preferences. A disabled setting or unavailable selected provider must cause the corresponding backend operation to fail closed.

See `docs/LOCAL_LLM_ARCHITECTURE.md` for the Local text runtime/provider boundary and `docs/KITTENTTS_CLOSEOUT_EVIDENCE_2026-09-12.md` for Local KittenTTS acceptance evidence and the deliberately deferred human voice-audition gate.
