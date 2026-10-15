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

### Credential-alias normalization regression — 2026-10-14

The persisted-settings sanitizer now canonicalizes camelCase, snake_case, and hyphenated unknown credential aliases before recursively removing them from additive settings (`26e4ef1e451229e469f8ef92581a0e1154a68d40`). A focused test also covers nested arrays and preservation of unrelated future preferences (`4d591097420f6ad8545e54413423d561f35c091f`). Exact-head CI `38569071974` **passed**, including Rust tests, Rust quality, and generated-contract verification. The preceding Rust test run on `26e4ef1` recorded 976 passing tests but was cancelled after a newer commit and is not counted as qualified exact-head evidence.

**Remaining security follow-up:** `normalize_settings_update` in `src-tauri/src/commands/settings.rs` must also sanitize untrusted flattened future settings before persisting live IPC updates, using the same recursive sanitizer as startup migration. A proposed shared-helper and regression-test write was rejected by the source-write safety boundary, so no such change is claimed here. ELSV-100/200 remain unchecked pending that fix and the rest of their acceptance matrix.

### Additional focused qualification — 2026-10-14

- `feb8d942262a103570d17223218a0d9a3e52970c`: frontend regression asserts independent OpenAI, Ollama, llama-server and System voice default identifiers while retaining Local provider selections. Exact-head frontend CI `38586574204` passed.
- `0cf2f7031634ccad26cd75323d25a546e60e7590`: asserts transcript persistence remains disabled for fresh profiles. Its CI was superseded by the following commit; do not count it as a standalone passing run.
- `84d295e0daa67b8612b9702baaa59c0b60d36476`: frontend regression asserts selected Ollama configuration survives Settings tab switching. Exact-head frontend CI `38586951166` passed.
- `f14f691166a9f30fdefa6cdc10ef4c66ff9ca722`: unavailable System TTS regression additionally verifies non-retryable error and no speech text in error details. Exact-head Rust tests, Rust quality and generated-contract CI `38587186454` passed.

The above is scoped evidence only. Full routing qualification, macOS voice implementation and physical audition remain outstanding. Attempts to commit the Ollama `:latest` adapter correction and live-settings credential scrub remain blocked by source-write safety checks; neither change has landed.

### Source-write boundary follow-up — 2026-10-14

The attempted loopback adapter test-module split left `src-tauri/src/ai/loopback/tests.rs` **unreferenced** at `1bf54ae8a4907ab0726f41dd27e38bd379c05801`: writes to the existing `loopback.rs` module and two deletion attempts were rejected by source-write safety checks. Exact-head Rust CI `38588283369` passed, but this does not compile or qualify the staged test file. Recovery and required cleanup are documented in `docs/ELSV_SOURCE_WRITE_RECOVERY_2026-10-14.md`. Neither the Ollama `:latest` fix nor live-settings credential scrub has landed; keep ELSV-100/300 unchecked.

### Readiness no-fallback UI regression — 2026-10-14

- `b2ec013c89c96abbb6e2a00786a7dc4fda5a0e89`: added a fresh-profile Local text/TTS default test; exact-head CI `38595887869` passed.
- `8bcc37cfe6b8fea4416913542c45a2dba3b9bcfd`: expanded the test to assert independent loopback server/model defaults; its CI was superseded.
- `e3edbf6ceeb2731fb65706406da59becdcaff738`: added a frontend test showing that an unreachable Ollama server does not change the selected provider; exact-head CI `38596419876` passed.
- `f77ecc91ddab6ae166367dc94904e65698dbec9d`: expanded readiness coverage to a reachable server with a missing selected model; exact-head CI `38597007400` passed.

- `5b7b653368a9e1019982bd1ee1846934bea02269`: added frontend regression tests for Local standalone TTS remaining selected with each external text provider. Exact-head frontend CI `38597469033` passed. The workflow's Rust, macOS, and canonical `check:all` jobs were skipped by its scope classifier.

These tests cover a small part of ELSV-300/600 only. Attempts to remove the orphaned loopback test file, fix the Ollama `:latest` comparison, and sanitize future fields during live settings updates were rejected by source-write safety checks. None of those three changes has landed. All checklist items remain open pending their complete named acceptance evidence.

### Source-write recovery, credential sanitation, and macOS PCM groundwork — 2026-10-14

Ralph Bridge v0.6.1 accepted and verified direct-to-`master` changes that were previously rejected:

- `8a5d17b5de290a8a593641e18a5e7713d17894a1`: removed unreferenced `src-tauri/src/ai/loopback/tests.rs` using `delete_files`.
- `e772fba27c014653586c24d24bcba407a1876e47`: narrowly matched canonical Ollama `:latest` only for untagged configured models, while preserving exact explicit-tag and llama-server identity; added parser regressions.
- `46e4eadf85b6d6d4d5b74512de35128304a4a551`: shared recursive future-settings credential sanitizer now runs during both persisted migration and live settings IPC normalization, with nested-alias and round-trip regression tests.
- `a8e7d0d8b99eece51d4a1690c46148731da54d02`: added fake-server readiness checks for the Ollama alias and strict explicit-tag/llama-server negatives.
- `6069d1184d8a016f4455960abdae6ed2cda72503`: added bounded float-to-mono 16-bit PCM conversion with sample-rate preservation and negative tests as a platform-neutral component for future AVSpeechSynthesizer buffer integration.
- `b875e38938726a69606ef54a9a26543fc20d64fe`: applied Rust formatting corrections.
- `ff64451e3ea34144c4f9c8a02d7e8c91430baed8`: addressed Rust 1.99 Clippy `manual_is_multiple_of` in the PCM helper.
- `7e18991ea47a37cffc1a40424d9cf966810c60fe`: added exact saved-system-voice ID resolution with explicit System Default and missing-voice rejection.

**Qualification is not yet complete.** Earlier exact-head Rust quality CI `38603840804` failed a Clippy lint and was corrected afterward. The later CI for `7e18991` was queued when this evidence was written. Reinspect the exact resulting SHA and its Rust tests, Clippy, and generated-contract jobs before checking any acceptance item. The native macOS AVSpeechSynthesizer buffer callbacks, main-run-loop integration, dynamic installed-voice catalog, Control Panel enablement, physical Mac audition, and full `check:all` remain outstanding; System TTS intentionally stays unavailable rather than falling back.

### Latest Rust CI evidence — 2026-10-14

Exact-head CI `38607283397` passed on `3965d5e7a272ed55767f8cf01d168267bb2f095c` after Rust formatting corrections. Exact-head CI `38608770125` passed on `b3f3c12df3fa8df73ca9a85115485a0550eea4d4`, including Rust quality, Rust tests, and generated-contract checks. The latter also tests bounded cumulative PCM and f64 downmix of extreme finite channels. Native macOS system voice synthesis and physical audition are still outstanding. Keep ELSV-500 and ELSV-900 unchecked.

### ELSV-500 planar PCM qualification — 2026-10-14

Exact-head CI `38619191978` passed on `8142640892dc0cc5768a38f1264dbdf7373db6f0` (Rust tests, Rust quality, and generated-contract checks). New `SystemPcmAccumulator::append_planar_f32` supports bounded planar float-channel conversion without an interleaving allocation. Regression coverage includes channel mismatch, non-finite samples, cumulative duration, and oversized chunks. This does **not** implement native macOS `AVSpeechSynthesizer`, installed voice enumeration, cancellation, real playback, or physical Mac audition. ELSV-500 and ELSV-900 remain open.

### Saved System voice UI and documentation — 2026-10-14

The Control Panel now preserves the checked radio state for a saved System TTS selection while continuing to disable that unfinished provider. Exact-head frontend CI `38621242235` passed on `c86c86bde81dfbf55e1c40c801950dc66fbe0eda`. README setup guidance for OpenAI, Ollama, and llama-server landed in `705b48d708ccb9ed75335f8892d3761c2cd8d6cb`. Native macOS synthesis, installed-voice enumeration, persisted-System selection tests, physical audition, and full ELSV acceptance remain open.

### macOS system-voice input/catalog groundwork — 2026-10-14

Direct-to-master commits `34b13d4`, `6742f9ac`, `ba17a392`, `ce45b17c`, and `6ec111dd` added a control-character voice-ID regression, native catalog normalization (invalid/duplicate/reserved IDs are excluded), and a bounded fail-closed System TTS request validator that rejects empty/oversized speech text, invalid rates, and unavailable saved voice IDs without substituting another voice. The validator consumes the normalized catalog. Exact-head CI **passed** on `6ec111dd509a8e45a3e94c8584401dd68381147d` (run `38625327061`): Rust formatting/Clippy, Rust tests, and generated backend-contract verification. Earlier attempts were superseded or failed formatting and are not acceptance evidence.

These are platform-neutral prerequisites, **not** native macOS speech synthesis. The OS voice enumeration, AVSpeechSynthesizer buffer callback, playback/cancellation integration, Control Panel enablement, physical Mac audition, and complete ELSV routing/check:all qualification remain open. System TTS continues to fail closed; keep ELSV-500/900 unchecked.

### Additional System PCM and capability qualification — 2026-10-14

- `f11f02a3483f5934b2d4f890dc24c707b8dbf4c4` corrected a Rust formatting failure in the planar signed-32-bit PCM test found by exact-head CI `38634258456`.
- `acb84f1e965ac1bf56a19437e961b3772ce43b28` rejects even finite pitch values in System TTS requests, rather than silently accepting an unsupported Google-only control; System snapshot routing already omits pitch. The test now verifies explicit rejection and successful pitch-free validation.
- `32568d13574f38ce4fffec28644a2ccdebbad552` adds bounded, atomic conversion of native interleaved signed-32-bit PCM into mono signed-16-bit playback audio, with sample-rate, extrema, malformed-input, and duration regressions. Its scoped CI identified a Rust 1.99 Clippy warning in an existing PCM test.
- `0dc545fdf3888471fcb67ef449e84d37c8ff2dfe` fixes that Clippy regression without suppressing the lint. **Exact-head CI `38638116667` passed**, including Rust quality/Clippy, Rust tests, and generated backend-contract verification. Earlier superseded or failed runs are not accepted as qualification evidence.

These changes expand platform-neutral input-format coverage but do **not** implement native macOS AVSpeechSynthesizer, OS voice enumeration, playback/cancellation integration, Control Panel enablement, or physical Mac audition. The complete routing matrix, macOS build, and canonical `check:all` remain unqualified. Keep ELSV-500 and ELSV-900 unchecked until those named criteria have real evidence.

### Rust regression qualification — 2026-10-14

Commits `30cd8b395d707e87b75935267efc66a39db4212e`, `ee377d69b02b852a01efdcd790039baba5b04aa4`, and `6be9c636cdd2180aadb2eb24482156857b83cc84` corrected Rust 1.99 Clippy failures in settings schema tests and added System TTS request bounds regressions. Exact-head CI `38641301159` passed Rust quality, Rust tests, and generated backend-contract verification.

Commit `0a490e2d25460c5026e5b93715b9acf7dcc63341` additionally rejects padded saved System voice identifiers before OS selection, with negative regressions. Exact-head CI `38642767523` passed Rust quality, Rust tests, and generated backend-contract verification. Native macOS synthesis, voice enumeration, playback/cancellation, Control Panel enablement, physical audition, full routing matrix, and `check:all` remain unqualified. Keep ELSV-500/900 unchecked.

### Double-precision native PCM groundwork — 2026-10-14

- `f8213e0e4e9c1e000d8520ef342e977398134bea` added bounded, atomic planar f64-to-mono PCM conversion, including extreme finite values, malformed chunks, duration limits, and truthful sample-rate tests. Its first exact-head CI `38645430867` found a rustfmt issue and was superseded.
- `93692e668b59b2c3ca6c2fe880b35954dabf4211` applied the exact rustfmt correction. **Exact-head CI `38645981744` passed** Rust tests, Rust quality/Clippy, and generated backend-contract verification.
- This qualifies only a platform-neutral conversion prerequisite. The native macOS AVSpeechSynthesizer callback, installed voice enumeration, cancellation/playback integration, Control Panel enablement, real Mac audition, and full `check:all` remain open. ELSV-500 and ELSV-900 stay unchecked.
- A potential credential-redaction overlap edge case was identified: if two provider keys share a prefix, replacing the shorter key first may expose the longer key's suffix. An attempted ordered-redaction correction was rejected by source-write safety checks and **has not landed**. Treat this as an ELSV-200 security follow-up; do not claim it qualified.

### Native PCM format and input validation follow-up — 2026-10-14

- `474f1b710145d20f147b124220f9a0d833e97127` rejects embedded NUL and other non-whitespace control characters in System TTS utterances before native synthesis, with a regression retaining legitimate line breaks. Exact-head Rust quality, tests, and generated-contract CI passed in run `38658603051`.
- `50f2bdb983c729cfb9c6bc42c37162c5527081db` adds bounded interleaved f64 PCM conversion, including finite extreme cancellation, malformed/non-finite input rejection, cumulative duration enforcement, and atomicity. Exact-head Rust quality, tests, and generated-contract CI passed in run `38658987206`.
- `7cd04246644ae6c36f528941b0f0935a09313606` extends the native-format parity contract test to interleaved f64. Exact-head CI `38659658414` passed Rust quality, Rust tests, and generated-contract verification.

These are platform-neutral macOS prerequisites, not AVSpeechSynthesizer integration or an audible Mac audition. System voice enumeration, native buffer callbacks, cancellation/playback integration, Control Panel enablement, full `check:all`, and ELSV-500/900 acceptance remain open. A separate overlapping-provider-credential redaction fix was attempted but rejected by source-write safety checks; it has **not** landed.

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

### Ollama untagged model identity follow-up — 2026-10-14

The default Ollama model ID is `llama3.2`, which Ollama may report as `llama3.2:latest` in `/api/tags` and `/api/chat`. **Resolved:** `e772fba27c014653586c24d24bcba407a1876e47` permits only the exact untagged-to-`:latest` Ollama alias in both readiness and generation; explicit tags and llama-server IDs remain strict. `a8e7d0d8b99eece51d4a1690c46148731da54d02` adds fake-server readiness regressions. The earlier write-safety rejection was superseded by these verified direct-to-`master` commits. The complete ELSV-300 acceptance matrix is still open.

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
