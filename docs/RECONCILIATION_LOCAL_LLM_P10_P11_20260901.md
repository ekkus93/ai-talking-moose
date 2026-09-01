# Local LLM P10/P11 Reconciliation — 2026-09-01

Base master: `ee5fbfbae0e51cfc9a8c6364aa2bfe74697a8a62`

This reconciliation distinguishes already-implemented coverage from the residual tests added in the P10/P11 Ralph tranche. The source TODO predates most of the Local LLM implementation and its unchecked boxes are not authoritative implementation status by themselves.

## P10 — Generated Contract and IPC Integrity

### LLM-100 — backend contract exporter

**Status: implemented; ordinary CI is authoritative.**

Evidence:

- `src-tauri/src/bin/export_frontend_contract.rs` emits authoritative `AppSettings` plus representative Local LLM IPC values for `LocalModelInstallError`, `LocalModelDescriptor`, `LocalModelDiagnostics`, and `LocalModelInstallProgress`.
- `src/generated/backendContract.json` is generated from that Rust exporter.
- `npm run check:generated-backend-contract` regenerates the contract, fails on drift, and then executes the frontend shape verifier.
- `npm run check:all` includes this gate.

### LLM-101 — TypeScript shape verification

**Status: implemented.**

Evidence:

- `scripts/check_frontend_contract_shapes.mjs` compares Rust representative object keys with TypeScript interface keys.
- It verifies the JSON category of each represented property.
- The script explicitly reports its residual limits: numeric narrowing, enum completeness, optionality semantics, primitive command arguments, and per-command type associations are not claimed.

### LLM-102 — command registration verification

**Status: implemented with residual negative probe added in this tranche.**

Evidence:

- `scripts/check_tauri_command_contract.mjs` extracts frontend `invoke("...")` names and Rust `tauri::generate_handler!` registrations and fails on missing registrations.
- The Local LLM lifecycle/test commands are already included in the production bridge and Rust registration list.
- This tranche adds a deliberate `test_local_llm_model` registration-rename simulation and requires the same comparator to report the original frontend command as missing. A gate that could not detect that probe now fails itself.

## P11 — Frontend and Rust Test Coverage

### LLM-110 — settings/provider tests

**Status: covered.**

Existing tests cover settings migration/idempotence/defaults, Google-without-key failure, Local missing-model failure, no Fake fallback, and independent provider-specific model selections.

Residual added here:

- `src-tauri/src/app/provider_switch_tests.rs` proves provider choice is snapshotted when a `TextModel` is constructed: a captured Local model stays Local after settings switch, while the next model uses Google.
- The same module proves switching providers does not erase either provider-specific model ID.

### LLM-111 — catalog/installer tests

**Status: covered by the existing catalog and adversarial installer suites.**

Coverage includes:

- catalog uniqueness/path-safety/SHA/revision/default/license metadata validation;
- wrong SHA and wrong byte count;
- truncated and oversized artifacts;
- cancellation/interruption and stale staging cleanup;
- duplicate install rejection;
- failed atomic promotion;
- root/staging/model/revision/artifact/marker symlink attacks;
- safe delete behavior that does not follow model-tree links outside the owned root.

No additional installer behavior was invented in P11; P9 already hardened the production path and added the adversarial cases.

### LLM-112 — runtime manager tests

**Status: covered, with explicit residual lifecycle/serialization tests added.**

Existing runtime tests cover request bounds, missing install, pre-cancellation, model switching, delete coordination, shutdown, and privacy-safe diagnostics.

Residual added here:

- same-model load reuse is explicit;
- explicit unload followed by reload is explicit;
- the `RuntimeState` mutex that encloses native generation is stress-tested with two workers and must report a maximum native-generation concurrency of exactly one.

The production manager additionally serializes generation/load/delete/shutdown through its async `operation_lock`; the state-level test deliberately verifies the innermost native-generation exclusion rather than merely asserting a field exists.

### LLM-113 — Local text caller integration

**Status: unit/injected boundary covered; real-GGUF success remains a P12 acceptance concern.**

Current caller/provider coverage is intentionally decomposed rather than manufacturing an installed GGUF in ordinary CI:

- ambient requests use `AppState::get_text_model()` and have explicit selected-Local and selected-Google routing tests;
- typed `send_text_message` has explicit selected-Local failure/recovery and selected-Google failure tests;
- `LocalTextModel` has successful injected-runtime generation tests, including bounded typed and ambient requests and network-denied generation;
- conversation and ambient prompt tests cover memory Off/On behavior;
- transcript retention Off/On is covered directly;
- typed Local failure restores `Idle` and queues no playback;
- muted/post-generation and standalone speech state semantics remain provider-neutral and are covered by the pre-existing speech/conversation tests.

Ordinary CI must not synthesize a fake installed GGUF just to claim an end-to-end Local success. P12 owns the missing real-artifact CPU success evidence and will run outside the model-weight-free unit gate.

### LLM-114 — frontend settings tests

**Status: covered; residual lifecycle/failure tests added here.**

Existing `SettingsModal` coverage proves Local/Google selection, explicit no-auto-download behavior, model selection, install, test, delete-with-selection-preserved, Google controls, and cloud/local voice/TTS disclosure.

`src/test/LocalLlmSettingsPanel.test.tsx` adds:

- live download byte progress rendering;
- distinct verifying state;
- explicit cancel request and result;
- install/checksum-style failure reconciliation to authoritative backend status;
- backend model-selection rejection without a silent replacement.

## Boundary before P12

P10/P11 do **not** treat injected Local generation as evidence that a bundled llama.cpp build can load the pinned GGUFs on a real CPU. The next phase must provide that evidence through a separate real-model acceptance harness while keeping ordinary CI model-weight-free.
