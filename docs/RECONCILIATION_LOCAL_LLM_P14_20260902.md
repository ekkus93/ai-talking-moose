# Local LLM P14 Documentation / Developer Experience Reconciliation — 2026-09-02

## Status

**P14 implementation is complete in this tranche; merge remains gated on ordinary CI.**

P14 changes documentation/developer guidance only. It does not change provider routing, model installation, native runtime behavior, settings migration, packaging, or privacy enforcement code.

P14 starts from `master`:

`c552d35a6d3b4a4ad7cb71e170f150532da419a3`

That base already contains the accepted Local Text V1 implementation through P13, including:

- P12 real CPU acceptance run `33663595759` on source `28aef16cbeeb91d9570177111560158811730b89`;
- LLM-013 Local/SmolLM2 new-profile default merged through PR #37;
- P13 implementation ordinary CI `33677289000`;
- P13 two-architecture packaging acceptance run `33686740197` on `abc5586814feeb4351672c01a4c8d9edc6578e7a`;
- canonical P13 evidence merged through PR #39, producing this P14 base.

## LLM-140 — README

`README.md` now states the actual product boundary instead of presenting Google text generation as universal:

- fresh profiles default to Local text generation;
- SmolLM2 is selected but not automatically downloaded;
- **Download & Verify** is the explicit installation step;
- exact supported model download sizes are shown;
- canonical P12 CPU/RSS/throughput measurements are presented as host-specific evidence, not performance guarantees or a semantic-quality benchmark;
- a Google AI Studio key is not required for Local typed/ambient text generation;
- Gemini Live remains the V1 voice provider and Google TTS remains cloud-backed when used;
- Local failures do not fall back to Google/Fake;
- ordinary CI is model-weight-free and the two heavyweight Local LLM workflows are documented separately.

The architecture section also distinguishes provider-neutral Local/Google text generation from Gemini Live voice.

## LLM-141 — Local LLM architecture document

New `docs/LOCAL_LLM_ARCHITECTURE.md` records one authoritative V1 architecture description covering:

- `TextProvider` selection and legacy/new-profile semantics;
- provider-neutral typed/ambient call sites;
- `LocalTextModel` and `LocalRuntimeManager` ownership;
- catalog metadata and explicit verified installer flow;
- `<app-data>/models/llm` storage ownership and revision-scoped installs;
- no Local -> Google -> Fake fallback;
- network-enabled explicit install versus network-independent generation;
- packaging/license boundaries;
- ordinary-CI versus manual-acceptance boundaries;
- the current Gemini Live voice path;
- the explicitly deferred future `Moonshine -> Local TextModel -> speech synthesizer` seam.

The document intentionally does not imply that the future local-voice seam already exists.

## LLM-142 — Local model catalog maintenance guide

`docs/LOCAL_LLM_CATALOG.md` is expanded from a static inventory into a maintenance procedure. It now explains:

- model/artifact selection criteria;
- CPU practicality and known-template requirements;
- immutable 40-hex revision pinning;
- independent exact-byte and SHA-256 verification;
- license metadata/document synchronization;
- catalog/default/contract update steps;
- ordinary validation gates;
- when and how to dispatch **Local LLM Real CPU Acceptance**;
- why a compile/download success alone is insufficient to declare a model supported.

The guide preserves the exact accepted SmolLM2 and Qwen identities and links the recommendation to P12 runtime evidence without converting that evidence into a quality claim.

## LLM-143 — Privacy documentation

`docs/PRIVACY.md` now explicitly separates four boundaries that were previously easy to conflate:

1. local Moonshine speech recognition;
2. Local typed/ambient text generation;
3. explicit Local model download/verification;
4. Google-backed voice/text/TTS capabilities.

It records:

- Local text as the fresh-profile default;
- no auto-download on selection;
- no Local-to-Google/Fake fallback;
- no network required for Local generation after installation;
- Gemini Live voice remaining cloud-backed even when Local text is selected;
- finalized Moonshine transcript handoff to Gemini Live in V1;
- Google TTS cloud semantics;
- Local model filesystem/persistence policy;
- no-GGUF-in-bundle policy;
- prompt/output/native-error logging constraints.

`docs/GOOGLE_AI.md` is also corrected so `gemini-3.7-flash` is described as the default **Google-provider model**, not the application's new-profile text-provider default.

## LLM-144 — Agent/developer gates

`AGENTS.md` now documents:

- `ai/local/` and the Local Text V1 architecture boundary;
- `python3 scripts/check_local_llm_packaging_policy.py`;
- ordinary model-weight-free validation expectations;
- the manual real-CPU and P13 packaging workflows as separate evidence paths;
- catalog/installer/runtime/packaging/privacy change checklists;
- no silent provider/model substitution;
- Local model-root/runtime ownership quirks;
- `.gguf`/`.GGUF` Git/bundle prohibition.

`CLAUDE.md` is reconciled with those rules. In particular it no longer claims `node_modules/` and `dist/` are intentionally tracked; the repository's generated-tree gate explicitly rejects that state. It also adds the Local text privacy/fail-closed rules and P15-aware phase workflow.

## Historical TODO reconciliation

`docs/TODO(20260831-081800).md` had accumulated stale unchecked boxes even though the project had already advanced phase-by-phase through accepted P12/P13 evidence. P14 normalizes the historical phase status:

- P0 through P14 are checked complete;
- the conditional LLM-013 Google-default rationale is explicitly marked N/A because Local is the chosen default;
- **P15 remains entirely open**;
- the **Final Gate remains open**.

This is documentation reconciliation, not an attempt to pre-pass P15. P15 must re-audit the final exact `master` state, including negative probes, privacy/silent-failure review, canonical quality gates, and the final Local LLM reconciliation report.

## Validation for this tranche

Because P14 changes Markdown/documentation only, useful local validation is structural rather than a substitute for repository CI:

- `git diff --check`;
- referenced repository-path existence checks;
- phase/TODO check that P0-P14 have no open boxes while P15 remains open;
- review that no source/workflow/runtime files changed.

Ordinary CI on the P14 PR remains the merge gate and proves that documentation edits did not accidentally alter repository formatting/build inputs or conflict with current source state.
