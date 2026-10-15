# External LLM Providers and System Voices — TODO

**Date:** 2026-10-13<br>
**Status:** Implementation in progress; acceptance and platform qualification outstanding<br>
**Specification:** `docs/EXTERNAL_LLM_SYSTEM_VOICES_SPEC_2026-10-13.md`

Check an item only after its implementation and named evidence exist. Keep the app-owned Local defaults and no-fallback rule throughout.

**Current state (2026-10-15):** ELSV-100/200/300/400/600 are implementation-qualified by the focused tests and CI records below. Native macOS System Voices are implemented and qualified by automated arm64 `AVSpeechSynthesizer` and routing acceptance; Linux System Voices correctly fail closed. **The remaining unchecked ELSV-500/900 items require physical human-audition/bubble-completion evidence and complete macOS Tauri release-smoke verification.** See `docs/ELSV_SYSTEM_TTS_PHYSICAL_AUDITION_2026-10-15.md`. The dated checkpoint sections below are historical; earlier claims that System TTS is not implemented are superseded by the 2026-10-15 checkpoints. No unauditioned speech quality or physical playback success is claimed.


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

### Atomic credential-redaction overlap qualification — 2026-10-14

Commit `1f7dde6646c27ad6393b62fe5b23e80ac0598903` initially added longest-first provider credential redaction, but its exact-head CI `38663749233` failed Rust formatting. Commit `903523e9f3f72900746aaa83399dde7eac93ab04` corrected that formatting. Longest-first replacement still had a security edge case when two different credentials overlapped without one fully containing the other: a sequential replacement could disclose a suffix or prefix of the second credential.

Commit `d6c88b10a3a6d23166f1c7e4b5825c2b3c1c141f` changed `SecretStore::redact` to find both credentials' UTF-8 boundary matches against the original input and coalesce overlapping spans before substituting redaction markers, with regressions for partial overlaps, repeated-pattern overlaps, independent occurrences, and Unicode boundaries. Its CI identified a formatting-only failure in the Unicode assertion. Commit `bcd47fe6cce967fdbd2c366b5fe7b7b938b93cdb` applied the exact rustfmt correction. **Exact-head CI `38667953946` passed** Rust quality/Clippy, Rust tests, and generated backend-contract verification on that final code SHA. This closes the *focused overlapping-credential redaction* follow-up only, not all ELSV-200 acceptance criteria.

Native macOS `AVSpeechSynthesizer` integration, runtime catalog, cancellation/playback, physical Mac audition, full provider routing matrix, and canonical `check:all` are still outstanding. Keep ELSV-200, ELSV-500, ELSV-600, and ELSV-900 unchecked until their complete named acceptance evidence exists.

### Legacy function-call/refusal isolation — 2026-10-14

Direct-to-`master` commit `30f97470c9668e8062aca538529b510b91d89f8b` rejects legacy `function_call` metadata and refusal payloads in both Ollama and llama-server assistant responses, even when the same response includes plausible text. The parser continues to accept absent/null metadata and empty modern `tool_calls`. Its first exact-head CI `38668804961` found only a rustfmt layout failure; commit `e401f05c11119e6397e70bbcd54e977d0d71d261` corrected that formatting. **Exact-head CI `38669056161` passed** Rust quality/Clippy, Rust tests, and generated backend-contract verification. This is focused ELSV-300 response isolation evidence, not completion of the full server routing, cancellation, or platform matrix. ELSV-300/900 remain unchecked.

### OpenAI Settings credential lifecycle and stale-status qualification — 2026-10-14

Commit `5a5c6cdc585d2a5ab739a806181ed815ba114b36` adds OpenAI Control Panel lifecycle tests for trimmed key save, password-input clearing, persistence exclusively through the secret-store bridge, independent Google key presence, OpenAI-only removal, and sanitized key-store failures. A formatting correction landed at `42bc82a2ee7c93a91cb92bd47040aadfaf6fa3f2`. Their scoped CI runs were cancelled when another active Ralph session committed to `master`; cancelled runs are **not** qualification evidence.

Commit `c32e1ed07bc16f8781447d3ea52a1363354d8511` guards pending `hasOpenaiApiKey` replies with a request version that is invalidated by explicit save/removal actions, preventing stale presence results from overriding newer UI state. Its focused frontend CI `38669658070` **passed**. Commit `47e1cb1f6dc8cc0af47832b44ce5b847efff82dc` strengthens the asynchronous race test by awaiting React state flush; `64c0a67746186754de33528a6be4713e08ccd80b` corrects Prettier import formatting. **Exact-head frontend CI `38669894519` passed** on the final code, including frontend quality tests. This qualifies only the focused OpenAI key UI/race regressions, not the complete ELSV-200/600 acceptance matrix.

Native macOS `AVSpeechSynthesizer`, installed-voice catalog integration, playback/cancellation, physical Mac audition, full provider-routing acceptance, and `check:all` remain outstanding; keep ELSV-200/500/600/900 unchecked.

### Provider type contract, OpenAI timeout, and non-Live Talk cancellation — 2026-10-14

The ELSV-100 provider-enum checklist entry is verified against Rust `src-tauri/src/ai/types.rs` (`TextProvider::{Local,Google,Openai,Ollama,LlamaServer}` and `TtsProvider::{Local,Google,System}`), TypeScript `src/types/moose.ts`, and Rust-derived frontend settings defaults/contract; the generated backend-contract gate and Rust enum serialization tests passed in exact-head CI `38672195380` on `b9b159241e67141f74a7ff14adff692ed9bcb3b4`. Only this discrete ELSV-100 subtask is checked. Settings migration, backend policy/routing, and full cross-platform ELSV-100 acceptance are not implied by that check.

Commits `2326ec6ddb95f0e34872dc78a5f96534ecaddc49`, `c765a0ed81f83e85bd012e11a00de1b2b4f53ce4`, and `ca9404b76b8ab28a44d8ce8cca81226253eefead` added/qualified OpenAI fake-server timeout/no-retry and oversized-response rejection tests, including safe public errors that contain neither a key nor request/response text. Exact-head CI `38671638133` **passed** Rust quality/tests and generated-contract verification on `ca9404b`.

Commit `b9b159241e67141f74a7ff14adff692ed9bcb3b4` prevents an interrupted non-Live transcript publication that was waiting on event-channel backpressure from subsequently initiating standalone TTS, which could otherwise send an obsolete answer to a selected cloud speech service. A regression covers interruption during blocked transcript publication. **Exact-head CI `38672195380` passed** Rust quality/tests, generated contract, and Local LLM compile proofs. These are focused ELSV-200/400 regressions, not full provider-routing acceptance.

**Still outstanding:** native macOS voice enumeration and `AVSpeechSynthesizer` integration, physical Mac playback audition, the complete provider/ASR/TTS matrix, full `check:all`, packaging-policy and generated-tree qualification, and the remaining individual TODO tasks. No new branches or PRs were created.

### ELSV-100 persisted-settings migration and contract verification — 2026-10-14

Version-8 `AppSettings` contains independent OpenAI/Ollama/llama-server model IDs, two loopback server addresses, and `system_tts_voice`; `AppSettings::from_persisted_json` migrates version-7 profiles without discarding explicit saved Google/Local selections or unrelated future additive fields. Existing `state/tests/settings_migration.rs` and `app/settings_policy.rs` tests verify migration, current/default serialization, provider round trips, invalid model/address rejection, nested credential scrubbing, and future-version fail-closed behavior; `frontend_contract_matches_authoritative_rust_defaults_and_catalogs` checks the checked-in generated contract. New direct-to-`master` commit `0f65ed0fe1e1af5281c0d276a78d4fa8284103e6` further verifies version-7 Local-text + Google-standalone-TTS identity, external provider defaults, and idempotent second migration. **Exact-head CI `38673594292` passed** Rust tests, Rust quality/Clippy, and generated backend-contract verification.

These findings qualify the discrete ELSV-100 settings-field/migration and defaults/serialization/invalid-input/future-version test subtasks, now checked. The remaining ELSV-100 policy/immutable-snapshot/ordered frontend write subtask and full cross-provider/platform verification are still open.

### ELSV-100 routing policy and serialized settings write qualification — 2026-10-14

The backend settings boundary in `src-tauri/src/app/settings_policy.rs` rejects invalid OpenAI model IDs, malformed Ollama/llama-server model IDs, non-loopback addresses, invalid ports, path/query/fragment/user-info URLs, and invalid preference ranges. `src-tauri/src/app/request_snapshot.rs` captures request-scoped typed `AppSettings` and resolves the selected text provider without fallback; non-Live Talk composes selected text/TTS from that snapshot, while explicit Gemini Live audio ASR retains its distinct route. `src/stores/mooseStore.ts` persists field-intent patches serially against the latest successful settings snapshot.

Frontend direct-to-`master` commits `848a614c6a5fb129fb431f6f19b25520f95ac2df`, `8ca0f8b317d1e528af3dc4611dc599bd10153e6f`, and `1f684cb79e1379368df6b09a5694355ff1afcb2e` added and qualified the independent provider/TTS serialized-patch regression, including overlapping writes without lost provider selections or concurrent persistence. **Exact-head frontend quality CI `38675039144` passed** on `1f684cb`; Rust settings-boundary and snapshot-related tests and the generated-contract gate passed in `38673594292` and `38672195380`. This qualifies the final individual ELSV-100 policy/request-snapshot/patch-write subtask. All four ELSV-100 subtasks are now checked, while the complete ELSV release and platform acceptance remain open.

### Bounded serialized requests and provider-isolated loopback qualification — 2026-10-14

Direct-to-`master` changes `d5443ee832043efd4e6ede764ce59339c8aee80d` and `35d4868f320f357bd56827e122825c7fb2d52ea6` added fake-server regressions for both Ollama and llama-server: an existing independent Google or OpenAI key is not sent in local request headers or JSON, quota errors expose no private upstream content, and the generation adapter does not automatically retry. Commit `10721c0e8b17cacb2960efed5f8c1c8f842b56da` bounds the *serialized* generation JSON, including combined prompts, instructions, and escaping, before contacting either loopback endpoint. Commit `91904ec3364a03219b48a8f8c52761738dbe2577` enforces the equivalent outbound JSON limit in OpenAI Responses. Rustfmt follow-ups landed on `master` without weakening the checks.

**Exact-head CI `38679309547` passed** on `ae6d373035bb09ba9e41bfe6a6c7ef80658d88a5`: Rust tests, Rust quality/Clippy, and generated backend-contract verification. These are focused ELSV-200/300 request-boundary and credential-isolation regressions, **not** full provider acceptance. macOS System voice implementation/audition, routing matrix, canonical `check:all`, and ELSV-900 still need evidence. Keep broader checklist items unchecked.

### Non-Live routing, cancellation, and immutable text-selection evidence — 2026-10-14

- `8a18ba78630e14e201cca3a1f36ca08e92575de7` adds a regression asserting replacement of an in-flight external text turn drops its pending generation future; its CI was superseded and cancelled, so do not count that SHA as standalone qualification.
- `264aeba85894b01898a999a22c095f7aeb8f3828` makes unsupported System TTS fail at non-Live Talk connection setup **before** any selected Local, Google, OpenAI, Ollama, or llama-server text model is contacted. The test asserts all five fail closed without emitting a transcript or exposing the private instruction. `b5e330935023e2db568b838e920fd171529edd3c` fixes its Rust formatting.
- `9fc700e7ed928e54480644a1f5f43ce88dfc8e95` adds a regression that captures OpenAI/Ollama/llama-server text provider settings once, changes the live selection, and verifies the original provider's failure identity remains intact without a cloud fallback. **Exact-head CI `38684700623` passed** Rust quality/Clippy, Rust tests, and generated-contract verification on this final SHA.

These are focused ELSV-400 safety/routing regressions only. The complete provider/ASR/TTS matrix, native macOS System TTS, physical voice audition, and ELSV-900 full validation remain outstanding. Commit `89bbd6df9a52d854bc27c237ac3d4822ff99b270` subsequently introduced cancellation-aware transcript/error/audio/completion sends and stricter transcript/audio backpressure regressions. Its exact-head Rust CI `38686419939` was cancelled by a newer documentation commit, so this fix is **implemented but not yet CI-qualified**. Additional blocked-error/completion and replacement-generation tests remain advisable. ELSV-400 stays unchecked.

### OpenAI multi-message fail-closed parsing — 2026-10-14

Commit `cfced4d1a67d8e62527b4a5aeb8d4f342481b641` rejects a Responses payload with multiple distinct assistant message items instead of silently concatenating them into one speech bubble. The negative parser regression and existing Rust checks passed at the exact commit in CI run `38690938204` (Rust quality, Rust tests, and generated backend contract). This is focused ELSV-200 evidence only; credential/transport/platform and complete release acceptance remain outstanding.

### System voice identifier spoofing guard — 2026-10-15

Direct-to-`master` commit `77ca59f4c2bc42764e5a18e6858cdc9e7ee10cd2` extends installed System voice ID normalization and saved-voice resolution to reject invisible zero-width separators and BOM characters in addition to existing bidirectional controls. Negative tests cover invisible IDs in the OS catalog and saved selection. Its first CI run `38697384843` failed only Rust formatting; commit `9108adbb305e2868e623db6faecc6bafa391e870` applied the exact rustfmt correction. **Exact-head CI `38698370612` passed** Rust quality/Clippy, Rust tests, and generated backend-contract verification.

This is ELSV-500 platform-neutral identifier safety evidence, not runtime macOS voice enumeration, AVSpeechSynthesizer buffer synthesis, audible playback, or physical Mac audition. Keep ELSV-500/900 unchecked.

### Saved System voice preflight regression — 2026-10-15

Commit `8c8ba6ef31c3926778699b023821c16edbc114d9` adds a request-level regression that rejects a saved OS voice ID containing an invisible zero-width separator even if an untrusted installed catalog advertises that same identifier. **Exact-head CI `38701336175` passed** Rust quality, Rust tests, and generated backend-contract verification. This is platform-neutral ELSV-500 selection safety evidence only. Native macOS enumeration, AVSpeechSynthesizer, actual audio playback, physical audition, and ELSV-900 full validation remain open.

### Native voice display-metadata spoofing regression — 2026-10-15

Direct-to-`master` commit `d21f799a67b51908f5caa2b6774a636058e4697a` rejects zero-width separators and word joiners in OS-supplied System voice display names and style metadata, not just stable voice IDs. Its negative tests retain valid voices while excluding invisible-formatting spoof cases. **Exact-head CI `38703363239` passed** Rust quality/Clippy, Rust tests, and generated backend-contract verification. This is ELSV-500 catalog hardening only: runtime macOS enumeration, AVSpeechSynthesizer, audible playback, physical Mac audition, full routing, and ELSV-900 remain open.

### OS catalog spoofing regression — 2026-10-15

Direct-to-`master` test commit `2e354f1`, corrected for an actual invisible separator in `201909a8` and formatted in `c9811982234331652459949c4e5cae207817c4be`, verifies that malformed third-party macOS voice display metadata cannot make a saved voice appear installed. A subsequent valid OS catalog entry with the same stable ID remains selectable; an unrelated installed voice is never silently substituted. **Exact-head CI `38710911761` passed** Rust tests, Rust quality/Clippy, and generated backend-contract verification. This is platform-neutral ELSV-500 safety evidence, not macOS voice enumeration, buffer synthesis, playback, or physical audition. ELSV-500 and ELSV-900 remain unchecked.

### ELSV-600 voice selection persistence-error isolation — 2026-10-15

Direct-to-`master` commits `fb4c9c4a937e1f9d3139c011e2422096825d264a` and `d90bf620e733650d29b9d36948534ecdf89a77f1` now handle rejected standalone and Gemini Live voice-selection writes in Voice & Audio without unhandled promise rejections or displaying raw backend errors. Focused frontend regressions assert both failures produce sanitized, actionable alerts without exposing private error content. The initial CI run `38718110017` failed Prettier only; formatting was corrected and **exact-head CI `38718259438` passed** frontend quality (typecheck, lint, formatting, and frontend tests). Rust, macOS, full `check:all`, native System TTS, and physical audition were not exercised by this frontend-scoped run. Keep ELSV-600 and ELSV-900 unchecked pending the complete acceptance matrix.

### ELSV-600 active-Live standalone voice isolation — 2026-10-15

Direct-to-`master` commits `81c4a6414289719817717271476bfc628535b6cd`, `78e330e3aae3ba49a9df74eb653c0e9523abc9b1`, and `af6030f056dd8075bb930cbd811da56167b997a2` add frontend regressions proving that switching the standalone TTS provider does **not** cancel an active Gemini Live session, while an idle standalone speech session is cancelled **before** applying a provider-selection patch. Initial CI runs `38718690575` and `38718851889` failed Prettier only. **Exact-head frontend CI `38718964624` passed** on `af6030f`, including formatting, lint, typecheck, and frontend tests. This qualifies only the tested ELSV-600 active-Live provider-switch behavior, not all voice/UI combinations, macOS synthesis, or full release acceptance.

### Credential mutation serialization and Voice & Audio error privacy — 2026-10-15

Direct-to-`master` changes `d173c08d42842657000bc371bf1f04f428616655` and rustfmt follow-up `8be28d5118aab820281948e17cae7ae0c0a95c01` serialize Google/OpenAI secret-backend writes, verification reads, and cache updates across clones of the in-memory `SecretStore`. The new concurrent-update regression checks both provider caches against their backend values and confirms clearing OpenAI leaves Google intact. The first exact-head CI `38723657674` failed Rust formatting; its corrected successor `38723904673` was cancelled by a later source commit. **Do not count the Rust change as CI-qualified until a subsequent exact-head Rust test/Clippy/contract run passes.**

Direct-to-`master` Voice & Audio changes `3a5c1ec85229e9ed04c3f9a6463b577805ec4059`, `2597d9f4176ad9595fb1131b251137192e060111`, and `e8d5eeb3ed9b2ca51493d1801ab600b7ad2467fb` replace raw IPC/backend exception interpolation on standalone TTS provider switches and auditions with fixed, credential-safe user errors. Regression tests assert no private device path or transcript from rejected operations appears in the alert. **Exact-head frontend CI `38725723839` passed** on `e8d5eeb3ed9b2ca51493d1801ab600b7ad2467fb`, including typecheck, lint, formatting, and frontend tests. Earlier runs `38724627306` and `38725036312` failed only formatting and were corrected.

These focused ELSV-200/600 safety improvements do not implement native macOS System TTS or complete the provider/ASR/TTS matrix, platform audition, or ELSV-900 release acceptance. Keep the incomplete acceptance checkboxes unchecked.

### Per-section audit and gap closure — 2026-10-15

Read-only audits of ELSV-200/300/400/600 against code and tests (local Linux x86_64; `npm run check:frontend` and `npm run check:rust` pass at each commit below). Items are checked only where the named tests exist.

- **ELSV-200:** Responses adapter (bounded, explicit model, strict parser, no retry/fallback) and the test matrix are checked. `8417a87e` added HTTP 400/404 → Model, 401 → Auth, malformed/HTML/truncated/empty body → Protocol, plus command-level Google/OpenAI key isolation and no key in serialized settings or the stored settings row. **Still open:** (1) `SecretStore::redact` has no production caller (OpenAI errors rely on fixed messages; only Google uses `redact_secret`); macOS Keychain is untestable on Linux. (2) No assertion that the key never reaches browser storage or logs/diagnostics. No end-to-end saved-key → fake-OpenAI test (`with_endpoint` is private, endpoint is hard-coded).
- **ELSV-300:** adapters/bounds and the fake-server matrix are checked. `6046393e` added stall timeout without retry, 64 KiB response cap over HTTP, refused connection → Unreachable (distinct from a 500 → ReachableButModelUnavailable), `localhost` generation, mid-request cancellation, error sanitization, and a backend no-fallback test. **Still open:** (1) no assertion that the `localhost` → `127.0.0.1` resolve pin is what makes the connection (the fake binds 127.0.0.1 either way); remote-URL rejection is tested for `generate` but not `check_readiness`. (3) No test asserts the adapters never launch a server or pull weights. No-fallback proof for Google/OpenAI is indirect (their endpoints are hard-coded).
- **ELSV-400:** `da46154c` adds the first success-path routing test: the selected Ollama and llama-server reply, through the app's `get_text_model_for`, is exactly what non-Live Talk synthesizes with the settings voice and rate. **Still open:** same for OpenAI (endpoint not injectable); typed and ambient paths for external providers; Talk with Google TTS auth failure or unavailable KittenTTS; `start_conversation`-level System TTS rejection; non-Live audio-completion and bubble-lifetime tests; typed-reply TTS is snapshotted separately from the text snapshot (design question, deferred); CI qualification of `89bbd6df`.
- **ELSV-600:** `96b55e13` onboarding discloses OpenAI cloud and loopback servers; stale PRIVACY/ARCHITECTURE status text refreshed. `5ad4e489` adds Voice & Audio catalog loading/empty states plus audition success, no-overlap and missing-key tests. `22a0afab` keeps external model-ID drafts across tab switches, shows "OpenAI key saved / model not verified", and adds cloud/loopback disclosure tests. `56cf1087` adds role-based accessibility checks (no axe dependency was added). **Still open:** System Voices refresh/selection/Audition (blocked on ELSV-500); the audition lock is a 2.5 s post-call debounce, not playback completion; OpenAI has no live readiness check.

### macOS System Voices native implementation and CI acceptance — 2026-10-15

`eb75fb37` adds `src-tauri/src/ai/system_tts/macos.rs`: voice enumeration via `AVSpeechSynthesisVoice`, buffer capture via `AVSpeechSynthesizer.writeUtterance:toBufferCallback:` on the main dispatch queue, conversion through the existing bounded `SystemPcmAccumulator`, cancel-on-drop (`stopSpeakingAtBoundary`), first-buffer/gap timeouts, and a monotonic `speaking_rate` → `AVSpeechUtterance.rate` mapping. The catalog lists System Voices only on macOS (`get_tts_catalog`), the static catalog and generated contract are unchanged, and other platforms still fail closed with no subprocess or provider fallback. `3d0413f9` enables the Voice & Audio System radio only when the catalog offers it, adds Refresh System Voices, and blocks Audition with a reason when the saved voice is no longer installed.

Evidence (workflow `macOS System TTS acceptance`, `macos-15` arm64, exact head `f1cc46af`, run `38797713849`, **success**): 29 System TTS unit tests pass; the `system_tts_acceptance` binary found 191 installed voices and passed all checks — System Default and one installed voice produce non-silent audio (RMS ≈ 5283 / 4598, peak ≈ 24 170) at 22 050 Hz, 4.4 s / 5.2 s; rate 2.0 is shorter than rate 1.0 (2.26 s vs 4.43 s); a nonexistent voice fails closed with a Model error; synthesis succeeds after a cancelled request. The first run (`38796306560`) failed only because two Linux-only "System unavailable" tests also ran on macOS; they are now gated by `cfg(not(target_os = "macos"))` (`f1cc46af`).

**Still open (not claimed):** a human listening to the uploaded `system-tts-audition` WAVs and observing a spoken bubble persist until playback ends in the real app (ELSV-500 item 4, ELSV-900 audition). The acceptance binary does not exercise real-time playback, the 15 s/10 s timeout paths, a no-audio response, or an unsupported buffer format on macOS. The `macos-15-intel` architecture was not run. The automatic "first installed voice" case picked a non-English voice (Majed) speaking English text; it proves synthesis, not suitability. The ordinary CI macOS bundle jobs were not re-inspected for this change.

### ELSV-400/300 Rust routing tests — 2026-10-15

Commits `67b54959`, `895f4b4d`, `eada5286`, `4771011d` (local `npm run check:rust` exit 0, 1065 lib tests; test-only code, no production behavior changed). Typed replies through the real `send_text_message` command and ambient remarks reach only the selected Ollama / llama-server / OpenAI fake (OpenAI via the `cfg(test)`-only endpoint override), with success and 503 cases showing a safe error, no retry, no fallback and unchanged `text_provider`. Talk voice failures never fall back (Google TTS without a key surfaces `Auth`; uninstalled Local TTS surfaces a Setup/Model error with no Google fallback), and an already-created provider keeps its original selection after settings change. The non-Live event order (final transcript → exact PCM `AudioData` → `TurnComplete`) and stale-generation suppression are pinned at the provider-event level. Remote/private/unspecified addresses are rejected by readiness without a connection attempt.

Findings and limits: (1) With a non-System TTS provider, Talk sends the text to the selected text provider before a Google TTS auth failure appears; only System TTS is rejected up front (behavior is pinned, not changed). (2) The `localhost` → `127.0.0.1` pin cannot be observed on a host where `localhost` resolves only to IPv4; that test is a decoy that does not discriminate here, so it is not counted as pin evidence. (3) Ambient success is tested at `generate_ambient_text`; end-to-end ambient success needs a real voice and unmuted state. (4) Bubble lifetime lives in the event loop/frontend and is not covered here. (5) No production code logs or formats an OpenAI key (grep audit); `SecretStore::redact` remains unused outside tests. ELSV-300 item 1 and ELSV-400 items 2–5 stay unchecked for these reasons.

### Current workspace follow-up — 2026-10-15

Voice audition no-overlap state now uses the queued PCM duration returned by `audition_tts_voice`, with a 250 ms output margin, instead of a fixed 2.5 s debounce. Frontend regression coverage checks the lock through a 5 s sample. Voice & Audio now describes macOS System rate/pitch capability accurately and discloses that human listening and playback completion acceptance remain open. README, Privacy, onboarding, Local LLM architecture, and voice-selection guidance now describe the implemented native macOS AVSpeechSynthesizer path and the unsupported-platform fail-closed behavior. ELSV-600 implementation and frontend-test subtasks are checked based on their existing provider, disclosure, accessibility, stale-result, audition, and Live-isolation coverage.

On Linux x86_64, `CARGO_BUILD_JOBS=2 npm run check:all` passed on the current workspace (base commit `23941076b2172b1161e6672ea684ffca25e8c68d`), including 1,066 Rust tests (3 ignored), 194 frontend tests, production build, and generated backend-contract verification. `npm run check:frontend` also passed after the final UI/test changes. `python3 scripts/check_local_llm_packaging_policy.py` printed `local-llm-packaging-policy-ok`; `git diff --check` was clean. The key lifecycle tests assert no key in local/session storage or captured logs, in addition to settings/SQLite and provider-isolation checks. The loopback pin regression requires an IPv6 decoy and proves `localhost` reaches IPv4 rather than silently weakening to a reachability-only test. ELSV-200/300 and Linux verification subtasks are checked below. ELSV-400 routing composition and lifecycle behavior are checked; its full cross-provider/voice/ASR combination matrix remains open. Native AVSpeechSynthesizer synthesis, bounded PCM handling, cancellation and timeout/error handling are implemented; Mac smoke plus human audition and real-app bubble observation remain open.

macOS compilation/smoke checks and a person listening to the generated WAVs and observing the spoken bubble through playback still require a macOS host and human audition; these are not claimed complete here.

### ELSV-400 full provider-boundary matrix and native macOS routing — 2026-10-15

- Direct-to-master commit `964660cb3f5ec3b0289a0243ad5a9561de450d0a` extends non-Live routing coverage across Moonshine Tiny/Small streaming ASR, all five text providers, and both Local/Google standalone voices (20 combinations). Exact-head CI `38805038356` **passed** Rust tests, Clippy, and generated-contract verification.
- Commit `2b51ec4603e7adddee8b26f4289ef306b7b20ca3` verifies explicit Gemini Live ASR stays in Google's Live authentication boundary for all five standalone text choices and all three saved standalone voice choices (15 combinations), without silent fallback. Exact-head CI `38805871639` **passed**.
- Another concurrent Ralph commit, `c468660bb51993cc5c6c31f8cae13763dfd39073`, tests non-macOS System voice rejection across three local ASR modes and five text providers (15 combinations). Its CI was cancelled after later commits; its code remains covered by the passing Rust suite on subsequent exact heads.
- Commit `3a0906c95f21a83faf89ede14dd02a0f56d9e83a` adds macOS-only non-Live System TTS routing tests covering the same 15 local-ASR/text combinations, registered through `state/tests/mod.rs`. The updated native workflow ran the new routing tests and the existing AVSpeechSynthesizer synthesis acceptance: **macOS arm64 run `38807233726` passed**, and produced the `system-tts-audition` WAV/report artifact. A new manual acceptance document, `docs/ELSV_SYSTEM_TTS_PHYSICAL_AUDITION_2026-10-15.md`, retains honest pending human observations. Initial general CI failed only the document trailing-whitespace gate; `b201cb3dae31d601bec2bb8ec68d89975e9de966` corrected it and docs-fast-path CI `38807560474` **passed**.
- Commit `623cf06606b8b02ab260940ccecf64090566d27a` adds a Linux fake-server test using valid test Google/OpenAI credentials and three reachable fake endpoints. For each local ASR/text combination, unsupported System TTS fails at connection preflight with **zero** accepted OpenAI, Ollama or llama-server connections. Exact-head Rust tests, Clippy and generated-contract CI `38808413823` **passed**.
- Together with previously qualified selected-provider generation, typed/ambient, failed-model/voice, mid-request setting snapshot, cancellation, stale-event and TTS negative tests, these establish the supported routing/negative-path **test matrix**. The ELSV-400 matrix subtask is checked for this deterministic qualification; it does **not** assert that every cloud/provider combination was auditioned with live credentials or real downloaded weights, nor that the real Mac UI bubble is qualified. Those remain separate ELSV-500/900 human-hardware and platform-release acceptance gates.

**Still unqualified:** real physical Mac audible comprehension, chosen output device and speech-bubble completion/interrupt observation; full supported macOS Tauri release bundle/smoke (including Intel); and final ELSV-900 release closeout. Automated WAV analysis or synthetic provider stubs cannot substitute for those observations.

## ELSV-100 — Settings and IPC foundations

- [x] Add stable OpenAI, Ollama, and llama-server `TextProvider` values and System `TtsProvider` value across Rust, TypeScript, and generated contract.
- [x] Add separate provider model IDs, loopback addresses, and system voice ID to settings; bump settings version; migrate old settings without changing saved Google/Local choices or wiping unknown future settings.
- [x] Validate model IDs, server addresses, and provider combinations at the backend settings boundary; preserve request-scoped settings snapshots and ordered frontend patch writes.
- [x] Add migration, serialization, defaults, invalid-input, and future-version tests; regenerate and review `src/generated/backendContract.json`.

**Acceptance:** fresh profiles remain Local/KittenTTS, existing profiles retain their choices, and frontend/backend settings shapes agree.

## ELSV-200 — OpenAI credentials and text adapter

- [x] Extend secret backend, macOS Keychain identifiers, redaction, and Tauri commands for an independent OpenAI key; keep Linux key session-only and say so in the UI.
- [x] Ensure key save/clear/has never reaches persisted settings, SQLite, frontend storage, logs, diagnostics, or another provider; clearing Google must leave OpenAI intact and vice versa.
- [x] Implement bounded OpenAI Responses text requests from `TextRequest`, explicit model selection, output extraction, safe error categories, and no automatic retry/fallback.
- [x] Add injected-transport tests for request shape, auth, no-key, invalid model, rate limit, timeout, oversized/malformed output, redaction, and provider isolation.

**Acceptance:** selected OpenAI text works with a saved key and failures cannot send the same request to another provider.

### Ollama untagged model identity follow-up — 2026-10-14

The default Ollama model ID is `llama3.2`, which Ollama may report as `llama3.2:latest` in `/api/tags` and `/api/chat`. **Resolved:** `e772fba27c014653586c24d24bcba407a1876e47` permits only the exact untagged-to-`:latest` Ollama alias in both readiness and generation; explicit tags and llama-server IDs remain strict. `a8e7d0d8b99eece51d4a1690c46148731da54d02` adds fake-server readiness regressions. The earlier write-safety rejection was superseded by these verified direct-to-`master` commits. The complete ELSV-300 acceptance matrix is still open.

## ELSV-300 — Ollama and llama-server adapters

- [x] Implement backend-only loopback-address validation for each configured server, including port, URL component, redirect, and effective-destination checks.
- [x] Implement provider-specific chat-completions request/response adapters and bounded HTTP behavior; do not assume the two servers expose identical capabilities.
- [x] Add explicit connection/model checks with truthful reachability versus model-readiness results; never launch a server or pull weights.
- [x] Add fake-server tests for IPv4/IPv6/localhost, rejected remote/redirect URLs, model mismatch, malformed body, timeout, cancellation where available, error sanitization, and no cloud fallback.

**Acceptance:** an unavailable or incompatible server reports a clear error while remaining the selected provider.

### Non-Live Talk preflight and request-scoped cancellation qualification — 2026-10-14

Commits `24af65d24fc024fdfa3714a93824d126d435a4ad` and `4a66bfdd3f86e7a160f36cbbd50045703e3dcfde` extended non-Live Talk interruption from stale-output suppression to actual cancellation of the in-flight selected text-model future. Commit `8a18ba78630e14e201cca3a1f36ca08e92575de7` tests that replacing a turn drops its previous pending generation instead of continuing to consume the external provider. Commit `264aeba85894b01898a999a22c095f7aeb8f3828` rejects every unsupported non-Live System TTS pairing before sending any text provider request, without changing explicit Gemini Live ASR routing. Its rustfmt correction landed at `b5e330935023e2db568b838e920fd171529edd3c`. Commit `9fc700e7ed928e54480644a1f5f43ce88dfc8e95` verifies immutable request-scoped external text-provider identity even when a user changes settings after capture.

**Exact-head CI `38684700623` passed** on `9fc700e7ed928e54480644a1f5f43ce88dfc8e95`, including Rust tests, Rust quality/Clippy, and generated backend-contract checks. Earlier superseded/cancelled runs are not counted as passes. This is focused ELSV-400 routing and cancellation evidence, not the complete provider/ASR/TTS matrix. Native macOS System TTS, physical audition, frontend/full release checks, and ELSV-900 remain outstanding; keep incomplete acceptance items unchecked.

## ELSV-400 — Route selected providers through app behavior

- [x] Extend text-model factory for typed replies and ambient remarks using one immutable request snapshot.
- [x] Refactor non-Live Talk to compose selected text model and selected standalone TTS, including Local, OpenAI, Ollama, llama-server, KittenTTS, Google TTS, and System Voices where supported.
- [x] Keep Gemini Live audio mode and existing explicit Google Live behavior separate; show or reject unsupported combinations before Talk starts.
- [x] Preserve transcript/privacy gates, generation identity, barge-in, cancellation, stale-response suppression, audio completion, and bubble lifetime.
- [x] Test every supported routing combination and negative path, including cloud-text/local-TTS, local-text/cloud-TTS, unavailable model/voice, mid-request setting changes, and no unintended Gemini Live connection.

**Acceptance:** the provider shown in Settings is the one used by each relevant request; a selected failure never silently changes provider.

## ELSV-500 — macOS system TTS

- [x] Add macOS runtime voice enumeration and stable voice IDs, including explicit System Default and unavailable saved voice behavior.
- [x] Implement AVSpeechSynthesizer buffer synthesis and bounded PCM conversion into the existing Rust playback path; implement cancellation, timeout, no-audio, and format-error handling.
- [x] Expose truthful voice capabilities, rate/pitch controls, and system-provider metadata through the TTS catalog/IPC; show unsupported status on Linux without a subprocess fallback.
- [ ] Add unit tests for catalog, selection, conversion, cancellation, unavailable voice, and Linux compile behavior; record a real Mac audition with audible completion and bubble-duration observation.

**Acceptance:** audition and non-Live Talk speak through the selected OS voice, and the bubble persists until playback ends or is cancelled.

## ELSV-600 — Control Panel and disclosure

- [x] Extend AI & Models with five provider choices and conditional OpenAI/server controls, provider-specific connection states, and accurate data-destination labels.
- [x] Extend Voice & Audio with System Voices, voice refresh/selection, Audition, capability-aware controls, and clear Live versus non-Live scope.
- [x] Preserve tab-switch state, no-overlap auditions, active-Live-session behavior, accessibility, loading/error/empty states, and stale-patch safety.
- [x] Update README, Privacy, onboarding, Local LLM architecture, and any relevant Settings help text to reflect cloud, loopback, Local, and Gemini Live boundaries.
- [x] Add frontend tests for provider switching, key handling, server readiness, system voice availability, audition, and privacy disclosures.

**Acceptance:** the Control Panel makes the actual network and voice behavior understandable before the user selects a provider.

## ELSV-900 — Verification and closeout

- [x] Run focused provider, routing, secret, settings migration, TTS, and frontend tests without live services or downloaded weights.
- [x] Run `CARGO_BUILD_JOBS=2 npm run check:all`, `npm run check:generated-trees`, and `python3 scripts/check_local_llm_packaging_policy.py`; fix any drift without weakening a gate. (2026-10-15, Linux x86_64, local at `99052efd`: all three exit 0; packaging policy printed `local-llm-packaging-policy-ok`. Linux only; macOS not exercised.)
- [ ] Run supported macOS build/smoke checks and a physical Mac system-voice audition; record exact commands, results, limitations, and platform.
- [x] Review generated contract, documentation, Git diff, and status; verify no API keys, GGUF files, generated trees, or audio artifacts entered the commit.
- [ ] Mark this TODO complete only when every acceptance criterion has evidence; record any intentionally deferred platform work separately.

**Final acceptance:** OpenAI, Ollama, llama-server, and macOS system voices are explicitly selectable, accurately disclosed, correctly routed, and validated without weakening Local defaults or privacy boundaries.
