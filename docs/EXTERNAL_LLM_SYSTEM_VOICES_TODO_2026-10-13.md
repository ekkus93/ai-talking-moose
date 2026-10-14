# External LLM Providers and System Voices — TODO

**Date:** 2026-10-13<br>
**Status:** Implementation in progress; acceptance and platform qualification outstanding<br>
**Specification:** `docs/EXTERNAL_LLM_SYSTEM_VOICES_SPEC_2026-10-13.md`

Check an item only after its implementation and named evidence exist. Keep the app-owned Local defaults and no-fallback rule throughout.

## Implementation checkpoint — 2026-10-14

The checklist remains intentionally unchecked until its full named acceptance evidence exists. Current `master` contains initial implementation in `src-tauri/src/ai/openai.rs`, `src-tauri/src/ai/loopback.rs`, `src-tauri/src/app/request_snapshot.rs`, `src-tauri/src/app/state.rs`, `src-tauri/src/secrets/store.rs`, and `src/components/Settings/AiTab.tsx`. These provide provider enum/settings scaffolding, independent OpenAI secret handling, text adapters, snapshot routing, and initial conditional UI. This is **not** evidence that all ELSV-100/200/300/400/600 acceptance tests pass.

The loopback adapter now rejects non-finite/out-of-range temperatures and zero-token requests before network I/O, with fake-server path/response and redirect-denial tests (commits `083c528`, `d133d96`, `9b54b31`). The earlier exact-head CI on `d133d96` found a Rust formatting failure; rustfmt corrections for the loopback adapter, module order, and settings command have since landed. OpenAI/secret-store formatting and full exact-head CI still require qualification. Do not mark verification complete based on partial or cancelled runs.

System voices are **not implemented**: the Rust TTS catalog and Voice & Audio selector still expose Google/Local only, and `TtsProvider::System` currently resolves to an unavailable synthesizer. ELSV-500 and the physical Mac audition remain open. Preserve this fail-closed behavior until the actual macOS buffer-synthesis and playback path is qualified.

### Qualification update — 2026-10-14

Scoped frontend CI passed on `57458245f2119d20ef8db180701e537d02b7d824` (run `38502627142`). Scoped Rust quality, Rust tests, and generated-contract CI passed on `f4ad9f800ecfc839c8c1e72e5ef41317d4e16905` (run `38503061659`). Changes include provider disclosure and state handling, explicit disabled System voice UI, stricter llama-server assistant-response validation, and OpenAI missing-key isolation testing. The attempted OpenAI 429 fake-server test was reverted and is not qualified evidence.

These are scoped checks, not the complete ELSV acceptance matrix or physical macOS audition. Do not mark the following items complete on this evidence alone.

### CI regression and focused routing proof — 2026-10-14

The exact-head CI run `38517544143` on `a0f90a935b9861c36446205473130bc4ba3f393f` still fails three pre-existing `SettingsModal.test.tsx` assertions. They assume that selecting Google *text* exposes the Gemini Live model selector, but that selector is intentionally gated by `asr_mode === "gemini_live_audio"`. The new `ExternalTextLiveBoundary.test.tsx` passes both tests proving that local ASR with Google text hides Live configuration and explicit Gemini Live ASR exposes it. The existing tests must set both Google text and Gemini Live ASR when asserting Live controls. Root cause and exact remediation are recorded in `docs/ELSV_FRONTEND_CI_REGRESSION_2026-10-14.md`. The three legacy Live-selector tests now explicitly select Gemini Live ASR, and a new negative test verifies that Google text with local ASR remains non-Live. The frontend quality gate passed at exact `master` head `32928314562b42e85a1727ee859192df92c5a48d` (run `38521509616`). That run was frontend-scoped; Rust, macOS, full `check:all`, and physical voice audition were not executed. Keep ELSV-600 and ELSV-900 unchecked until their full acceptance evidence exists.

### External provider disclosure qualification — 2026-10-14

The new `src/test/ExternalTextProviderDisclosure.test.tsx` covers OpenAI, Ollama, and llama-server conditional settings with local ASR, and confirms that explicit Gemini Live ASR keeps its separate Live controls. Its initial CI failed formatting on `e297b73`, then a duplicate Live-disclosure text assertion on `4fc85af`. Both were corrected without relaxing lint, format, or test gates. The exact-head frontend quality workflow **passed** on `93bc7c11c655e53ea62dd69684e8d542837be353` (run `38538281410`, 2026-10-14). This is frontend-scoped evidence only: Rust quality/tests, full `check:all`, macOS system voices, physical audition, and ELSV-900 remain unqualified. Keep checklist items unchecked until all named acceptance conditions are met.

### Ralph Bridge v0.6.1 source-write recovery and focused CI — 2026-10-14

The connected Ralph Bridge reported version `0.6.1`, build `bf1740f60dcececb42c4cf2df5931689a809ef00`, with working repository reads and exact-head direct-to-`master` writes. The prior generic safety-check refusal did not recur on the following verified writes:

- `bfcc71d8733a92c945eb86b6ad42f2e621cfe80b`: OpenAI Responses parser rejects an individually incomplete/in-progress assistant message even when its outer response status is completed; three negative regression cases added. The run on this SHA (`38549211807`) was superseded and cancelled, so it is **not** Rust qualification evidence.
- `b4a56b398b6370a8f79e40b4567ca1e52a204c42`: the local-server readiness UI now binds results to the selected provider/model/address and ignores older overlapping requests; focused frontend tests added. CI run `38549715898` failed only at Prettier formatting of the new test (typecheck and lint reached successfully).
- `a4fb23f3aa5fee212d261878476b5d5cf781e33b`: formatted that test. Exact-head frontend quality CI `38549978526` passed, including the frontend quality gate.

These commits are partial progress on ELSV-200/300/600; they do not satisfy their full acceptance criteria. The exact-head Rust verification after the latest frontend commits, full `check:all`, routing matrix, macOS implementation, and physical Mac audition remain outstanding. Keep the implementation checklist unchecked until the complete named evidence exists.

### Additional fail-closed regression evidence — 2026-10-14

Fresh-profile Local text and Local TTS defaults were asserted by `src/test/ExternalProviderDefaults.test.ts` (exact-head frontend CI `38554805135`, passed on `52292722ec506540ee380c161681a2b5609be6cb`). The Voice & Audio panel's disabled System Voices option was asserted by `src/test/SystemVoiceDisabled.test.tsx` (frontend CI `38554984333`, passed on `5c66eaef22e5673bcca86c1ee6856e2e26e53b16`). Backend unavailable-System TTS now has an explicit no-fallback test in `src-tauri/src/ai/unavailable.rs` (Rust CI `38555219055`, passed on `c0156739bfd1ff079ea3e49794d7792761a76723`). These are regression guards for incomplete functionality, not System TTS implementation or physical Mac acceptance. ELSV-500 and ELSV-900 remain open.

OpenAI Responses parsing now rejects unexpected non-message output items (including unrequested tool calls) and non-text message parts (including refusals) instead of silently extracting partial text (`360286e6cd49e0c24638c30caf66af0b034577ee`). Exact-head Rust quality, Rust tests, and generated-contract CI passed in run `38556133913`. A focused negative parser regression test for this new behavior is still outstanding, so ELSV-200 remains unchecked.

### OpenAI negative parser qualification — 2026-10-14

Commit `ab402a49116273180d1bdf9dacd6de288e0eeabb` adds explicit regression cases rejecting unrequested Responses tool-call items, refusal parts, and non-text output. Exact-head Rust quality, Rust tests, and generated backend-contract CI passed in run `38557444569` (2026-10-14). A second regression on `45dce71cd214e27f788ad4901d32bbdf34549d68` rejects a response that mixes valid assistant text with an unrequested tool call; exact-head Rust quality, Rust tests, and generated-contract CI passed in run `38558738133` (2026-10-14). This is focused ELSV-200 parser evidence, not completion of OpenAI credential, transport, routing, or platform acceptance. ELSV-200 and ELSV-900 remain unchecked.

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
