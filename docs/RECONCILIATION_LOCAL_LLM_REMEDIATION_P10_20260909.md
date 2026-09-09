# Local LLM Remediation P10 Reconciliation — 2026-09-09

## Status

**P10 COMPLETE — focused audit, optimized exact-head full validation, guarded merge, and exact merged-master CI all accepted.**

This record covers the source-audit portion of `LLMR-1000` / `LLMR-1002` and defines the evidence path for `LLMR-1001` / `LLMR-1003`. It does not pre-close merge- or CI-dependent tracker boxes.

Authoritative P10 base: `a1e8892e010d4ed226ca39ce487f08685fb79297`.

That base is the P9 tracker-closeout `master`. Exact post-merge P9 master CI `34373281618` completed successfully on the same SHA using the documentation-only fast path.

## Local/static verification boundary — LLMR-1000

The current implementation environment for this audit is GitHub source inspection plus repository/CI APIs, not a durable local checkout with the repository's Node/Rust build dependencies. Therefore this record does **not** claim local frontend or Rust command execution.

Available static/source checks performed here:

- exact source reads from `a1e8892e010d4ed226ca39ce487f08685fb79297`;
- focused production-path inspection for every `LLMR-1002` invariant;
- code search for stale settings writes, Fake/cloud/local fallback paths, diagnostics/private-payload fields, template ownership, and cancellation ownership;
- workflow inspection proving the explicit full-validation path contains the required frontend/Rust/dependency/RustSec/release/compile/bundle/canonical gates;
- P8 contract negative probes remain wired through `check:frontend`, and generated-contract drift remains owned by `check:generated-backend-contract`.

Unavailable local execution gates remain intentionally delegated to exact-head CI rather than being claimed as locally passed.

## Focused source audit — LLMR-1002

### 1. Installer cancellation path end-to-end

Audited production source: `src-tauri/src/ai/local/installer.rs`.

The authoritative in-flight state owns both a cancellation token and explicit `downloading` / `verifying` / `promoting` phase. Cancellation is checked after download, during the blocking SHA loop, after verification returns, before promotion, after rename, and at the marker-commit boundary. The valid marker is the durable install linearization point; cancellation accepted before marker commit cannot later become a valid install. Staging and any uncommitted promoted artifact are cleaned on cancellation/failure.

**Result:** no unresolved cancellation defect found.

### 2. Artifact verification/cache path end-to-end

Audited production source: `src-tauri/src/ai/local/installer.rs`.

Runtime admission distinguishes fast marker/shape validity from cryptographic runtime-use verification. Before runtime load, the current GGUF is SHA-verified unless a conservative successful fingerprint cache applies. The fingerprint includes canonical path and file metadata/identity, is invalidated on observed mutation, and is re-read after hashing to reject mutation during verification. SHA mismatch or unsafe path fails before llama.cpp; no provider/model substitution occurs.

**Result:** no unresolved artifact-integrity/cache defect found.

### 3. Persisted settings-version path end-to-end

Audited production source: `src-tauri/src/app/state.rs`.

`AppSettings::from_persisted_json()` parses raw JSON and checks `settings_version` before deserialization/normalization. A version greater than `CURRENT_SETTINGS_VERSION` returns typed `PersistedSettingsError::FutureVersion`. Startup only persists a normalized settings document after successful decode/migration, so a future-version rejection does not rewrite or strip unknown future fields.

**Result:** no unresolved future-version downgrade defect found.

### 4. Typed request snapshot/provider path end-to-end

Audited production sources:

- `src-tauri/src/app/request_snapshot.rs`;
- `src-tauri/src/commands/conversation/core.rs`.

`capture_text_request_settings()` captures one `AppSettings` clone and applies that same clone to the request's `CharacterConfig`. `send_text_message()` captures once before provider invocation, then uses that snapshot for provider/model selection, prompt memory/privacy input, character prompt configuration, and transcript-retention decisions. `get_text_model_for(&snapshot.settings)` does not re-read mutable provider/model settings.

**Result:** no split-snapshot defect found.

### 5. Ambient request snapshot/provider path end-to-end

Audited production source: `src-tauri/src/commands/ambient.rs`.

`process_ambient_event()` captures one request snapshot before its initial policy decision. Prompt construction, memory inclusion, character configuration, and provider/model selection use that snapshot. After generation, the existing current-state delivery gate intentionally re-evaluates whether speech is still allowed; that is a fail-closed post-generation suppression rule, not mixed request/provider semantics.

**Result:** no split-snapshot or privacy regression found.

### 6. Frontend patch-write/reconciliation path end-to-end

Audited production source: `src/stores/mooseStore.ts` plus Settings caller search.

Components express `SettingsPatch` intent. Serialized writes rebase each patch on the last successfully persisted authoritative baseline. Continuous writes coalesce patch intent only. Failure reconciliation fetches authoritative settings when possible and rebuilds optimistic state by replaying still-pending patches. Production Settings tabs use `updateSettingsPatch` / `updateSettingsContinuousPatch`; the prohibited `updateSettings({ ...settings, ... })` pattern appears only in remediation documentation describing what must not be used.

**Result:** no stale full-object overwrite path found.

### 7. Diagnostics Rust -> contract -> TypeScript -> UI path end-to-end

Audited production sources:

- `src-tauri/src/commands/local_llm_models.rs`;
- `src-tauri/src/ai/local/diagnostics.rs`;
- `src-tauri/src/ai/local/runtime/types.rs`;
- `src-tauri/src/lib.rs` command registration;
- `src/generated/backendContract.json` / exporter and P8 negative probes;
- `src/lib/tauriBridge.ts`;
- `src/components/Settings/LocalLlmSettingsPanel.tsx`.

The production Tauri command composes installer diagnostics and `LocalRuntimeManager::diagnostics()`. The command is registered and invoked by the typed bridge. Runtime diagnostics expose model identity, state, thread/context policy, generation-in-progress, safe error category, duration/token counts, and throughput. The UI renders those fields and installer error **kind**, not private error payloads.

No diagnostics shape field carries prompt text, generated output text, memory, transcript, credentials, URLs, or filesystem paths.

**Result:** production reachability and privacy boundary remain intact.

### 8. Chat-template and runtime cancellation truthfulness

Audited production sources:

- `src-tauri/src/ai/local/runtime/chat_template.rs`;
- `src-tauri/src/ai/local/text_model.rs`;
- Local runtime manager/llama cancellation call sites and architecture documentation.

Application code owns deterministic SmolLM2/Qwen ChatML rendering. `LlamaModel::chat_template(None)` is used only to retrieve the embedded template source for fail-closed family compatibility validation. Altered/generic/cross-family fixtures are rejected.

The provider-neutral `TextModel::generate()` API has no application cancellation parameter. Normal Local generation creates a fresh private cooperative token; comments do not claim a user-facing typed/ambient cancellation handle. The runtime still checks cancellation in the decode loop for its internal/runtime ownership cases.

**Result:** code, tests, and documentation agree on template and cancellation ownership.

### 9. Silent Fake/cloud/local fallback audit

Production provider selection is snapshot-explicit: Google constructs Google text with the selected Google model; Local constructs `LocalTextModel` with the selected Local model. Local missing/unsafe/unloadable artifacts fail closed. Local model deletion preserves selection as non-installed rather than choosing another model/provider. `FakeTextModel` exists as test infrastructure but source search found no production provider-selection reference to it.

**Result:** no Local -> Google -> Fake, Google -> Fake, or Local -> alternate-local silent fallback path found.

### 10. Prompt/output/credential leakage audit

`LocalRuntimeDiagnostics` has no prompt/output/private-context/credential/path fields. Local runtime errors use stable static messages/categories. Native llama.cpp logging is disabled with `LlamaBackend::void_logs()` because native diagnostics may include filesystem paths. Existing non-vacuous privacy tests inject prompt/system-memory/ambient/output sentinels and include positive controls proving the Local path was exercised.

**Result:** no new diagnostics/error leakage path found.

## P10 audit conclusion

No mandatory source defect from `docs/SPEC(20260905-141500).md` remains unresolved after the focused P10 re-read.

This conclusion is **source-inspection evidence**, not a substitute for final executable CI evidence. `LLMR-1001` remains open until the final exact P10 PR head passes the optimized explicit full-validation matrix.

## Full-validation performance correction

The first exact-head full-validation implementation proved the required acceptance surface but was operationally too slow. Run `34377196346` completed successfully on P10 head `41abfb4daf2738264f75c1796b4fbbd3cfa323a7`, but wall-clock time exceeded 20 minutes.

Profiling identified the dominant cost in the Intel macOS bundle smoke job:

- pinned Moonshine native rebuild: roughly four minutes;
- cold release-mode Tauri/Rust build on `macos-15-intel`: roughly seventeen minutes;
- only Cargo downloads were cached, not compiled target outputs or the pinned native runtime.

That runtime was rejected as an acceptable steady-state validation design even though the run was green.

The explicit `full-validation` label workflow is therefore optimized without reducing the P10 acceptance surface:

1. it verifies the labeled PR branch still resolves to the exact event head SHA;
2. every job checks out that exact SHA directly;
3. canonical `npm run check:all`, dependency/RustSec audit, release-static validation, and all three Local LLM compile proofs run in parallel;
4. failure/stress matrices are verified after the complete Rust suite instead of re-executing their named tests;
5. arm64 and x86_64 unsigned macOS **smoke** bundles use debug builds rather than optimized release builds;
6. the x86_64 smoke bundle is cross-built on the faster Apple-silicon `macos-15` runner using Rust's `x86_64-apple-darwin` target;
7. `prepare_moonshine_macos.sh` permits only the controlled arm64-host -> x86_64-target cross-build in addition to same-architecture builds and still enforces pinned source/ONNX hashes, Mach-O architecture, load-path hygiene, and ad-hoc signing after install-name mutation;
8. architecture-specific Cargo build outputs and pinned Moonshine/ONNX runtime outputs are cached with input-sensitive keys.

This changes CI execution mechanics only. The production release workflow remains release-mode and continues to own signed/notarized release acceptance. P10 still requires both unsigned application bundle smoke artifacts and exact provenance verification.

The packaging-policy gate now fails closed if the optimized explicit workflow loses any required architecture, the literal canonical gate, debug cross-architecture bundle smoke, native-runtime verification, or exact build-provenance verification.

## Required optimized full-validation evidence

The final optimized run must prove on one exact P10 head:

- frontend quality, Rust fmt/Clippy/complete tests, and generated-contract checks through literal `npm run check:all`;
- failure/stress matrix membership remains complete;
- dependency audit and the RustSec-generated `Security audit` check-run;
- release metadata/static gate;
- Linux Local LLM compile proof;
- macOS arm64 Local LLM compile proof;
- macOS x86_64 Local LLM compile proof;
- arm64 unsigned macOS application bundle smoke;
- x86_64 unsigned macOS application bundle smoke;
- bundle native-runtime integrity and exact build-provenance checks.

## P12/P13 boundary

The P10 source audit and CI-performance changes do not change model identity, model bytes, tokenizer/load semantics, prompt rendering, Qwen non-thinking framing, provider routing, or decode/generation behavior. Existing P12 real-model evidence therefore remains applicable unless a later P10 repair unexpectedly changes those semantics.

P13 signed/notarized execution, physical Mac audio/TCC acceptance, and human voice audition remain explicitly owner-deferred.

## Closure sequence

1. exact P10 PR head ordinary/path-scoped CI passes;
2. apply `full-validation` label and require the optimized exact-head full matrix to pass at an acceptable runtime;
3. guarded squash-merge the exact validated P10 head;
4. verify exact merged `master` ordinary CI;
5. create a documentation-only final reclosure that records the exact optimized full-validation/merge/master evidence, closes `LLMR-1000` through `LLMR-1003`, formally recloses `LLM-042`, `LLM-045`, `LLM-054`, `LLM-082`, `LLM-114`, `LLM-152`, and closes the Final Remediation Gate checklist;
6. docs-only closeout CI + expected-head guarded merge + exact final master CI.


## Final P10 closure evidence — 2026-09-09

P10 is formally accepted and the post-review Local LLM Final Gate is reclosed.

Exact evidence:

- accepted P9-closeout base: `a1e8892e010d4ed226ca39ce487f08685fb79297`;
- final P10 PR: #68;
- final validated PR head: `c94ef628eb2b60637a3488ff981136d5e8c9e145`;
- exact-head ordinary CI `34387806598`: **success**;
- optimized exact-head full-validation run `34387831210`: **success**;
- optimized full-validation wall clock: 10 minutes (`2026-09-09T18:14:49Z` → `2026-09-09T18:24:49Z`), replacing the rejected >20-minute design while preserving the P10 acceptance surface;
- guarded squash merge used exact expected head `c94ef628eb2b60637a3488ff981136d5e8c9e145`;
- resulting exact `master`: `2520a98265da14678c3df59223fcc1f598e653bd`;
- exact post-merge master CI `34389089979`: **success**.

The accepted optimized full gate included literal `npm run check:all`, dependency and RustSec auditing, release/static policy, Linux/macOS arm64/macOS x86_64 Local LLM compile proofs, and both unsigned macOS application bundle smoke builds with native-runtime and exact-provenance verification.

The focused source audit found no mandatory unresolved defect. Installer cancellation end-state tests justify restoring the cancellable installer Final Gate statement. `LLM-042`, `LLM-045`, `LLM-054`, `LLM-082`, `LLM-114`, and `LLM-152` are formally reclosed through the remediation evidence chain.

No P12 rerun is required: the accepted P10 work changes CI execution/provenance/bootstrap mechanics and documentation, not model identity/bytes, tokenizer/model loading semantics, prompt rendering, Qwen non-thinking framing, provider routing, or decode/generation behavior. Existing P12 real-model evidence remains applicable.

Signed/notarized P13 release execution, physical Mac audio/TCC acceptance, and human voice acceptance remain explicitly owner-deferred and are not misclassified as complete.

This closeout intentionally records the accepted implementation/full-validation/merge/master evidence. Its own documentation-only CI is procedural closeout evidence and is not recursively required to rewrite this record again.
