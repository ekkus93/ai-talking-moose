# Local LLM V1 Final Reconciliation — 2026-09-03

## Status

**Closure candidate.** P0 through P14 are already implemented and reconciled. The P15 implementation/probe work is merged to `master`, and the exact merged implementation tree is green. This final reconciliation branch adds one missing durable quality gate: ordinary CI now executes the literal canonical command `npm run check:all` after clean dependency installation.

P15 and the Local LLM V1 Final Gate must remain open until that new canonical job passes on the exact final reconciliation head. After that proof exists, this report and `docs/TODO(20260831-081800).md` will be updated with the exact run/head and closed on a final exact-head green run.

## Authoritative implementation state

- P15 probe tranche PR: #41 — `Local LLM: close P15 negative-probe and silent-failure gaps`
- P15 probe merged `master`: `1feaaa951c987da55828d0774ded0cfdf0651b27`
- P15 persisted-settings fail-closed PR: #42 — `Local LLM: fail closed on persisted settings errors`
- Current implementation `master`: `9b3e8621fb41836365f0d5af2167564025279956`
- PR #42 exact-head CI: `33734840062` — success on `262316fd0a98377020f002b27aa4c6ed6ed2edbb`
- Exact merged-master CI: `33736402906` — success on `9b3e8621fb41836365f0d5af2167564025279956`

Run `33736402906` is the canonical final implementation-tree matrix. It passed Rust quality, frontend quality, dependency audit, release metadata/static validation, Local LLM compile proofs on Linux x86_64/macOS arm64/macOS x86_64, unsigned macOS Tauri bundle smoke jobs on arm64/x86_64, and the security audit.

## LLM-150 — Canonical quality gate

### Proven on exact merged implementation master

Run `33736402906` proves:

- clean `npm ci` in frontend, dependency-audit, release-static, and both macOS bundle jobs;
- generated-tree rejection via `npm run check:generated-trees`;
- frontend Tauri command-registration verification;
- frontend IPC shape verification;
- TypeScript typecheck, ESLint, Prettier check, Vitest, Vite preview smoke, and frontend build;
- Rust `cargo fmt --check`, Clippy with `-D warnings`, Rust tests, backend failure matrix, and backend stress matrix;
- generated backend-contract drift validation;
- Local LLM packaging-policy validation;
- production and development Node dependency audits plus RustSec audit;
- llama.cpp CPU-policy compile proof on Linux x86_64, macOS arm64, and macOS x86_64;
- supported macOS arm64/x86_64 Tauri application bundles, native-runtime self-containment, release provenance, and dependency-license collection.

### Literal `npm run check:all` gap and remediation

The final audit found that `.github/workflows/ci.yml` executed every constituent gate but did not invoke the literal canonical aggregate command even though `AGENTS.md`, `README.md`, and this TODO call `npm run check:all` the canonical ordinary repository gate.

This reconciliation branch therefore adds a durable `Canonical npm run check:all` CI job. It waits for the normal frontend and Rust jobs, performs a clean `npm ci`, installs the same Linux/Rust prerequisites, generates the application icon set, and executes `npm run check:all` directly.

**Pending before closure:** exact-head success of that new job on the final reconciliation head.

## LLM-151 — Local-provider negative probes

All required negative/edge probes are implemented on current `master` and are executed by the Rust/frontend suites in ordinary CI.

| Requirement | Evidence |
| --- | --- |
| Google missing key fails without Fake/Local substitution | `src-tauri/src/app/state.rs` — `configured_google_text_without_secret_fails_auth_instead_of_using_fake_provider` requires `ProviderErrorKind::Auth`. |
| Local missing model fails without Google/Fake substitution | `src-tauri/src/app/state.rs` — `configured_local_text_with_unknown_model_fails_without_cloud_fallback` requires a Local model error. |
| Corrupt checksum fails installation | `src-tauri/src/ai/local/installer/adversarial_tests.rs` — `same_size_wrong_hash_is_rejected_without_installing_artifact` exercises the complete staging/verification/promotion path and requires SHA mismatch with no installed artifact/marker. |
| Corrupt GGUF fails load safely | `src-tauri/src/ai/local/runtime/llama.rs` — `corrupt_gguf_fails_load_with_safe_model_error` passes malformed bytes through the actual llama.cpp engine load path, requires safe `ModelLoad`, excludes the filesystem path from the message, and leaves no model loaded. |
| Network-denied Local generation succeeds | `src-tauri/src/ai/local/text_model.rs` — `local_generation_succeeds_with_repository_network_boundary_denied`; canonical P12 real-model run `33663595759` independently proves real GGUF generation after entering a network-denied namespace. |
| Model switch/delete race does not corrupt runtime state | `src-tauri/src/ai/local/runtime/tests.rs` — `generation_and_delete_are_serialized_without_runtime_corruption` proves deletion waits behind generation, unloads exactly once, removes the artifact afterward, and leaves no loaded runtime state. Existing model-switch tests additionally require unload-before-replacement and no fallback. |

These probes passed in PR #41 CI `33728972880`, PR #42 CI `33734840062` where applicable, and the exact merged implementation-master Rust suite in `33736402906`.

## LLM-152 — Privacy and silent-failure re-audit

### No prompt/output logging

`src-tauri/src/ai/local/text_model.rs` contains `local_prompt_memory_ambient_and_output_sentinels_never_enter_normal_logs`. It sends distinct typed/system/memory/ambient/output sentinels through the Local provider boundary and proves none enter normal captured logs. The test is non-vacuous through the repository log-capture liveness assertion. Native llama.cpp logging is disabled; frontend/runtime diagnostics expose only safe identity/state/timing/category data.

### No provider substitution

`AppState::get_text_model()` has an exhaustive typed `TextProvider::{Google, Local}` match. Missing Google credentials return authentication failure. Missing/unknown Local models and Local runtime failures remain Local failures. There is no production Fake selector in persisted settings and no Local-to-Google fallback.

### No swallowed install/generation/startup failures that report success

- Local installer failure states remain authoritative and are surfaced by the settings UI.
- Empty/invalid Local generation is a failure rather than manufactured success.
- The P15 audit found one real startup bug: `AppState::new_with_secret_store` used nested `if let Ok(...)` around persisted `app_settings`, so a database read error or malformed JSON could be silently ignored and fresh defaults substituted.
- PR #42 removed that behavior. Persisted-settings read and decode failures now abort startup with explicit safe context. `malformed_persisted_settings_abort_startup_instead_of_using_fresh_defaults` is the regression test.

This is intentionally fail-closed because silently substituting fresh defaults could change provider, model, privacy, and behavior choices for an existing profile.

### No stale optimistic Local-model state after backend rejection

`src/test/LocalLlmSettingsPanel.test.tsx` includes `rolls back optimistic download state when install and status refresh both fail`. The UI restores the authoritative pre-install descriptor and clears optimistic progress before attempting the refresh, so a second backend failure cannot leave the interface falsely claiming a download is active. Existing model-selection rejection tests likewise prevent an unaccepted replacement from appearing selected.

### No preview-only Fake behavior reachable in production

`src/test/tauriBridgeRuntime.test.ts` proves missing/malformed production-like Tauri IPC fails closed, query/local-storage/runtime globals cannot opt production into preview behavior, and browser preview is selected only by the explicit development path. Fake implementations remain test/development infrastructure, not ordinary production provider choices.

## LLM-153 — Evidence inventory

### Canonical real CPU model acceptance

Workflow: `Local LLM Real CPU Acceptance`

- run: `33663595759`
- accepted source SHA: `28aef16cbeeb91d9570177111560158811730b89`
- artifact ID: `9859917109`
- artifact name: `local-llm-real-cpu-28aef16cbeeb91d9570177111560158811730b89`
- artifact ZIP SHA-256: `b9e075822025dd98deac9e161d6f18281860148d13622e870a1ce75cc1c76b80`
- host: Linux x86_64, AMD EPYC 9V74 80-Core Processor
- runtime-reported available parallelism: 4
- generation occurred after the workflow entered an isolated network namespace; both model reports record `network_denial_probe_passed = true`.

The pinned runtime does not expose first-token timing separately. Canonical reports therefore record `first_token_latency_ms = null`; no synthetic value is invented.

### SmolLM2-360M-Instruct Q4_K_M

Identity:

- catalog ID: `smollm2-360m-instruct-q4-k-m`
- revision: `ab928a97ee49f3a015f35194879f68211291d6ca`
- artifact: `SmolLM2-360M-Instruct-Q4_K_M.gguf`
- SHA-256: `2fa3f013dcdd7b99f9b237717fa0b12d75bbb89984cc1274be1471a465bac9c2`
- bytes: `270,590,880`
- quantization: `Q4_K_M`
- license: `Apache-2.0`
- production installer verified: yes

Measured CPU evidence:

- cold probe: `454 ms`
- warm probe: `328 ms`
- cold-load estimate: `126 ms`
- RSS delta: `311,816,192` bytes (~297.4 MiB)
- 60-token-cap ambient generation: `937 ms`
- ambient output: `34` tokens
- throughput: `36.255314 tokens/s`
- owner-drop/reload: `452 ms`, success

SmolLM2 remains the recommended/default Local model because the canonical same-run usability evidence shows lower artifact/RAM cost and materially better measured latency/throughput than the larger alternative. This is a resource/usability decision, not a semantic-quality claim.

### Qwen3-0.6B Q4_K_M

Identity:

- catalog ID: `qwen3-0-6b-instruct-q4-k-m`
- revision: `7bcae0bc7b0606f1e948f8cdb31b98a2c10635db`
- artifact: `Qwen_Qwen3-0.6B-Q4_K_M.gguf`
- SHA-256: `9acfc1e001311f34b4252001b626f2e466d592a42065f66571bff3790d4e1b14`
- bytes: `484,220,320`
- quantization: `Q4_K_M`
- license: `Apache-2.0`
- production installer verified: yes

Measured CPU evidence:

- cold probe: `1,015 ms`
- warm probe: `545 ms`
- cold-load estimate: `470 ms`
- RSS delta: `747,933,696` bytes (~713.3 MiB)
- 60-token-cap ambient generation: `986 ms`
- ambient output: `14` tokens
- throughput: `14.187486 tokens/s`
- non-thinking output cleanliness: passed
- owner-drop/reload: `838 ms`, success

This second real model proves the catalog/runtime is not hard-coded to one GGUF/model family.

## Packaging and licensing evidence

P13 reconciliation: `docs/RECONCILIATION_LOCAL_LLM_P13_20260902.md`.

Canonical P13 packaging acceptance:

- run: `33686740197`
- exact merged source SHA: `abc5586814feeb4351672c01a4c8d9edc6578e7a`
- baseline SHA: `3253e52f7331ed7b03f0a3a4443eeef6d8e45aac`

arm64 artifact:

- artifact ID `9868918280`
- ZIP SHA-256 `212c7a0a56b59eb340246c3d97f35a4f8e472839eccedb056550494cd4723919`
- logical app delta `4,858,014` bytes, `+7.3550%`
- embedded GGUF count `0`

x86_64 artifact:

- artifact ID `9869434458`
- ZIP SHA-256 `76759816ee0e4b4e2c7b7e5bf0c120d68fc839c757f99c6321ff4a47f3dba9d1`
- logical app delta `5,344,110` bytes, `+6.7688%`
- embedded GGUF count `0`

Shipped Local runtime dependency/license evidence:

- `llama-cpp-2 = 0.1.154` — `MIT OR Apache-2.0`
- `llama-cpp-sys-2 = 0.1.154` — `MIT OR Apache-2.0`
- `utilityai/llama-cpp-rs` release commit `bed81ad4ab1a6c904b11d425608e50f976d8ea62`
- vendored llama.cpp revision `5f55650a78f92aff4d48d671423e888fac0469ff` — MIT
- native notices are shipped under `src-tauri/native/macos/notices/LocalLlmRuntime/`
- downloadable SmolLM2 and Qwen model weights are documented separately in `docs/LOCAL_LLM_MODEL_LICENSES.md` and are not redistributed in the application bundle.

## Environment and validation boundaries

Final ordinary CI uses GitHub-hosted Ubuntu and supported macOS runners. The exact merged implementation tree is validated by run `33736402906`; the P12 real-model acceptance used the Linux/AMD EPYC host documented above.

The ChatGPT sandbox used during the final P15 probe work did not have a usable Rust toolchain. An attempted offline clean frontend install was also blocked because the npm cache did not contain the required `zustand-4.5.7.tgz`. No claim in this report treats that sandbox as authoritative. GitHub Actions is the canonical environment for Rust, frontend, generated-contract, audit, and bundle gates, while the manually dispatched P12/P13 workflows are canonical for real-GGUF and packaging measurements.

No GGUF weights were added to ordinary CI or committed into the application tree.

## Known residual limitations

These are explicit limitations, not silent fallbacks:

1. **Cancellation is cooperative.** The safe high-level `llama-cpp-2 0.1.154` context-parameter API does not expose llama.cpp's abort-callback setter. Cancellation is observed between generated-token decode iterations; code does not reach into unsafe raw FFI solely to force interruption.
2. **Shutdown is bounded rather than forcibly aborting native decode.** Application exit calls `begin_shutdown()` to reject new Local work and allows `LocalRuntimeManager::shutdown()` at most five seconds. If a native decode does not return in time, a safe timeout is logged and process exit continues; the OS reclaims remaining process state.
3. **First-token latency is unavailable from the pinned runtime surface.** P12 records `null` rather than fabricating a metric.
4. **V1 CPU policy means no GPU offload is requested.** The runtime sets zero GPU layers; target-specific dependencies may still contain platform acceleration code, but Local LLM V1 does not require or select GPU offload.
5. **Local text does not imply fully local audio.** If Google TTS is selected, locally generated text is still sent to Google TTS for synthesis. Gemini Live voice conversations remain cloud-based.

## Deferred fully local voice

Local LLM V1 intentionally ends at text generation:

```text
Ambient event ─┐
               ├─> TextModel -> Local or Google text -> existing TTS/speech
Typed message ─┘
```

Voice remains independent:

```text
Microphone -> Moonshine local ASR or Gemini Live audio -> Gemini Live session -> Gemini audio
```

The following path is **explicitly deferred to a later phase** and is not claimed by Local LLM V1:

```text
Microphone -> Moonshine ASR -> Local LLM -> local/provider-neutral TTS -> speakers
```

In short: **`Moonshine -> Local LLM -> TTS` voice conversation is deferred.**

## Final Gate mapping

Subject only to the pending exact-head literal `npm run check:all` proof, the implementation evidence supports every Local LLM V1 Final Gate item:

1. typed Local/Google text-provider selection exists;
2. production Fake fallback is eliminated;
3. Local never silently substitutes Gemini;
4. SmolLM2 is pinned, verified, installable, and real-CPU tested;
5. Qwen3-0.6B supplies a second real catalog/runtime model;
6. ambient Local generation is covered;
7. typed Local generation is covered;
8. post-install Local generation is network-independent, including real-GGUF P12 proof;
9. memory/transcript/privacy/speech state semantics have regression coverage;
10. installer integrity/cancellation/atomic/path safety is adversarially tested;
11. model switch/delete synchronization is tested;
12. existing Google profiles migrate explicitly to Google;
13. settings UX distinguishes Local text from Gemini Live voice and Google TTS;
14. generated contract and Tauri command/IPC gates cover the new surface;
15. ordinary CI and application bundles remain GGUF-download-free;
16. real CPU evidence is recorded for both supported models;
17. runtime/model licenses are reconciled;
18. ordinary release/static/bundle gates are green on exact merged implementation `master`;
19. fully local `Moonshine -> Local LLM -> TTS` voice is explicitly deferred.

## Closure procedure

1. Run the reconciliation PR CI on its exact head with the new `Canonical npm run check:all` job.
2. If green, update this report with that run/head and mark LLM-150 through LLM-153 plus the Final Gate complete in `docs/TODO(20260831-081800).md`.
3. Require CI to pass again on that exact documentation-closeout head.
4. Squash-merge with an expected-head guard.
5. Verify the resulting exact `master` push CI is green. At that point Local LLM V1 is canonically closed.
