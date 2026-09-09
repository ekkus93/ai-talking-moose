# Local LLM Remediation P9 Reconciliation — 2026-09-09

## Status

**P9 documentation and historical reconciliation implementation is accepted and the P9 tracker closeout is prepared.**

This record covers `LLMR-900` through `LLMR-903`. P9 changes documentation and historical interpretation only; P10 remains the final focused source audit and remediation Final Gate.

Implementation base: `8d2e318cec837de3a26c35fe9afc3c0f5847ea69`, the P8 tracker-closeout `master` produced by PR #64. Exact post-closure master CI `34309719864` completed successfully on that SHA.

P9 implementation PR #65 used exact head `4537c4a0e4937ccef8143ca89622e3e41196fb89`. Exact-head CI `34327812759` completed successfully, including the then-canonical `npm run check:all` gate. PR #65 was squash-merged with the expected-head guard to exact `master` SHA `c3400e78d4a265a282d765ee762b2c431edb6b34`; exact post-merge master CI `34330866678` completed successfully.

Before this tracker-only closeout, CI performance remediation PR #66 was separately accepted. Its exact head `2798815cb228672eb52db305b1c349b0ad486b07` passed CI `34338993830` and was guarded-squash-merged to exact `master` SHA `6008939c475e67620dc0003bb6bb3d27eb045f80`. Exact master push CI `34372468221` passed on that SHA using the new CI-plumbing fast path. PR #66 changes CI orchestration and matrix execution ownership; it does not change Local LLM model identity, runtime behavior, prompt rendering, provider routing, or P9 documentation semantics.

P9 is documentation/reconciliation only. It does not change model identity, runtime loading, installer behavior, prompt bytes, chat-template rendering, provider routing, tokenization, or generation semantics.

## LLMR-900 — Historical tracker addendum

The original tracker `docs/TODO(20260831-081800).md` remains a historical record of the 2026-09-03 Local LLM V1 closure. Its existing `[x]` evidence is intentionally preserved rather than rewritten as if the earlier CI and acceptance never occurred.

The 2026-09-05 source review later reopened only these claims:

- `LLM-042` — verification-phase installer cancellation truthfulness;
- `LLM-045` — installer cancellation semantics and production runtime-diagnostics exposure;
- `LLM-054` — production reachability of Local runtime diagnostics;
- `LLM-082` — UI Cancel truthfulness during verification;
- `LLM-114` — frontend cancellation test end-state coverage;
- `LLM-152` — the silent-failure audit missed acknowledged-but-ignored verification cancellation;
- only the **cancellable** part of the original Final Gate statement that the installer was fully integrity-checked, cancellable, atomic, and path-safe.

The active remediation authority is `docs/SPEC(20260905-141500).md` plus `docs/TODO(20260905-141500).md` and the P0-P9 reconciliation records.

Reclosure mapping after the implementation phases:

| Historical claim | Post-review remediation evidence | State before P10 final gate |
| --- | --- | --- |
| `LLM-042` | P1 `LLMR-100`-`104`: explicit installer phases, blocking SHA worker, cooperative verification cancellation, pre-promotion/marker-boundary checks, complete cancellation end-state matrix | Remediated; formal Final Gate reclosure is performed in P10 |
| `LLM-045` | P1 cancellation repair plus P3 production `get_local_llm_diagnostics` path and P8 command-contract negative proof | Remediated; formal Final Gate reclosure is performed in P10 |
| `LLM-054` | P3 `LLMR-300`-`303`: live runtime diagnostics through production Tauri IPC/UI with liveness/privacy tests | Remediated; formal Final Gate reclosure is performed in P10 |
| `LLM-082` | P1 truthful installer state/cancellation plus P7 UI cancel end-state regression | Remediated; formal Final Gate reclosure is performed in P10 |
| `LLM-114` | P7 replaces dispatch-only cancellation coverage with cancel -> non-installed -> explicit retry success lifecycle proof | Remediated; formal Final Gate reclosure is performed in P10 |
| `LLM-152` | P1/P2/P3/P4/P5/P7 deterministic probes plus P10 final silent-fallback/privacy audit | Remediation evidence complete through P9; formal reclosure awaits P10 audit |
| Installer “cancellable” Final Gate clause | P1 production cancellation semantics and P7 frontend end-state proof | Implementation evidence complete; Final Gate wording remains formally reopened until P10 |

This separation preserves historical evidence while preventing the old 2026-09-03 Final Gate line from being mistaken for the current remediation state.

## LLMR-901 — Post-review finding inventory and exact remediation evidence

### Finding inventory

| Finding | Severity | Review finding | Remediation |
| --- | --- | --- | --- |
| F1 | High | Cancel was acknowledged while SHA verification could continue to successful install | P1: authoritative in-flight phase/token record; verification cancellation checks; pre-promotion and marker-boundary linearization; cancellation end-state tests |
| F2 | Medium | Runtime diagnostics existed but were not production reachable | P3: composed `get_local_llm_diagnostics` Tauri path, generated contract, TypeScript bridge, Settings diagnostics UI, liveness/privacy tests |
| F3 | Medium | One request could observe settings from multiple moments | P4: immutable `TextRequestSettingsSnapshot`, `get_text_model_for(snapshot)`, barrier-controlled typed/ambient A/B consistency tests |
| F4 | Medium | A newer persisted settings schema could be destructively normalized/downgraded | P4: inspect version before normalization, typed future-version failure, preserve persisted future document unchanged |
| F5 | Medium | Marker-valid installed GGUF bytes were not cryptographically reverified before runtime load | P2: first-use SHA verification, conservative in-memory file-identity cache, invalidation/re-hash, fail closed before llama.cpp |
| F6 | Medium/Low | Large synchronous SHA verification ran on an async executor worker | P1: streaming hash moved to `spawn_blocking` with bounded chunk cancellation checks |
| F7 | Medium/Low | Stale frontend render snapshots could overwrite unrelated newer settings | P5: patch-intent component API and deterministic stale-callback regression |
| F8 | Low | Status refresh could regress/lie about the `verifying` phase | P1 truthful phases plus P7 frontend lifecycle proof |
| F9 | Low | Installer `last_error` chronology depended on `HashMap` iteration | P2 monotonic ordered error state and deterministic two-model chronology test |
| F10 | Low | Redirect policy did not independently enforce HTTPS on every redirect target | P2 HTTPS-only redirect decision with finite hop bound and offline negative probe |
| F11 | Low | Comments incorrectly attributed supported chat-template application to llama.cpp | P6 documents application-owned SmolLM2/Qwen rendering and deterministic embedded-template compatibility signatures |
| F12 | Maintainability | Installer/runtime responsibilities were mixed enough to obscure correctness invariants | P2 separates redirect, in-flight state, ordered errors, marker validity, runtime verification cache, SHA verification, promotion/marker commit, and cleanup where correctness requires it; no broad refactor beyond verified invariants |

### Phase evidence

| Phase | Main implementation evidence | Accepted CI/merge evidence |
| --- | --- | --- |
| P0/P1 | PR #46, head `3888cf671399a67da70a7c39531e29b37922f70d` | exact-head CI `34009203437`; merged `6f9aa2dac6f99d69eaa86d8e9ea666173124e7eb`; post-merge CI `34010580951` |
| P2 | PR #48, head `9a6ea2a6d090450c87ac01589aa4493caef8e617` | exact-head CI `34015871421`; merged `51ad339b8c5faf129b5f06a03da153b657988ae9`; post-merge CI `34017283156` |
| P3 | PR #50, head `ebcc36ebf6e985b443a808d39eede80780e6efac` | exact-head CI `34023525137`; merged `d0279cd479669d5d4aaf58a6988d463174611c5b`; post-merge CI `34025909865` |
| P4 | PR #53, head `c0b8fa536a1fd94a621c52c534ec90004479d7f0` | exact-head CI `34053798461`; merged `9ed82604b54308951bcd172ee82402729f6a60e0`; post-merge CI `34055657716` |
| P5 | PR #57, head `38ff0ad2129d203bd3bd9677123d0c4c482bc7d1` | exact-head CI `34071222525`; merged `b2251de2317b6e5c8832f0b1829847668e2ad228`; post-merge CI `34089027966` |
| P6 | PR #59, head `31ac0b76b99c28d0f5225c0c6ebcfd8176c78231` | exact-head CI `34144002755`; merged `0782091c8f46bb6471b86a2e724364e10b611077`; post-merge CI `34146860974` |
| P7 | PR #61, head `40a531319b49bb1f51306eb4ca07b430558b86d8` | exact-head CI `34274531469`; merged implementation `d80f6ab804d8dcc698836283d87bab77da9fcf65`; post-merge CI `34278242897`; accepted P7 closure master `e3533f5cab2c8730683701b11277871e8042c7fb` passed CI `34287904554` |
| P8 | PR #63, head `f22d161d37a08e8aa52eebd0b815a542b86783e6`; closure PR #64 head `da447b3aee7caa6c49edaf3cce9d971841620e1e` | implementation CI `34290605186`; merged implementation `32fb0b1bd82ee548311510a98e0d6f237c4f06f4`; post-merge CI `34294230896`; closure CI `34306366112`; accepted P8 closure master `8d2e318cec837de3a26c35fe9afc3c0f5847ea69` passed CI `34309719864` |
| P9 | PR #65, head `4537c4a0e4937ccef8143ca89622e3e41196fb89` | exact-head CI `34327812759`; merged `c3400e78d4a265a282d765ee762b2c431edb6b34`; post-merge CI `34330866678` |

The phase-specific reconciliation documents remain the detailed authority for exact tests, tree identities, superseded failed authoring runs, and closure mechanics.

### Evidence classes

The remediation intentionally distinguishes evidence types:

- **source inspection** — the 2026-09-05 finding inventory and later focused audits identify ownership/control-flow defects and documentation mismatches;
- **deterministic tests/probes** — barrier-controlled races, installer fixtures, privacy sentinels, command/shape mutation probes, and frontend lifecycle tests prove specific invariants without public-network/model/hardware dependence;
- **ordinary CI** — path-scoped frontend/Rust/contract/dependency/release/Local LLM gates selected by changed inputs; explicit full validation retains the complete cross-platform compile/bundle matrix and literal canonical `npm run check:all`;
- **real-model evidence** — the accepted P12 real CPU/network-denied model run from the original V1 remains the generation-semantic evidence because P0-P9 do not materially change those semantics;
- **deferred physical/release evidence** — signed/notarized release execution, physical Mac audio/TCC acceptance, and human voice audition are owner-deferred and are not inferred from CI.

### P12 real-model rerun decision

**No P12 rerun is required for the remediation as implemented through P9.**

P0-P9 change installer cancellation/integrity, runtime artifact admission, diagnostics reachability, request settings consistency, frontend settings persistence intent, documentation truthfulness, regression coverage, and contract/CI proof. They do not materially change pinned model identities, the llama.cpp generation backend, tokenizer/model loading semantics after successful artifact admission, rendered SmolLM2/Qwen prompt framing, Qwen non-thinking control semantics, provider routing, or decode/generation behavior.

P2 can reject a tampered artifact earlier, but an accepted pinned artifact reaches the same runtime/generation path. P6 changes template ownership documentation and compatibility identification without changing rendered prompt bytes. P7-P9 are test/documentation/contract work only. If P10 unexpectedly introduces a generation-semantic change, this decision must be reopened before final closure.

## LLMR-902 — Developer/architecture documentation

`docs/LOCAL_LLM_ARCHITECTURE.md` is updated to describe the production code rather than the original pre-review assumptions. Its post-review section covers:

- immutable request settings snapshot ownership;
- production Local runtime diagnostics path and privacy boundary;
- installer `downloading` / `verifying` / `promoting` phases and cancellation linearization;
- first-runtime-use artifact SHA verification and conservative cache invalidation;
- future settings-version fail-closed behavior;
- patch-oriented frontend settings writes;
- product-visible versus private/cooperative generation cancellation semantics;
- application-owned SmolLM2/Qwen chat-template rendering and embedded-template compatibility validation.

## LLMR-903 — Residual limitations and deferred gates

The following are explicit supported boundaries, not silent success claims:

1. **Cooperative native generation cancellation remains limited.** The safe high-level `llama-cpp-2 0.1.154` surface does not expose a product-level abort handle. Normal typed/ambient requests do not expose cancellation to callers.
2. **Shutdown is bounded, not a forced native abort.** New Local work is rejected during shutdown; teardown waits within the established five-second application bound and then allows process exit/OS cleanup rather than using unsafe forced interruption.
3. **Local LLM V1 is CPU-only by policy.** Zero GPU layers are requested; GPU-offload controls are outside V1 scope.
4. **First-token latency remains unavailable.** The pinned runtime surface does not provide a trustworthy separate first-token measurement, so accepted P12 evidence records it as unavailable/null rather than inventing a number.
5. **Fully local voice remains deferred.** Gemini Live remains the realtime voice provider; Moonshine-finalized transcripts still feed that cloud conversation path, and Google TTS remains cloud when selected.
6. **P13 signed/notarized release execution remains owner-deferred.** Existing P13 packaging, bundle-size, GGUF-exclusion, dependency-license, and notice evidence remains preserved and accepted; P9 does not claim a signed/notarized release candidate was executed.
7. **Physical Mac and human voice acceptance remain deferred.** Ordinary CI and unsigned bundle smoke tests do not substitute for physical microphone/audio/TCC acceptance or human voice audition.

## P9 closure boundary

P9 implementation is accepted through exact post-merge master CI. This documentation-only closeout marks only `LLMR-900` through `LLMR-903` complete in the authoritative tracker.

The closeout branch is based on exact `master` `6008939c475e67620dc0003bb6bb3d27eb045f80`, after CI optimization PR #66 and its successful exact-master push CI `34372468221`. The closeout must itself pass the docs-only exact-head CI path and be expected-head guarded when merged. Its resulting exact master push must also pass before P9 is considered procedurally accepted.

P10 remains entirely open. P9 does not pre-close `LLMR-1000` through `LLMR-1003` or the Final Remediation Gate checklist.
