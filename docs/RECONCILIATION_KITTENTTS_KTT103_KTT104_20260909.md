# AI Talking Moose — KittenTTS KTT-103 / KTT-104 Reconciliation

**Tracker:** `docs/TODO(20260909-120003).md`  
**Tasks:** `KTT-103 — Add provider/model/voice capability metadata`; `KTT-104 — Carry settings and metadata through the generated contract`  
**Implementation base:** `0a0769599c450b3fb74834d0a1ed09f93827eff6`  
**Final implementation head:** `e5d8b5c23d24f9805d3ff6fa575857b72ad0ea9f`  
**Implementation PR:** #77  
**Merged implementation master:** `a2f12865034c9c44e8996ee74d38df31f156288f`  
**Status:** KTT-103 / KTT-104 COMPLETE — exact-head CI, guarded merge, and exact merged-master CI accepted 2026-09-09

## Implementation

KTT-103 adds an authoritative typed TTS capability catalog in `src-tauri/src/ai/tts_catalog.rs`. Standalone TTS providers and Gemini Live remain separate domains rather than overloading one voice list.

The standalone provider catalog now describes:

- stable provider IDs and display names;
- local/cloud classification;
- Google standalone model and voice IDs;
- Local KittenTTS Mini 0.8 model and the eight accepted local voice IDs;
- source sample rate;
- speaking-rate capability;
- pitch capability;
- installation requirement;
- license/runtime summary.

The Local provider truthfully reports 24,000 Hz source audio, speaking-rate support, no pitch support, installation required, and the permissive production stack accepted in P0: application-owned Rust ONNX integration, `ort 2.0.0-rc.13`, Microsoft ONNX Runtime 1.23.2 CPU artifacts, and `piper-plus-g2p 0.4.0` with the permissive-only dependency graph.

Gemini Live voices are exposed through a separate `GeminiLiveVoiceCatalog`. Local voice IDs therefore cannot become valid Gemini Live voice IDs merely because both are speech features.

KTT-104 carries the metadata through the generated Rust → JSON → TypeScript contract. The implementation:

- extends the Rust representative frontend-contract export with `TtsCatalog`;
- regenerates `src/generated/backendContract.json`;
- adds matching TypeScript TTS catalog types;
- adds a checked `getTtsCatalog()` conversion path rather than an unchecked handwritten JSON cast;
- updates native and browser-preview bridges;
- updates production-like frontend dispatcher fixtures;
- registers the `get_tts_catalog` Tauri command;
- extends command-registration and frontend contract tests;
- adds `scripts/check_tts_contract_negative_probe.mjs`, which deliberately removes `supports_pitch` from the Rust representative shape and requires the frontend shape gate to reject the drift.

The legacy Google-only voice command remains temporarily for the existing UI and is not the authoritative capability model for new Local TTS work.

## Validation and acceptance evidence

The first temporary materializer run `34421588071` failed before the product metadata code compiled because its Ubuntu runner omitted `libasound2-dev`, causing `alsa-sys` to fail while locating `alsa.pc`. A separate attempt to repair that temporary workflow encountered the repository token restriction on workflow mutation; the product branch itself was not weakened to work around either CI-plumbing problem.

Corrected materialization/targeted validation run `34432769629`: PASS. It covered the focused Rust TTS catalog tests, frontend contract shape checks, the deliberate TTS negative-drift probe, Tauri command-registration checks, TypeScript typechecking, focused frontend contract tests, and Rust formatting.

The iterative implementation history was then collapsed to one clean product commit directly on accepted master `0a0769599c450b3fb74834d0a1ed09f93827eff6`.

Exact PR head:

`e5d8b5c23d24f9805d3ff6fa575857b72ad0ea9f`

Exact-head ordinary CI:

- run `34433358793`: PASS;
- event: `pull_request`;
- head SHA: `e5d8b5c23d24f9805d3ff6fa575857b72ad0ea9f`.

PR #77 was merged only after that exact head was accepted, with the merge guarded against head movement.

Resulting master:

`a2f12865034c9c44e8996ee74d38df31f156288f`

Exact post-merge master CI:

- run `34436645106`: PASS;
- event: `push`;
- head SHA: `a2f12865034c9c44e8996ee74d38df31f156288f`;
- exact merged-master frontend quality, generated-contract verification, Rust tests, Clippy/quality, dependency audit, and release/license metadata gates all completed successfully.

The fresh uploaded master snapshot also resolves to exactly `a2f12865034c9c44e8996ee74d38df31f156288f`. Local Rust execution is unavailable in the current sandbox image, and an attempted `npm ci` could not complete because the sandbox could not resolve the npm registry; those environment limitations do not replace or weaken the exact-head and exact-master CI evidence above.

Therefore every KTT-103 and KTT-104 tracker item and acceptance criterion is satisfied. P1 is complete. The next dependency-ready task is `KTT-200 — Define the production Kitten artifact manifest`.
