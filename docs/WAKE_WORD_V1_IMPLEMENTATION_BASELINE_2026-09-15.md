# Wake Word V1 implementation baseline

**Recorded:** 2026-09-15
**Implementation base:** `595016a09f33fee094045f5966bbddb2ea8eaa08`
**Required ancestor:** `e638daa706e63f8bbb36b514322679c1c39e19a6`
**Companion specification:** `docs/WAKE_WORD_V1_SPEC_2026-09-14.md`
**Implementation queue:** `docs/WAKE_WORD_V1_TODO_2026-09-14.md`

## Frozen V1 scope

The implementation begins from `595016a09f33fee094045f5966bbddb2ea8eaa08`, a verified descendant of the approved baseline. The following product/runtime decisions remain frozen for Wake Word V1 unless a concrete blocker is documented for owner review:

- sherpa-onnx keyword spotting is the wake engine;
- the default wake phrase is `Hey, Moose`;
- wake mode is disabled by default;
- wake-disabled operation preserves the existing manual-listen path;
- existing ASR provider selection and fallback policy remain unchanged;
- Local, Google, and Gemini speech-provider ownership remains separated;
- Local KittenTTS retains its production two-thread policy;
- wake KWS remains local/offline and does not silently invoke full-time ASR or cloud services;
- V1 does not add barge-in or acoustic echo cancellation;
- V1 does not acoustically trim the wake phrase before downstream ASR;
- wake pre-roll PCM is memory-only and must not be serialized or logged.

## Scope-control rule

Any discovery outside the approved V1 specification is to be documented as a separate follow-up rather than silently folded into Wake Word V1. Source changes must remain traceable to the specification and TODO, and consequential heads must receive exact-head qualification before guarded merge.
