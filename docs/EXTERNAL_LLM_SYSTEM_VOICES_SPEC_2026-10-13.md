# External LLM Providers and System Voices — Specification

**Date:** 2026-10-13<br>
**Status:** Proposed<br>
**Companion TODO:** `docs/EXTERNAL_LLM_SYSTEM_VOICES_TODO_2026-10-13.md`

## Goal

Let people choose OpenAI API, Ollama, or llama-server for text generation, and use installed macOS voices for text-to-speech (TTS). Keep the app-owned Local LLM and KittenTTS as fresh-profile defaults. Each choice must be explicit, truthful about where data goes, and usable from the existing Control Panel.

## Current seams and constraints

- `TextModel` and `SpeechSynthesizer` in `src-tauri/src/ai/traits.rs` are provider-neutral. `AppSettings` in `src-tauri/src/app/state.rs` holds provider/model choices; `src-tauri/src/app/request_snapshot.rs` chooses text models.
- `src/components/Settings/AiTab.tsx` owns AI & Models UI; `VoiceTab.tsx` and `src-tauri/src/ai/tts_catalog.rs` own standalone voice choices. Settings cross Rust/TypeScript through the generated backend contract and `tauriBridge`.
- Typed replies and ambient remarks already use the selected text model. Today `get_conversation_provider_for` selects non-Live Talk only for Local text plus non-Gemini ASR, and constructs KittenTTS directly. This routing must change for new choices to work in Talk.
- Gemini Live is a distinct audio conversation path. It must remain labeled as such; choosing an external text model or system TTS must not silently send audio or transcript text to Gemini Live.
- The existing secret backend persists Google credentials in macOS Keychain but keeps them only in memory on Linux. An OpenAI key needs the same explicit platform semantics, with no secret stored in SQLite, settings JSON, frontend persistence, logs, or diagnostics.

## Product behavior

### Text-provider choices

Add stable `TextProvider` values for `openai`, `ollama`, and `llama_server`, retaining `local` and `google`. Existing saved choices and provider-specific model settings survive switching providers. New profiles remain `local`; missing provider fields migrate to `local`; a saved Google choice remains Google. Bump settings version and preserve the existing fail-closed handling of settings written by a newer app.

| Choice in AI & Models | Configuration | Data boundary |
| --- | --- | --- |
| Built-in Local | Existing app-owned catalog and explicit verified download | Inference on this computer; no generation network request |
| Google Gemini | Existing Google key and model controls | Text sent to Google |
| OpenAI API | Saved API key and explicit model ID | Text sent to OpenAI |
| Ollama | Server address and exact model ID | Text sent to the configured server |
| llama-server | Server address and exact model ID | Text sent to the configured server |

OpenAI uses the official Responses API with an explicit text-only request/response mapping. Ollama and llama-server use their documented OpenAI-compatible chat-completions endpoints where supported. Share bounded HTTP plumbing if useful, but keep provider-specific request, response, error, and capability handling: compatibility is not assumed to be identical. Version-specific API details must be checked against current official documentation during implementation.

The initial server-address policy is **loopback only**: default Ollama to `http://127.0.0.1:11434`, default llama-server to `http://127.0.0.1:8080`, permit explicit `http://127.0.0.1`, `http://[::1]`, or `http://localhost` with a valid port, and reject credentials, path/query/fragment, non-loopback hosts, redirects, and unexpected schemes. The backend validates the effective destination on every request, including connection tests; the frontend never supplies a complete request URL. LAN/remote servers require a later explicit design for authentication, TLS, and privacy disclosure. A loopback server is independently managed by the user: choosing it never starts a process or downloads a model. A reachable server with a missing model reports a model error and does not substitute another model.

Every adapter maps the shared `TextRequest` fields into supported provider parameters without leaking an unsupported parameter or misrepresenting its effect. Bound input/output size, response body size, request duration, and model-ID length; reject empty or malformed output; classify connection, authentication, rate-limit, model, timeout, cancellation, and malformed-response failures into sanitized user-facing errors. Do not log prompt, response, transcript, memory, credential, raw URL query, or raw server error body. Do not retry a generation request in a way that can duplicate an answer, and never fall back to Google, Fake, or another model. Retain the request-scoped settings snapshot so a settings change affects the next request.

### Talk routing

For non-Gemini local ASR, Talk uses the selected `TextModel` and selected `SpeechSynthesizer`: Local, OpenAI, Ollama, or llama-server text may pair with KittenTTS, Google standalone TTS, or system TTS. Preserve conversation generation IDs, barge-in, cancellation, speech-bubble duration, and stale-response suppression. A selected cloud text provider receives finalized transcript text and the existing prompt context, not microphone PCM. Cloud TTS separately receives the generated speech text only when selected.

If Gemini Live audio ASR is selected, Talk uses the existing Gemini Live audio pipeline and its Live voice. AI & Models and Voice & Audio must say that the selected text and standalone TTS providers do not control this Live session. Preserve the current legacy Google-text/Live behavior unless a separate explicit mode change is designed; do not silently route a new provider to Live. Test every text/ASR/TTS combination or reject unsupported combinations before starting Talk with a clear message. Typed replies and ambient remarks always use the selected text provider independently of the Live setting.

### System voices

Add `system` to `TtsProvider` and expose installed macOS voices through a runtime voice catalog. Use stable OS voice identifiers for persistence and display language/name/quality where available. Include a clearly labeled “System Default” choice. Refresh availability when the panel opens; an unavailable saved voice remains visible as unavailable and does not silently become another voice. The user may explicitly select System Default. Do not trigger an OS voice download from provider selection or audition.

Implement the macOS adapter with `AVSpeechSynthesizer` buffer synthesis. Convert the returned format to bounded mono PCM with an honest sample rate, then feed the existing Rust audio queue so device choice, volume, cancellation, mouth state, and bubble completion share the normal lifecycle. Do not implement direct independent OS playback that could finish after the app hides the bubble. Set rate only within the OS-supported range. Show pitch only if the adapter supports and tests it truthfully; preserve but do not apply a Google-only pitch setting otherwise. Empty text, missing voice, no buffers, conversion failure, timeout, and cancellation must terminate cleanly. Do not retain synthesized speech on disk.

The first release is **macOS system voices only**. On Linux, the System option is hidden or disabled with an explicit unsupported-platform message, while Local and Google TTS keep working. A Linux speech engine is a separate follow-up; no `espeak`/Speech Dispatcher subprocess fallback is part of this release.

### Control Panel

- **AI & Models:** Show five text-provider choices. Keep each provider's controls in a conditional panel. OpenAI has key save/remove/presence status and model ID. Ollama and llama-server each have address, model ID, and a provider-specific Test Connection action. State whether the selected destination is app-owned local, another local server, or cloud. Connection tests are explicit, bounded, and do not create a conversation, download weights, or change selection. Report reachability and model availability separately where the server API allows; do not claim generation works solely because a port is open.
- **Voice & Audio:** Add System Voices next to Local KittenTTS and Google TTS in standalone speech. Show available voice picker, system-default option, rate control, and Audition. Use the existing audition cancellation/debounce and no-overlap behavior. State when this choice applies to non-Live Talk and when Gemini Live uses its own voice. Preserve existing Local install UI and Google settings.
- Use accessible labels, loading/empty/error states, and stable controls during tab changes. Provider switching must not interrupt an active Live session; standalone auditions must not overlap. Existing saved settings should not be overwritten by stale render-time patches.

## Secret storage and privacy

Extend the secret abstraction with an independent OpenAI API key, including set/clear/has operations and redaction. On macOS persist it in Keychain under a separate service/account from Google. On Linux, where the current backend is memory-only, the OpenAI key is **session-only** until an OS-backed secret backend is separately implemented; label this behavior in Settings. Never persist a key in `AppSettings`, SQLite, the generated contract, browser storage, or error text. Clearing one provider's key must not erase the other's. Never send an OpenAI key to Ollama or llama-server.

Update `README.md`, `docs/PRIVACY.md`, onboarding, Settings copy, and `docs/LOCAL_LLM_ARCHITECTURE.md` to distinguish app-owned Local, loopback server providers, OpenAI/Google cloud, selected TTS, and Gemini Live. Explain that loopback servers may have their own logs/storage policies and that a selected provider's failure causes an explicit error, not an automatic transfer to another provider. Keep all model weights out of Git and app bundles.

## Verification and acceptance

1. Settings migration/serialization and Rust-derived frontend contract round-trip all five text and three TTS values, preserve old selections and per-provider fields, and reject future versions as before.
2. Injected HTTP-server tests prove exact destination, auth isolation, model/parameter mapping, timeout/size/error handling, redirect denial, loopback policy, secret-free logs, and no fallback. No live cloud key or model download is needed for ordinary CI.
3. Routing tests prove typed, ambient, non-Live Talk, and Gemini Live boundaries; every supported non-Live text/TTS pairing is selected from the saved snapshot. Failure and provider switch cannot replay or misattribute a speech bubble.
4. macOS adapter tests cover voice enumeration/selection, format conversion, bounded synthesis, cancellation, unavailable voice, and completion; a physical Mac audition checks audible output and bubble lifetime. Linux compile/tests prove truthful unsupported behavior.
5. Frontend tests cover provider-specific controls, key presence/clear, connection-test states, system voice list/unavailable state, audition, and Live disclosure. Contract and privacy copy match the implemented behavior.
6. Before closeout run `CARGO_BUILD_JOBS=2 npm run check:all`, packaging-policy and generated-tree checks, and required macOS build/smoke checks. Keep normal CI free of live credentials, model downloads, and audio-device requirements. Record exact results, including any manual Mac evidence; do not infer success from compilation alone.

## Non-goals for this release

- OpenAI realtime audio, OpenAI TTS/ASR, streaming-token UI, or automatic provider/model failover.
- Managing Ollama/llama-server processes, pulling their models, or connecting to remote/LAN servers.
- Linux system voices, bundled OS voices, or changing the Gemini Live voice engine.

## Reference APIs

- OpenAI Responses: https://developers.openai.com/api/reference/resources/responses/methods/create
- Ollama OpenAI compatibility: https://docs.ollama.com/api/openai-compatibility
- llama-server: https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
- Apple AVSpeechSynthesizer: https://developer.apple.com/documentation/AVFAudio/AVSpeechSynthesizer
