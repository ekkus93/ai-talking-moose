# Talking Moose AI — Privacy & Security Architecture

## 1. Principles

1. **Local-first and data minimization.** Screen contents, OCR, keystrokes, clipboard contents, browser history, and arbitrary files are not collected by V1.
2. **Conservative fresh-profile defaults.** Active-application observation, cross-conversation memory, and transcript retention start **Off**. Users may explicitly enable each feature later.
3. **No silent cloud escalation.** Selecting a local Moonshine ASR model must never silently fall back to uploading microphone audio to Google when the model is missing, corrupt, overloaded, or unavailable.
4. **Secure credentials.** On macOS, the Google API key is stored in the user's Keychain. It is not stored in SQLite, frontend state, or normal logs.
5. **Complete local erasure.** Users can inspect and delete saved memories and can use **Forget Everything** to purge local memory, transcript, and observation data.
6. **Microphone lifecycle is explicit.** The microphone is permitted only for a user-initiated conversation or an explicit diagnostics action. Mute, dismiss, stop, provider failure, and app shutdown must terminate capture; these lifecycle guarantees are release-blocking requirements.

## 2. Speech-recognition privacy

Talking Moose supports three ASR choices:

| ASR choice | Microphone audio | Transcript text |
| --- | --- | --- |
| Moonshine Tiny Streaming | Processed locally | Finalized user text may be sent to Gemini Live for the Moose's conversational response |
| Moonshine Small Streaming | Processed locally | Finalized user text may be sent to Gemini Live for the Moose's conversational response |
| Google Gemini Live Audio | Streamed to Google during the active conversation | Google processes the corresponding conversation content |

Moonshine therefore makes **speech recognition local**; it does not make the LLM or Google-generated voice response local. The application must communicate that distinction clearly anywhere the ASR mode is selected.

Moonshine Tiny Streaming is the default for a new profile. The model is downloaded only after an explicit user action; selecting a missing model is not permission to download it or switch to cloud ASR automatically.

## 3. Fresh-profile defaults

The V1 settings baseline for a newly created profile is:

- Active application observation: **Off**
- Window-title observation: **Off**
- Cross-conversation memory: **Off**
- Local transcript retention: **Off**
- ASR: **Moonshine Tiny Streaming**
- Raw-audio retention: **Never enabled in V1**

An upgraded profile keeps explicit pre-existing choices where migration can do so safely. Profiles created before the ASR selector preserve the former Gemini Live audio behavior until the user explicitly changes ASR mode.

## 4. Logging policy

Normal application logs may contain operational metadata such as component names, model identifiers, byte counts, queue depths, durations, state transitions, and sanitized error categories. They must not contain:

- Google API credentials or credential-bearing URLs;
- user or Moose transcript text;
- system prompts or full provider setup frames;
- raw Gemini Live frames;
- microphone PCM or base64-encoded audio;
- saved memory text;
- window titles or desktop observation text;
- sensitive tool arguments or tool results.

Provider failures cross application boundaries as structured, provider-neutral categories with fixed user-safe messages; raw provider payloads, credential-bearing URLs, and transport error strings are not forwarded to the frontend or normal logs. Tool logging is limited to the registered tool name and success/failure metadata rather than arguments, results, or raw error payloads.

## 5. Local persistence

SQLite stores ordinary settings and, only when the user has opted in, memory/transcript/observation data. The generic settings repository rejects `google_api_key` so future callers cannot accidentally reintroduce plaintext credential persistence.

When upgrading an old database that contains a plaintext `google_api_key`, startup migrates the key into secure storage, verifies the secure read-back, and only then deletes the SQLite row. A failed secure-store write leaves the old row in place so migration does not destroy the user's only copy; the application treats the migration failure as an error rather than pretending it succeeded.

## 6. User controls

The Settings privacy surface must explain which features are local, which data can leave the device, and which persistence controls are Off by default. Onboarding must make the same defaults clear before asking the user to enable additional observation, memory, transcript retention, or cloud microphone processing.

Privacy-sensitive settings are runtime policy, not cosmetic preferences. A disabled setting must cause the corresponding backend operation to fail closed.
