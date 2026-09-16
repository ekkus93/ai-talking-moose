# Wake Word V1 scope and baseline evidence

**Recorded:** 2026-09-16
**TODO:** `WW-000` in `docs/WAKE_WORD_V1_TODO_2026-09-14.md`
**Companion spec:** `docs/WAKE_WORD_V1_SPEC_2026-09-14.md`
**Frozen baseline:** `e638daa706e63f8bbb36b514322679c1c39e19a6`
**Status:** WW-000 scope/baseline evidence reconciled; implementation remains governed by the companion TODO

## Baseline traceability

Wake Word V1 was specified against `e638daa706e63f8bbb36b514322679c1c39e19a6` on `master`. All Wake Word implementation work reviewed in this reconciliation is on verified descendants of that baseline. The baseline recorded in the approved specification and TODO therefore remains the authoritative implementation base; later source slices do not redefine scope.

## Frozen V1 decisions

The implementation remains constrained by the approved companion specification:

- sherpa-onnx KWS is the V1 wake engine;
- the user-visible default wake phrase is `Hey, Moose`, with engine normalization such as `HEY MOOSE` permitted only for tokenizer requirements;
- wake mode is persisted and disabled by default;
- wake-disabled mode preserves the existing manual-listen path;
- wake failures fail closed and must not alter ASR provider selection or introduce a hidden cloud/full-ASR fallback;
- Google, Gemini, and Local TTS remain separate provider paths;
- Local TTS retains its independent 2-thread production policy; KWS starts from its own explicit 1-thread policy;
- V1 has no barge-in or acoustic echo cancellation requirement;
- V1 does not acoustically trim the wake phrase from pre-roll audio;
- the two-second PCM pre-roll remains memory-only and must not be persisted or logged.

These constraints are not implementation suggestions: they are the frozen V1 product boundary. Any later discovery that would require changing one of them must be documented as a separate out-of-scope/blocker decision rather than silently incorporated into Wake Word V1.

## Evidence already present on current master

Current `master` contains repository-local evidence that the implementation continues to follow the frozen boundary:

- `docs/WAKE_WORD_V1_SPEC_2026-09-14.md` records the approved product/runtime/privacy scope and baseline.
- `docs/WAKE_WORD_V1_TODO_2026-09-14.md` decomposes that scope into explicit `WW-*` implementation and acceptance items.
- `docs/WAKE_WORD_V1_SHERPA_MODEL_SELECTION_2026-09-16.md` keeps the engine fixed to sherpa-onnx, the English phrase fixed to `Hey, Moose`, KWS at one thread, and production artifacts fail-closed until immutable identities are qualified.
- persisted settings work keeps wake disabled by default and validates/normalizes the fixed phrase without changing unrelated ASR/TTS settings.
- the generic PCM ring-buffer work is sherpa-independent and memory-only by contract.

## WW-000 reconciliation

The WW-000 baseline and scope requirements are therefore satisfied as governance/evidence requirements. This does **not** imply that later WW-100 through WW-970 implementation or acceptance tasks are complete. In particular, artifact/runtime pinning, real sherpa inference, lifecycle integration, UI, corpus acceptance, platform acceptance, performance evidence, final audits, and final exact-head/exact-master closeout remain governed by their own TODO items.

If future work discovers a requirement that conflicts with the frozen decisions above, record that discovery separately and require an explicit scope decision before changing V1 behavior.
