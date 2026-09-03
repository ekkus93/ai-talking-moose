# Local LLM V1 Final Reconciliation — 2026-09-03

## Status

**P15 implementation and evidence are COMPLETE. Local LLM V1 satisfies the Final Gate.**

This report closes `LLM-150` through `LLM-153` from `docs/TODO(20260831-081800).md` against exact implementation, CI, real-model, packaging, privacy, and negative-probe evidence.

The final reconciliation deliberately found and fixed two issues instead of treating prior green phases as sufficient:

1. persisted `app_settings` read/decode failures could be silently ignored and fresh defaults substituted;
2. ordinary CI executed the constituent repository gates but did not invoke the literal canonical command `npm run check:all` required by the TODO/developer contract.

Both are now addressed. The persisted-settings fix is merged to `master`, and the durable canonical `check:all` job passed on the exact stage-1 final-reconciliation head.

## Authoritative implementation and CI state

P15 implementation tranches:

- PR #41 — `Local LLM: close P15 negative-probe and silent-failure gaps`
  - final tested head: `534ca7334db23dccaa1057493a1c4dc5c41b76eb`
  - CI run: `33728972880` — success
  - merged `master`: `1feaaa951c987da55828d0774ded0cfdf0651b27`
- PR #42 — `Local LLM: fail closed on persisted settings errors`
  - exact tested head: `262316fd0a98377020f002b27aa4c6ed6ed2edbb`
  - CI run: `33734840062` — success
  - merged `master`: `9b3e8621fb41836365f0d5af2167564025279956`
- exact post-merge implementation `master` CI:
  - run: `33736402906`
  - exact SHA: `9b3e8621fb41836365f0d5af2167564025279956`
  - conclusion: success

Final reconciliation stage 1:

- PR #43 initial reconciliation head: `9338823063e1101a9abaf0155cbbc4e071f2f94d`
- CI run: `33741103743`
- conclusion: success
- new canonical job: `100606705422` — **Canonical `npm run check:all`** — success

Run `33741103743` therefore closes the literal aggregate-gate evidence gap rather than relying only on equivalence between separately executed steps.

## LLM-150 — Canonical quality gate

**COMPLETE.**

### Clean dependency installation and canonical aggregate command

The new ordinary-CI job `Canonical npm run check:all` performs:

1. exact repository checkout;
2. Node 22 setup;
3. Linux native build prerequisites;
4. Rust stable with `rustfmt` and `clippy`;
5. Rust dependency cache;
6. clean `npm ci`;
7. application icon generation required by Rust/build checks;
8. literal `npm run check:all`.

Job `100606705422` passed in run `33741103743` on exact head `9338823063e1101a9abaf0155cbbc4e071f2f94d`.

`package.json` defines `check:all` as the canonical aggregate of frontend, Rust, and generated-backend-contract validation. Thus this run directly proves the command required by LLM-150, not a hand-maintained approximation.

### Exact implementation-master CI matrix

Run `33736402906` on exact merged implementation `master` SHA `9b3e8621fb41836365f0d5af2167564025279956` passed the full ordinary matrix, including:

- **Frontend quality**
  - generated-tree rejection;
  - clean `npm ci`;
  - Tauri command-registration verification;
  - frontend IPC shape verification;
  - TypeScript typecheck;
  - ESLint;
  - Prettier formatting check;
  - Vitest;
  - frontend-only Vite preview smoke;
  - production frontend build.
- **Rust quality**
  - Moonshine runtime-manifest validation;
  - application icon generation;
  - `cargo fmt --check`;
  - Clippy with `-D warnings`;
  - Rust test suite;
  - backend failure matrix;
  - backend stress matrix.
- **Generated/release/static gates**
  - generated backend-contract exact drift check;
  - release metadata/icon validation;
  - release helper syntax;
  - Local LLM packaging-policy validation;
  - fail-closed release dependency-license collection.
- **Dependency/security audit**
  - production Node audit;
  - development Node audit;
  - Rust dependency audit/security check.
- **Local LLM compile proofs**
  - Linux x86_64;
  - macOS arm64;
  - macOS x86_64.
- **Supported macOS bundles**
  - arm64 unsigned Tauri app smoke bundle;
  - x86_64 unsigned Tauri app smoke bundle;
  - pinned Moonshine native runtime build/provenance;
  - self-contained native runtime verification;
  - packaged build provenance;
  - dependency-license collection.

Run `33741103743` then repeated the ordinary matrix with the new durable canonical `check:all` job and passed.

## LLM-151 — Local-provider negative probes

**COMPLETE.**

| Requirement | Implementation/test evidence |
| --- | --- |
| Google missing key fails; no Fake/Local fallback | `src-tauri/src/app/state.rs` — `configured_google_text_without_secret_fails_auth_instead_of_using_fake_provider`; requires `ProviderErrorKind::Auth`. |
| Local missing model fails; no Google/Fake fallback | `src-tauri/src/app/state.rs` — `configured_local_text_with_unknown_model_fails_without_cloud_fallback`; requires a Local model error. |
| Corrupt checksum fails installation | `src-tauri/src/ai/local/installer/adversarial_tests.rs` — `same_size_wrong_hash_is_rejected_without_installing_artifact`; complete staging/verify/promote path rejects same-size wrong-SHA content and leaves no final artifact/marker. |
| Corrupt GGUF fails load safely | `src-tauri/src/ai/local/runtime/llama.rs` — `corrupt_gguf_fails_load_with_safe_model_error`; actual llama.cpp load path receives malformed bytes, returns safe `ModelLoad`, does not expose the filesystem path, and leaves no loaded model. |
| Network-denied Local generation succeeds | `src-tauri/src/ai/local/text_model.rs` — `local_generation_succeeds_with_repository_network_boundary_denied`; P12 run `33663595759` independently proves real-GGUF CPU generation after entering an isolated network namespace. |
| Model switch/delete race does not corrupt runtime state | `src-tauri/src/ai/local/runtime/tests.rs` — `generation_and_delete_are_serialized_without_runtime_corruption`; delete waits behind generation, unload occurs before removal, and runtime/install state is clean afterward. Existing switch tests require unload-before-replacement and no fallback. |

The probes introduced in PR #41 passed in CI `33728972880` and remain green in subsequent exact implementation/reconciliation suites.

## LLM-152 — Privacy and silent-failure re-audit

**COMPLETE.**

### No prompt/output logging

`src-tauri/src/ai/local/text_model.rs` contains `local_prompt_memory_ambient_and_output_sentinels_never_enter_normal_logs`.

The test sends distinct private sentinels through the Local provider boundary for:

- typed/user prompt;
- system instruction;
- memory-derived context;
- ambient summary;
- generated output.

It proves those values reached the intended request/response path while none appear in captured normal logs. The repository log-capture liveness assertion prevents an empty/disconnected capture from producing a vacuous pass. Native llama.cpp logging is disabled; Local diagnostics contain identity/state/timing/token/category data, not prompt/output text.

### No provider substitution

`AppState::get_text_model()` uses the authoritative typed `TextProvider::{Google, Local}` selection. Missing Google credentials remain Google authentication failures. Missing/invalid Local state and Local runtime failures remain Local failures. There is no ordinary persisted Fake provider and no Local-to-Google substitution.

### No swallowed install/generation/startup failures that report success

The P15 audit found one real startup fallback defect in the then-current implementation:

```text
if let Ok(Some(json_str)) = db.get_setting("app_settings") {
    if let Ok((loaded, migrated)) = AppSettings::from_persisted_json(&json_str) {
        ...
    }
}
```

A database read failure or malformed settings JSON could therefore be ignored, allowing startup to continue with fresh defaults. For an existing profile, this could silently alter provider/model/privacy/behavior choices.

PR #42 changed this to fail closed:

- persisted-settings DB read failures abort startup with explicit safe context;
- persisted-settings decode failures abort startup with explicit safe context;
- successful migration/normalization behavior is preserved.

Regression test:

- `malformed_persisted_settings_abort_startup_instead_of_using_fresh_defaults`.

Installer and generation failures also remain explicit failure states rather than manufactured success.

### No stale optimistic Local-model state after backend rejection

`src/test/LocalLlmSettingsPanel.test.tsx` contains:

- `rolls back optimistic download state when install and status refresh both fail`.

The frontend restores the authoritative pre-install descriptor and clears optimistic progress before asking the backend for a fresh snapshot. If both installation and refresh fail, the UI cannot remain falsely stuck in `downloading`.

Existing model-selection rejection tests likewise prevent a rejected replacement from appearing selected.

### No preview-only Fake behavior reachable in production

`src/test/tauriBridgeRuntime.test.ts` proves:

- production-like execution uses native IPC;
- missing/malformed native IPC fails closed;
- query parameters, local storage, and arbitrary runtime browser globals cannot activate preview behavior;
- browser preview is selected only through the explicit development path.

Fake implementations therefore remain explicit test/development infrastructure rather than production fallbacks.

## LLM-153 — Final evidence inventory

**COMPLETE.**

### Canonical real CPU acceptance

Workflow: `Local LLM Real CPU Acceptance`

- run: `33663595759`
- accepted source SHA: `28aef16cbeeb91d9570177111560158811730b89`
- artifact ID: `9859917109`
- artifact name: `local-llm-real-cpu-28aef16cbeeb91d9570177111560158811730b89`
- artifact ZIP SHA-256: `b9e075822025dd98deac9e161d6f18281860148d13622e870a1ce75cc1c76b80`
- host: Linux x86_64, AMD EPYC 9V74 80-Core Processor
- runtime-reported available parallelism: `4`
- both model reports: `network_denial_probe_passed = true`

The pinned runtime does not expose first-token timing separately. The reports intentionally record `first_token_latency_ms = null`; no synthetic value is fabricated.

### SmolLM2-360M-Instruct Q4_K_M

Identity/install evidence:

- model ID: `smollm2-360m-instruct-q4-k-m`
- revision: `ab928a97ee49f3a015f35194879f68211291d6ca`
- artifact: `SmolLM2-360M-Instruct-Q4_K_M.gguf`
- SHA-256: `2fa3f013dcdd7b99f9b237717fa0b12d75bbb89984cc1274be1471a465bac9c2`
- exact bytes: `270,590,880`
- quantization: `Q4_K_M`
- license: `Apache-2.0`
- production installer verified: yes

CPU measurements:

- cold probe: `454 ms`
- warm probe: `328 ms`
- cold-load estimate: `126 ms`
- RSS delta: `311,816,192` bytes (~297.4 MiB)
- 60-token-cap ambient generation: `937 ms`
- ambient output: `34` tokens
- throughput: `36.255314 tokens/s`
- owner-drop/reload: `452 ms`, success

SmolLM2 remains the recommended/default Local model because the same-run usability evidence shows materially lower artifact/RAM cost and better measured latency/throughput than the larger alternative. This is a resource/usability decision, not a semantic-quality claim.

### Qwen3-0.6B Q4_K_M

Identity/install evidence:

- model ID: `qwen3-0-6b-instruct-q4-k-m`
- revision: `7bcae0bc7b0606f1e948f8cdb31b98a2c10635db`
- artifact: `Qwen_Qwen3-0.6B-Q4_K_M.gguf`
- SHA-256: `9acfc1e001311f34b4252001b626f2e466d592a42065f66571bff3790d4e1b14`
- exact bytes: `484,220,320`
- quantization: `Q4_K_M`
- license: `Apache-2.0`
- production installer verified: yes

CPU measurements:

- cold probe: `1,015 ms`
- warm probe: `545 ms`
- cold-load estimate: `470 ms`
- RSS delta: `747,933,696` bytes (~713.3 MiB)
- 60-token-cap ambient generation: `986 ms`
- ambient output: `14` tokens
- throughput: `14.187486 tokens/s`
- non-thinking output cleanliness: passed
- owner-drop/reload: `838 ms`, success

This second real model proves the runtime/catalog path is not hard-coded to one GGUF or model family.

## Packaging and licensing evidence

P13 reconciliation: `docs/RECONCILIATION_LOCAL_LLM_P13_20260902.md`.

Canonical P13 packaging acceptance:

- workflow run: `33686740197`
- exact merged source SHA: `abc5586814feeb4351672c01a4c8d9edc6578e7a`
- fixed pre-native-Local-LLM baseline SHA: `3253e52f7331ed7b03f0a3a4443eeef6d8e45aac`

arm64 artifact:

- artifact ID: `9868918280`
- ZIP SHA-256: `212c7a0a56b59eb340246c3d97f35a4f8e472839eccedb056550494cd4723919`
- baseline logical app bytes: `66,050,415`
- current logical app bytes: `70,908,429`
- delta: `4,858,014` bytes, `+7.3550%`
- embedded GGUF count: `0`

x86_64 artifact:

- artifact ID: `9869434458`
- ZIP SHA-256: `76759816ee0e4b4e2c7b7e5bf0c120d68fc839c757f99c6321ff4a47f3dba9d1`
- baseline logical app bytes: `78,952,231`
- current logical app bytes: `84,296,341`
- delta: `5,344,110` bytes, `+6.7688%`
- embedded GGUF count: `0`

Shipped Local runtime dependency/license evidence:

- `llama-cpp-2 = 0.1.154` — `MIT OR Apache-2.0`;
- `llama-cpp-sys-2 = 0.1.154` — `MIT OR Apache-2.0`;
- `utilityai/llama-cpp-rs` release commit `bed81ad4ab1a6c904b11d425608e50f976d8ea62`;
- vendored llama.cpp revision `5f55650a78f92aff4d48d671423e888fac0469ff` — MIT;
- native notices shipped under `src-tauri/native/macos/notices/LocalLlmRuntime/`;
- downloadable SmolLM2/Qwen weights are separately documented in `docs/LOCAL_LLM_MODEL_LICENSES.md` and are not redistributed in the application bundle.

## Environment and validation boundaries

Authoritative environments are intentionally separated by purpose:

- ordinary repository/Rust/frontend/generated-contract/audit/bundle validation: GitHub-hosted Ubuntu and supported macOS runners;
- exact merged P15 implementation matrix: run `33736402906`;
- final reconciliation canonical `check:all`: run `33741103743`, job `100606705422`;
- real-GGUF CPU/network-denied acceptance: run `33663595759` on Linux x86_64 / AMD EPYC 9V74;
- macOS packaging/size acceptance: run `33686740197` on supported macOS architectures.

The ChatGPT sandbox used during P15 was not treated as authoritative because it lacked a usable Rust toolchain. An attempted offline frontend install was also blocked by an uncached `zustand-4.5.7.tgz`. GitHub Actions therefore owns the canonical clean-install, Rust, frontend, contract, audit, and bundle evidence.

No GGUF model weights are committed or downloaded by ordinary CI.

## Known residual limitations

These are documented limitations, not silent fallbacks:

1. **Cancellation is cooperative.** The safe high-level `llama-cpp-2 0.1.154` API does not expose llama.cpp's abort-callback setter. Cancellation is observed between generated-token decode iterations; the implementation does not use unsafe raw FFI solely to force interruption.
2. **Shutdown is bounded rather than forcibly aborting native decode.** Application exit calls `begin_shutdown()` to reject new Local work and allows `LocalRuntimeManager::shutdown()` at most five seconds. If an in-progress native call does not return in time, a safe timeout is logged and process exit continues; the OS reclaims process state.
3. **First-token latency is unavailable from the pinned runtime surface.** P12 records `null` instead of inventing a value.
4. **V1 CPU policy means no GPU offload is requested.** The runtime selects zero GPU layers. Target-specific code may contain platform acceleration support, but V1 neither requires nor selects GPU offload.
5. **Local text is not fully local audio.** If Google TTS is selected, locally generated text is sent to Google TTS for synthesis. Gemini Live voice conversation remains cloud-based.

## Deferred fully local voice

Local LLM V1 ends at provider-neutral text generation:

```text
Ambient event ─┐
               ├─> TextModel -> Local or Google text -> existing TTS/speech
Typed message ─┘
```

Voice remains independent:

```text
Microphone -> Moonshine local ASR or Gemini Live audio -> Gemini Live session -> Gemini audio
```

The following path is explicitly deferred to a later phase:

```text
Microphone -> Moonshine ASR -> Local LLM -> local/provider-neutral TTS -> speakers
```

**`Moonshine -> Local LLM -> TTS` voice conversation is not part of Local LLM V1.**

## Final Gate reconciliation

All Local LLM V1 Final Gate requirements are supported by the evidence above and the phase-specific reconciliation documents:

1. explicit typed Local/Google text-provider selection exists;
2. production provider fallback to Fake is eliminated;
3. Local never silently falls back to Gemini;
4. SmolLM2-360M-Instruct Q4_K_M is pinned, installable, verified, and real-CPU tested;
5. Qwen3-0.6B Q4_K_M provides a second independently accepted catalog/runtime model;
6. ambient remarks work with Local;
7. typed messages work with Local;
8. post-install Local generation performs no network I/O, including real-GGUF P12 proof;
9. memory/transcript/privacy/speech state semantics remain covered and intact;
10. the installer is integrity-checked, cancellable, atomic, and path/symlink-safe;
11. generation/model-switch/delete ownership is race-safe and does not substitute another provider/model;
12. settings migration preserves existing Google users while new profiles default to Local;
13. settings UI distinguishes Local text from Gemini Live voice and Google TTS cloud behavior;
14. generated-contract and IPC command/shape gates cover the Local LLM surface;
15. ordinary CI remains independent of third-party GGUF downloads;
16. CPU real-model acceptance evidence is recorded for both supported catalog models;
17. runtime/model licensing and application distribution policy are reconciled;
18. literal `npm run check:all`, release/static gates, dependency audits, compile proofs, and supported macOS bundle jobs are green;
19. this reconciliation explicitly defers `Moonshine -> Local LLM -> TTS` fully local voice conversation.

## Conclusion

Local LLM V1 is ready for final docs-closeout CI and merge. P0 through P15 are implemented and evidenced. The remaining action is procedural: the exact PR #43 closeout head that updates this report/TODO must pass CI before squash merge; after merge, the resulting `master` push CI is the final repository-state confirmation.