# Wake Word V1 third-party notices and provenance boundary

**Recorded:** 2026-09-16
**TODO:** `WW-100`, `WW-110`, and `WW-900` in `docs/WAKE_WORD_V1_TODO_2026-09-14.md`
**Status:** Notice/provenance requirements frozen; production notices still require independently verified artifact bytes

## Scope

Wake Word V1 uses sherpa-onnx keyword spotting for local/offline detection of the fixed phrase `Hey, Moose`. This document records the repository-local notice and provenance requirements that must be satisfied before any model or native runtime artifact is production-eligible.

The project must not treat an artifact as production-ready merely because it appears in a CI cache, was downloaded from a familiar URL, or matches an expected filename. Production eligibility requires immutable byte identity, license/provenance review, and a corresponding notice entry.

## Selected third-party components

The current V1 selection and runtime policy identify the following third-party components:

| Component | Role | Current frozen decision | Production notice status |
| --- | --- | --- | --- |
| `sherpa-onnx` native runtime | CPU keyword-spotting runtime loaded from Rust/Tauri | release `v1.13.8` for initial Linux x86_64 and macOS arm64 targets | Open until exact runtime archive/library bytes and license text are recorded |
| `sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01` | English keyword-spotting model family | fp32 encoder/decoder/joiner plus `tokens.txt` and `bpe.model` | Open until exact model archive/consumed-file bytes and model-license provenance are recorded |
| GigaSpeech-derived training material | Upstream model provenance | Identified by upstream model naming/documentation as the source corpus family | Open until the selected model artifact's license/provenance metadata is reviewed directly |
| BPE tokenizer artifacts | Keyword preparation for `HEY MOOSE` | `tokens.txt` and `bpe.model` consumed from the selected model archive | Open until exact bytes and any applicable tokenizer/model notice obligations are recorded |

## Notice requirements before production use

Before WW-100/WW-110 can close, a follow-up commit must add a production notice record containing at least:

1. exact model archive URL, byte size, and SHA-256;
2. exact consumed model file paths, byte sizes, and SHA-256 values;
3. exact generated or verified `HEY MOOSE` keyword token representation;
4. exact sherpa runtime archive URLs, byte sizes, SHA-256 values, and supported platforms;
5. exact consumed native library paths, byte sizes, SHA-256 values, and architecture headers;
6. license names and repository-local license text or notice excerpts for every redistributed component;
7. provenance notes for the model artifact that distinguish runtime license from model/data license;
8. a statement of whether the application redistributes the artifacts, prepares them at install time, or requires a user/operator-managed artifact cache;
9. confirmation that no GPL/AGPL or otherwise incompatible production dependency is introduced by the Wake Word V1 artifact set;
10. confirmation that CI cache hits do not bypass byte/hash/license verification.

## Required repository-local files

The final production-ready state should include these repository-local records or their exact equivalents:

- `wake-word-artifacts.json` populated with immutable archive and consumed-file identities.
- A third-party notice document that names each redistributed or required runtime/model artifact.
- Any required license text or notice file referenced by the notice document.
- Evidence that the deterministic preparation and verification scripts reject missing, corrupt, wrong-architecture, or unhashed artifacts.
- Evidence that real Linux x86_64 and macOS arm64 KWS acceptance used only those verified identities.

## Fail-closed rule

Until the notice and provenance records exist, Wake Word V1 must remain in the current fail-closed state: the manifest may define policy and expected structure, but production KWS inference must not silently download, load, or execute unverified artifacts.

A later implementation may add installation assistance, cache restoration, or release packaging, but each path must converge on the same manifest verifier before inference. No packaging path may substitute an artifact based only on filename, URL, cache key, modification time, or release label.

## User-facing documentation rule

User documentation may say that Wake Word V1 is intended to run local/offline only after verified runtime and model artifacts are installed. It must not claim measured accuracy, false-trigger rates, platform support, or production readiness beyond the exact acceptance evidence present in the repository.
