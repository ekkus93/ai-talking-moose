# External LLM Providers and System Voices — TODO

**Date:** 2026-10-13<br>
**Status:** Implementation in progress; acceptance and platform qualification outstanding<br>
**Specification:** `docs/EXTERNAL_LLM_SYSTEM_VOICES_SPEC_2026-10-13.md`

Check an item only after its implementation and named evidence exist. Keep the app-owned Local defaults and no-fallback rule throughout.

## Implementation checkpoint — 2026-10-14

The checklist remains intentionally unchecked until its full named acceptance evidence exists. Current `master` contains initial implementation in `src-tauri/src/ai/openai.rs`, `src-tauri/src/ai/loopback.rs`, `src-tauri/src/app/request_snapshot.rs`, `src-tauri/src/app/state.rs`, `src-tauri/src/secrets/store.rs`, and `src/components/Settings/AiTab.tsx`. These provide provider enum/settings scaffolding, independent OpenAI secret handling, text adapters, snapshot routing, and initial conditional UI. This is **not** evidence that all ELSV-100/200/300/400/600 acceptance tests pass.

The loopback adapter now rejects non-finite/out-of-range temperatures and zero-token requests before network I/O, with fake-server path/response and redirect-denial tests (commits `083c528`, `d133d96`, `9b54b31`). The earlier exact-head CI on `d133d96` found a Rust formatting failure; rustfmt corrections for the loopback adapter, module order, and settings command have since landed. OpenAI/secret-store formatting and full exact-head CI still require qualification. Do not mark verification complete based on partial or cancelled runs.

System voices are **not implemented**: the Rust TTS catalog and Voice & Audio selector still expose Google/Local only, and `TtsProvider::System` currently resolves to an unavailable synthesizer. ELSV-500 and the physical Mac audition remain open. Preserve this fail-closed behavior until the actual macOS buffer-synthesis and playback path is qualified.

## ELSV-100 — Settings and IPC foundations

- [ ] Add stable OpenAI, Ollama, and llama-server `TextProvider` values and System `TtsProvider` value across Rust, TypeScript, and generated contract.
- [ ] Add separate provider model IDs, loopback addresses, and system voice ID to settings; bump settings version; migrate old settings without changing saved Google/Local choices or wiping unknown future settings.
- [ ] Validate model IDs, server addresses, and provider combinations at the backend settings boundary; preserve request-scoped settings snapshots and ordered frontend patch writes.
- [ ] Add migration, serialization, defaults, invalid-input, and future-version tests; regenerate and review `src/generated/backendContract.json`.

**Acceptance:** fresh profiles remain Local/KittenTTS, existing profiles retain their choices, and frontend/backend settings shapes agree.

## ELSV-200 — OpenAI credentials and text adapter

- [ ] Extend secret backend, macOS Keychain identifiers, redaction, and Tauri commands for an independent OpenAI key; keep Linux key session-only and say so in the UI.
- [ ] Ensure key save/clear/has never reaches persisted settings, SQLite, frontend storage, logs, diagnostics, or another provider; clearing Google must leave OpenAI intact and vice versa.
- [ ] Implement bounded OpenAI Responses text requests from `TextRequest`, explicit model selection, output extraction, safe error categories, and no automatic retry/fallback.
- [ ] Add injected-transport tests for request shape, auth, no-key, invalid model, rate limit, timeout, oversized/malformed output, redaction, and provider isolation.

**Acceptance:** selected OpenAI text works with a saved key and failures cannot send the same request to another provider.

## ELSV-300 — Ollama and llama-server adapters

- [ ] Implement backend-only loopback-address validation for each configured server, including port, URL component, redirect, and effective-destination checks.
- [ ] Implement provider-specific chat-completions request/response adapters and bounded HTTP behavior; do not assume the two servers expose identical capabilities.
- [ ] Add explicit connection/model checks with truthful reachability versus model-readiness results; never launch a server or pull weights.
- [ ] Add fake-server tests for IPv4/IPv6/localhost, rejected remote/redirect URLs, model mismatch, malformed body, timeout, cancellation where available, error sanitization, and no cloud fallback.

**Acceptance:** an unavailable or incompatible server reports a clear error while remaining the selected provider.

## ELSV-400 — Route selected providers through app behavior

- [ ] Extend text-model factory for typed replies and ambient remarks using one immutable request snapshot.
- [ ] Refactor non-Live Talk to compose selected text model and selected standalone TTS, including Local, OpenAI, Ollama, llama-server, KittenTTS, Google TTS, and System Voices where supported.
- [ ] Keep Gemini Live audio mode and existing explicit Google Live behavior separate; show or reject unsupported combinations before Talk starts.
- [ ] Preserve transcript/privacy gates, generation identity, barge-in, cancellation, stale-response suppression, audio completion, and bubble lifetime.
- [ ] Test every supported routing combination and negative path, including cloud-text/local-TTS, local-text/cloud-TTS, unavailable model/voice, mid-request setting changes, and no unintended Gemini Live connection.

**Acceptance:** the provider shown in Settings is the one used by each relevant request; a selected failure never silently changes provider.

## ELSV-500 — macOS system TTS

- [ ] Add macOS runtime voice enumeration and stable voice IDs, including explicit System Default and unavailable saved voice behavior.
- [ ] Implement AVSpeechSynthesizer buffer synthesis and bounded PCM conversion into the existing Rust playback path; implement cancellation, timeout, no-audio, and format-error handling.
- [ ] Expose truthful voice capabilities, rate/pitch controls, and system-provider metadata through the TTS catalog/IPC; show unsupported status on Linux without a subprocess fallback.
- [ ] Add unit tests for catalog, selection, conversion, cancellation, unavailable voice, and Linux compile behavior; record a real Mac audition with audible completion and bubble-duration observation.

**Acceptance:** audition and non-Live Talk speak through the selected OS voice, and the bubble persists until playback ends or is cancelled.

## ELSV-600 — Control Panel and disclosure

- [ ] Extend AI & Models with five provider choices and conditional OpenAI/server controls, provider-specific connection states, and accurate data-destination labels.
- [ ] Extend Voice & Audio with System Voices, voice refresh/selection, Audition, capability-aware controls, and clear Live versus non-Live scope.
- [ ] Preserve tab-switch state, no-overlap auditions, active-Live-session behavior, accessibility, loading/error/empty states, and stale-patch safety.
- [ ] Update README, Privacy, onboarding, Local LLM architecture, and any relevant Settings help text to reflect cloud, loopback, Local, and Gemini Live boundaries.
- [ ] Add frontend tests for provider switching, key handling, server readiness, system voice availability, audition, and privacy disclosures.

**Acceptance:** the Control Panel makes the actual network and voice behavior understandable before the user selects a provider.

## ELSV-900 — Verification and closeout

- [ ] Run focused provider, routing, secret, settings migration, TTS, and frontend tests without live services or downloaded weights.
- [ ] Run `CARGO_BUILD_JOBS=2 npm run check:all`, `npm run check:generated-trees`, and `python3 scripts/check_local_llm_packaging_policy.py`; fix any drift without weakening a gate.
- [ ] Run supported macOS build/smoke checks and a physical Mac system-voice audition; record exact commands, results, limitations, and platform.
- [ ] Review generated contract, documentation, Git diff, and status; verify no API keys, GGUF files, generated trees, or audio artifacts entered the commit.
- [ ] Mark this TODO complete only when every acceptance criterion has evidence; record any intentionally deferred platform work separately.

**Final acceptance:** OpenAI, Ollama, llama-server, and macOS system voices are explicitly selectable, accurately disclosed, correctly routed, and validated without weakening Local defaults or privacy boundaries.
