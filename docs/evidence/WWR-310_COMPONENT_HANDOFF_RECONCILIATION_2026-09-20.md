# WWR-310 — component wake→ASR handoff evidence

Date: 2026-09-20
Baseline master: `e5fdc2db21a8671f4ca6c91a872bd3af4a407225`

## Scope

This evidence records the component-level wake→ASR handoff behavior that is objectively present on current `master`. It deliberately does **not** claim end-to-end production wake activation, production capture-consumer wiring, real audio fixture acceptance, or downstream ASR transcription acceptance.

## Source evidence

`src-tauri/src/app/wake_word_pcm_router.rs` contains `CanonicalWakePcmRouter`, which routes a single caller-owned canonical 16-kHz mono PCM timeline into both Wake Word retained pre-roll and the KWS engine. The router owns no capture device and performs no second microphone open.

When KWS detection is accepted, the router calls `take_triggered_pre_roll()` and creates a `WakeAsrHandoff`. While that handoff is present, subsequent canonical microphone chunks are appended as live post-trigger audio and are not fed back into KWS. `transfer_handoff_to_asr()` removes the handoff and returns one chronological vector containing pre-roll followed by live post-trigger PCM. A second transfer returns `None`, making the handoff single-use. `clear_handoff()` drops stale buffered audio after startup failure/cancellation boundaries. `return_to_wake_listening()` clears handoff state, resets the KWS stream, and resumes the runtime so stale command audio cannot trigger a later interaction.

## Test evidence

The router unit tests cover these component requirements:

- `same_canonical_chunks_reach_ring_and_kws_in_order` proves the same canonical chunks reach ring retention and KWS in order.
- `detection_atomically_moves_runtime_out_of_listening_and_starts_handoff` proves accepted detection moves runtime out of Listening and starts handoff.
- `post_trigger_chunks_are_preserved_for_command_asr_without_refeeding_kws` proves live post-trigger samples are preserved for command ASR and are not re-fed to KWS.
- `handoff_transfer_is_single_use_and_return_resumes_listening` proves one-shot ASR transfer, ownership return, KWS reset, and later listening resume.
- `invalid_post_trigger_frame_does_not_mutate_live_handoff` proves invalid frames cannot contaminate live handoff.
- `clearing_handoff_drops_stale_audio_after_failed_startup` proves stale handoff audio is dropped after failed startup/cancellation-style cleanup.
- `disabled_runtime_never_feeds_kws` proves disabled runtime does not feed KWS.

## Reconciled component coverage

The current source supports the component-level portions of WWR-310 for chronological pre-roll snapshotting, post-trigger live preservation, no gap/duplication at the router boundary, stale handoff clearing after startup failure/cancellation, and recoverable return-to-listening after handoff failure.

The source also supports the component-level WWR-300/410 invariants that ring retention and KWS consume the same chronological canonical stream, the router transfers ownership to command ASR exactly once, and KWS state resets before returning to wake listening.

## Explicitly still open

These WWR-310 requirements remain open at the production/integration level:

- automatically starting the normal command-ASR path from an accepted Wake Word trigger;
- replaying the router-returned PCM into the live production command-ASR pipeline;
- real/reproducible `Hey Moose, tell me the time` audio acceptance;
- downstream proof that the first command word is present;
- end-to-end no-gap/no-duplicate/no-inversion acceptance across the actual production ASR provider boundary.

These boundaries must be implemented and qualified before WWR-310 or the WWR-300/400 integrated lifecycle acceptance can be marked complete.
