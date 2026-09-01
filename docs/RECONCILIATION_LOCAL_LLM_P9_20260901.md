# Local LLM P9 Privacy and Security Reconciliation

Date: 2026-09-01

Base `master`: `89b2c3239d6b27c9be27452d3c0e0b4a7741662e`

Scope: `LLM-090` through `LLM-094` from `docs/TODO(20260831-081800).md`.

This document records the implementation and test evidence for the Local Text V1 privacy/security pass. It intentionally distinguishes code dependencies that ship with the application from model weights that are downloaded later by explicit user action.

## LLM-090 — No network I/O during Local generation

### Architecture

`LocalTextModel` receives an already-created `LocalModelInstaller` and an application-owned `LocalRuntimeManager`. Generation calls `LocalGenerationRuntime::generate`; it does not construct a `reqwest` request, Google text client, WebSocket, or model-download request.

The only Local LLM network implementation is the explicit installer transport in `src-tauri/src/ai/local/installer.rs`. `ReqwestLocalModelDownloadTransport` is used by `install_local_llm_model` after an explicit UI/backend install action. Generation does not call that transport.

### Repository network-denial proof

`src-tauri/src/test_support.rs` provides the established test-only `deny_network_for_scope()` guard. Existing Google provider tests prove that the same guard stops Google text/TTS at their production network boundaries before an HTTP send.

P9 adds `local_generation_succeeds_with_repository_network_boundary_denied` in `src-tauri/src/ai/local/text_model.rs`. It enables the repository network-denial guard, invokes the real `LocalTextModel` provider boundary with an injected deterministic Local runtime, and requires successful Local output. The runtime invocation is also asserted exactly once.

This combination is intentional:

- the Local test proves the Local provider boundary has no dependency on the repository network boundary;
- the existing Google test proves the guard is live and would fail a cloud text request before HTTP transmission;
- provider-routing tests from P7 already prove `text_provider = local` does not substitute Google or Fake behavior.

Real-GGUF CPU/network-denied acceptance remains a separate P12 task. P9 does not download third-party weights in ordinary CI.

## LLM-091 — Local LLM log privacy

The Local llama.cpp engine disables native llama.cpp logging via `LlamaBackend::void_logs()`. The reason is security-relevant: native diagnostics may include the model path, and the application instead exposes sanitized categories/metrics.

P9 adds the non-vacuous test `local_prompt_memory_ambient_and_output_sentinels_never_enter_normal_logs`.

It passes distinct private sentinels through the Local provider boundary for:

- typed/user prompt content;
- system instruction content;
- memory-derived content;
- ambient-summary content; and
- generated model output.

The injected runtime proves the private request values actually reached the Local generation boundary and the returned response proves the output sentinel actually came back. The test then verifies that none of those values occur in captured normal logs.

The test uses the repository `capture_logs()` helper and calls `assert_log_capture_live()`. That helper emits and requires `TALKING_MOOSE_TEST_LOG_CAPTURE_LIVE`, so an empty or disconnected log capture cannot produce a false privacy pass.

## LLM-092 — Runtime failure sanitization

The Local runtime uses stable `LocalRuntimeErrorKind` values and static safe messages. llama.cpp/binding failures are mapped with `map_err(|_| ...)`; raw native/binding error strings are not retained in `LocalRuntimeError`.

`LocalTextModel::map_runtime_error` then maps runtime kinds to provider-neutral `ProviderErrorKind` values and constructs the frontend-facing error using `ProviderError::from_kind`. The raw Local runtime message is discarded.

P9 expands the mapping regression test across every current `LocalRuntimeErrorKind` and injects a deliberately sensitive raw detail containing both a model path and prompt sentinel. The resulting frontend/provider error must contain none of those values.

Current category mapping:

| Local runtime error | Provider error |
| --- | --- |
| shutdown / cancellation | `closed` |
| unknown/missing/unsafe/unloadable model, missing chat template | `model` |
| invalid request / prompt too long | `setup` |
| initialization/context/tokenization/decode/output-decode/delete internals | `internal` |

Diagnostics remain limited to model identity, load state, thread/context configuration, timing/token counts, throughput, and safe error categories. Prompt/output text is not part of the Local runtime diagnostics shape.

## LLM-093 — Artifact path and symlink hardening

### Catalog-owned names

`src-tauri/src/ai/local/catalog.rs` validates model IDs and artifact filenames as one normal path component. Empty names, `.`, `..`, nested paths, and absolute paths are rejected. Artifact filenames must also end in `.gguf`. Source URLs must use HTTPS and revisions are pinned.

Callers cannot supply an arbitrary artifact path. Runtime model paths are derived from the catalog-owned tuple:

`<app model root>/<model id>/<revision>/<artifact filename>`

### Installer hardening added in P9

Before P9, final runtime loading canonicalized the installed artifact and rejected root escapes, and deletion already removed a model-directory symlink itself rather than following it. However, installer promotion used recursive directory creation, which could follow a pre-existing symlink in the root/model/revision hierarchy before the later runtime check.

P9 closes that installer-boundary gap:

- the configured LLM root must itself be a plain directory, not a symlink or other file type;
- `.staging` must be a plain directory;
- model-ID and revision directories must be plain directories;
- staging artifacts must be regular non-symlink files before verification;
- an existing final artifact may be replaced only when it is a regular file;
- install markers must be regular files;
- marker temporary files use UUID names plus `create_new(true)`;
- install-state validation uses `symlink_metadata` and rejects symlink directories, artifacts, and markers;
- deletion of a model-directory symlink removes only the link;
- recursive deletion of an ordinary model directory is regression-tested not to follow a revision symlink;
- if marker promotion fails after the verified GGUF was renamed into place, the promoted GGUF is removed so the failed install cannot leave a half-promoted payload that appears usable.

Unix adversarial tests cover a symlinked model root, symlinked staging directory, symlinked model directory, symlinked revision directory, symlinked installed artifact, symlinked marker target, and deletion of a tree containing a revision symlink. Outside-root sentinel files must survive unchanged.

These protections are in addition to the runtime manager's canonical-root check before a GGUF is opened.

## LLM-094 — Model/runtime license and download audit

### Model weights

The model weights are not embedded into the application bundle. They are downloaded only after an explicit user action, into the app-managed Local LLM model root, and are verified against the catalog's exact byte count and SHA-256 before promotion.

#### SmolLM2 360M Instruct Q4_K_M

Catalog source:

`https://huggingface.co/bartowski/SmolLM2-360M-Instruct-GGUF`

Pinned catalog revision:

`ab928a97ee49f3a015f35194879f68211291d6ca`

The bartowski GGUF repository identifies the model as Apache-2.0, and the upstream HuggingFaceTB SmolLM2-360M-Instruct repository contains the Apache License 2.0 text:

- https://huggingface.co/bartowski/SmolLM2-360M-Instruct-GGUF
- https://huggingface.co/HuggingFaceTB/SmolLM2-360M-Instruct/blob/cbcad7f4d160a10174f725b968ab6faf2a76399e/LICENSE

The catalog value `Apache-2.0` is therefore locked by a regression test.

#### Qwen3 0.6B Q4_K_M

Catalog source:

`https://huggingface.co/bartowski/Qwen_Qwen3-0.6B-GGUF`

Pinned catalog revision:

`7bcae0bc7b0606f1e948f8cdb31b98a2c10635db`

The bartowski model card identifies `Qwen/Qwen3-0.6B` as the original model. The retrieved bartowski page does not expose a separate license field, so this audit does not invent one. The upstream Qwen model and Qwen's official GGUF repository both identify the model as Apache-2.0 and publish/use the Apache license:

- https://huggingface.co/Qwen/Qwen3-0.6B
- https://huggingface.co/Qwen/Qwen3-0.6B/blob/c1899de289a04d12100db370d81485cdf75e47ca/LICENSE
- https://huggingface.co/Qwen/Qwen3-0.6B-GGUF

The catalog declaration is therefore `Apache-2.0` and is locked by regression test. If the conversion repository later publishes an independent/restrictive license or notice, the catalog/audit must be revisited before release.

### llama.cpp Rust integration

The application pins:

- `llama-cpp-2 = 0.1.154`
- `llama-cpp-sys-2 = 0.1.154`

`llama-cpp-2` 0.1.154 declares `MIT OR Apache-2.0` and identifies the `utilityai/llama-cpp-rs` repository:

- https://docs.rs/crate/llama-cpp-2/0.1.154/source/Cargo.toml.orig

The underlying llama.cpp project is MIT licensed:

- https://github.com/ggml-org/llama.cpp/blob/master/LICENSE

### Shipped dependency inventory

`scripts/collect_release_licenses.py` collects production npm dependencies and non-dev Rust dependencies reachable for both shipped macOS targets. It honors packaged notice/license files and Cargo `license-file`, records declared license metadata when standalone evidence is absent, and fails closed if neither legal text nor declared license metadata is available.

Because `llama-cpp-2` and `llama-cpp-sys-2` are direct production dependencies in `src-tauri/Cargo.toml`, they are in the reachable shipped Rust graph and are subject to that collector. Ordinary CI runs the dependency-license collection gate; unresolved evidence prevents the gate from passing.

Model weights are intentionally handled separately from this shipped-code dependency inventory because the GGUF files are not bundled application dependencies. Their source, pinned revision, identity, and license metadata are maintained in the Local model catalog and this audit.

## P9 result

Subject to the canonical CI gate for the exact implementation head, P9 establishes the following fail-closed properties:

1. Local generation is independent of the repository network boundary after installation.
2. Local prompts, memory/ambient-derived text, and generated output are not emitted to normal tracing logs.
3. Native/runtime failures are reduced to stable safe frontend categories/messages.
4. Catalog and installer paths cannot escape through user-controlled path components or pre-existing model-storage symlinks covered by the adversarial suite.
5. Model and runtime licensing has explicit provenance, and shipped Rust dependency evidence remains enforced by the existing fail-closed release-license collector.

P12 still owns real-model, CPU-only, network-denied generation evidence. This P9 pass deliberately does not treat injected-runtime testing as a substitute for that later real-GGUF acceptance gate.
