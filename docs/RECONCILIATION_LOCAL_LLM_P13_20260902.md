# Local LLM P13 Packaging/CI Reconciliation — 2026-09-02

## Status

**P13 is COMPLETE.**

The implementation merged to `master` as:

`abc5586814feeb4351672c01a4c8d9edc6578e7a` — `Local LLM: implement P13 packaging and CI gates`

The implementation PR was #38. Its canonical ordinary CI run was `33677289000`, which passed on exact PR head `0abace3895b33a2e604586122f2914c73951ddda` before the squash merge.

The final evidence gate was the manually dispatched workflow `Local LLM P13 Packaging Acceptance`, run `33686740197`, executed on exact merged `master` SHA `abc5586814feeb4351672c01a4c8d9edc6578e7a`. Both macOS architecture jobs passed and both machine-readable artifacts were independently reviewed.

## P13 requirements

P13 covers:

- LLM-130 — supported-target native runtime compilation;
- LLM-131 — ordinary CI remains model-weight-free;
- LLM-132 — shipped runtime dependency/license evidence;
- LLM-133 — measured application bundle-size impact and proof that GGUF weights are external.

## LLM-130 — Supported-target native compilation

**COMPLETE.**

Ordinary CI proves the Local LLM runtime compiles and its CPU policy proof runs on all supported CI targets:

- Linux x86_64;
- macOS arm64;
- macOS x86_64.

Run `33677289000` passed all three `Local LLM compile proof` jobs. The application remains pinned to `llama-cpp-2 = 0.1.154` and `llama-cpp-sys-2 = 0.1.154` with default features disabled. The fail-closed packaging policy gate verifies the target matrix, exact binding pins, CPU/default-feature policy, and absence of a developer-local/system llama.cpp dependency.

## LLM-131 — Ordinary CI remains model-weight-free

**COMPLETE.**

The P13 packaging policy gate proves that ordinary CI, release gates, and `npm run check:all` do not invoke real-GGUF acceptance or model-host downloads. Real model execution remains isolated to manually dispatched acceptance workflows.

Additional fail-closed protections are in place:

- `*.gguf` and `*.GGUF` are ignored by Git;
- Tauri resources remain notice-only;
- macOS bundle verification rejects any embedded GGUF;
- unit/runtime tests use injected runtimes rather than downloading model weights;
- the generated-contract exporter does not instantiate or download a model.

The final P13 bundle evidence independently confirmed `gguf_file_count = 0` for both the baseline and current app bundles on both architectures.

## LLM-132 — Dependency and license inventory

**COMPLETE.**

The shipped Local LLM native runtime is represented by:

- `llama-cpp-2 = 0.1.154` — `MIT OR Apache-2.0`;
- `llama-cpp-sys-2 = 0.1.154` — `MIT OR Apache-2.0`;
- upstream `utilityai/llama-cpp-rs` tag `0.1.154`, commit `bed81ad4ab1a6c904b11d425608e50f976d8ea62`;
- vendored llama.cpp revision `5f55650a78f92aff4d48d671423e888fac0469ff` — MIT.

Release-license collection requires both llama binding crates at the exact expected version and fails if usable license evidence is missing. Both macOS smoke-bundle jobs in CI run `33677289000` passed dependency-license collection and self-contained bundle verification with the stronger P13 requirements.

Native runtime notices are shipped under `src-tauri/native/macos/notices/LocalLlmRuntime/`. Downloadable model licenses remain documented separately in `docs/LOCAL_LLM_MODEL_LICENSES.md` because GGUF model weights are not redistributed in the application bundle.

## LLM-133 — Canonical bundle-size impact evidence

**COMPLETE.**

Canonical workflow run:

- workflow: `Local LLM P13 Packaging Acceptance`;
- run: `33686740197`;
- event: `workflow_dispatch`;
- branch: `master`;
- current SHA: `abc5586814feeb4351672c01a4c8d9edc6578e7a`;
- fixed pre-native-Local-LLM baseline SHA: `3253e52f7331ed7b03f0a3a4443eeef6d8e45aac`.

The workflow verified the baseline ancestry and built both the baseline and current `.app` independently on each architecture using the same deployment floor. The measurement is the sum of regular-file logical byte sizes inside the `.app`; symlinks are excluded.

### arm64

Artifact:

- artifact ID: `9868918280`;
- artifact name: `local-llm-p13-packaging-arm64-abc5586814feeb4351672c01a4c8d9edc6578e7a`;
- artifact ZIP SHA-256: `212c7a0a56b59eb340246c3d97f35a4f8e472839eccedb056550494cd4723919`.

Measured evidence:

- baseline logical app bytes: `66,050,415` (~62.99 MiB);
- current logical app bytes: `70,908,429` (~67.62 MiB);
- app delta: `4,858,014` bytes (~4.63 MiB), **+7.3550%**;
- baseline executable bytes: `25,395,600`;
- current executable bytes: `30,122,560`;
- executable delta: `4,726,960` bytes, **+18.61%**;
- baseline file count: `649`;
- current file count: `680`;
- baseline GGUF count: `0`;
- current GGUF count: `0`;
- embedded model weights: **no**.

### x86_64

Artifact:

- artifact ID: `9869434458`;
- artifact name: `local-llm-p13-packaging-x86_64-abc5586814feeb4351672c01a4c8d9edc6578e7a`;
- artifact ZIP SHA-256: `76759816ee0e4b4e2c7b7e5bf0c120d68fc839c757f99c6321ff4a47f3dba9d1`.

Measured evidence:

- baseline logical app bytes: `78,952,231` (~75.29 MiB);
- current logical app bytes: `84,296,341` (~80.39 MiB);
- app delta: `5,344,110` bytes (~5.10 MiB), **+6.7688%**;
- baseline executable bytes: `25,787,992`;
- current executable bytes: `31,001,048`;
- executable delta: `5,213,056` bytes, **+20.22%**;
- baseline file count: `649`;
- current file count: `680`;
- baseline GGUF count: `0`;
- current GGUF count: `0`;
- embedded model weights: **no**.

The downloaded ZIP digests were independently recomputed after retrieval and exactly matched the SHA-256 digests reported by GitHub for both artifacts.

## P13 conclusion

The native Local LLM integration adds approximately 4.6–5.1 MiB to the logical macOS application bundle in the measured builds. That increase is attributable to shipped runtime/code/notices rather than model weights. The much larger GGUF artifacts remain external, explicit user downloads and are absent from both measured application bundles.

All P13 acceptance conditions are therefore satisfied. The next implementation phase is **P14 — Documentation and Developer Experience**.
