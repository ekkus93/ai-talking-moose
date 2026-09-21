# WWR-400/WWR-410 reconciliation evidence — 2026-09-21

This note records objective evidence that exists on `master` after PR #326 and PR #327. It is intentionally narrow: it does not claim real microphone, real KWS audio, full end-to-end production command ASR, or physical TTS self-wake acceptance.

## Qualified commits and CI

- PR #326 merged as `4f16b892ed6f2bfbb9807ac78fac26eeb8baef1b` with exact PR-head CI `35634849862` passing on `6d4c0524dcb546888957b1054914d19a4ab80fa1` and exact merged-master CI `35635649071` passing on `4f16b892ed6f2bfbb9807ac78fac26eeb8baef1b`.
- PR #327 merged as `a05df5286c4b3e65508658d3f60567e62c8ea882` with exact PR-head CI `35639339678` passing on `31b47094f713d18a2fe7f6bda5e796a3763a72b5` and exact merged-master CI `35640329089` passing on `a05df5286c4b3e65508658d3f60567e62c8ea882`.

## WWR-400 evidence added by PR #326

`src-tauri/src/app/wake_word_command_lifecycle.rs` and `src-tauri/src/commands/conversation/wake_word_lifecycle_tests.rs` now prove the following deterministic lifecycle boundary:

- A Wake Word runtime in `Error` state does not seize or block the normal/manual command-interaction ownership boundary.
- `suspend_for_command_interaction` returns `false` for Wake error state, preserving manual command interaction instead of attempting Wake ownership.
- This supports the WWR-400 test requirement “Wake error does not break manual listen” at the source-level command lifecycle boundary.

This evidence does not claim that every production UI, physical microphone, or ASR-provider path has been end-to-end exercised under real hardware/audio conditions.

## WWR-410 evidence added by PR #327

`src-tauri/src/app/wake_word_pcm_router.rs` now proves the following deterministic debounce/lifecycle behavior:

- One accepted wake phrase creates one trigger count and one handoff payload.
- Returning to Wake listening clears the handoff and resets the KWS stream.
- A later phrase after return to `Listening` can produce a second trigger count and second handoff payload.
- No cooldown timer is required for correctness at the covered PCM-router lifecycle boundary.

This evidence supports these WWR-410 items at the covered deterministic router boundary:

- Reset KWS stream at the verified lifecycle point.
- Permit a later phrase after return to Listening.
- One phrase with repeated positive frames yields one interaction, from prior PR #318 evidence.
- A second phrase after resume yields a second interaction, from PR #327 evidence.
- No cooldown is needed for correctness tests at the covered boundary.
- Debounce behavior is lifecycle-correct rather than timer-masking a state bug at the covered boundary.

This evidence does not claim full production microphone, command-ASR, or real-audio corpus acceptance.

## Remaining gaps

The remediation TODO should still leave real-audio, real-platform, production command-ASR, physical TTS self-wake, performance, and final audit items open until separately implemented and qualified. In particular, WWR-200 real positive/negative KWS fixtures, WWR-300 production microphone ownership acceptance, WWR-310 end-to-end command ASR handoff acceptance, and WWR-600+ corpus/platform gates remain open.
