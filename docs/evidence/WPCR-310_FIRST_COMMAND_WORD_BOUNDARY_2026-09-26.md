# WPCR-310 First Command Word Boundary Evidence

**Date:** 2026-09-26
**Remediation item:** WPCR-310 — downstream first-command-word acceptance
**Exact master inspected:** `ebeb2a118c63df4f8d71b3f41476163df80dfc3a`

## Scope

This evidence records deterministic boundary coverage for the first-command-word handoff path. It does not claim nondeterministic real-ASR transcription acceptance or final WPCR-950/960 closeout.

## Deterministic fixture boundary

The existing router test `handoff_audio_preserves_wake_phrase_tail_and_first_command_word_contiguously` in `src-tauri/src/app/wake_word_pcm_router.rs` uses explicit sample regions representing:

- wake phrase head;
- wake phrase tail plus the KWS trigger chunk;
- the immediate first command word after the wake phrase.

The test routes those regions through the production `CanonicalWakePcmRouter` boundary, forces KWS detection on the wake-phrase tail chunk, appends the first-command-word samples as post-trigger live audio, transfers a `WakeCommandHandoffAudio` payload, and asserts that the downstream payload is one contiguous chronological sample vector. The test also asserts that the KWS engine is not re-fed with post-trigger command samples.

## Downstream command-ASR boundary

The existing pipeline test `wake_handoff_primes_existing_ingress_in_exact_sample_order` in `src-tauri/src/asr/pipeline_tests.rs` primes the normal local command-ASR pipeline with a `WakeCommandHandoffAudio` payload and asserts that the existing Moonshine ingress receives the exact sample order after conversion to the worker's downstream PCM representation.

Together, these tests establish deterministic boundary receipt: the production wake router preserves the first command word in handoff audio, and the normal downstream command-ASR ingress receives wake handoff audio without clipping, duplication, or reordering.

## Evidence boundary

This is deterministic boundary evidence, not a real transcription claim. It proves that the first command word reaches the downstream local-ASR test boundary; it does not prove that a real Moonshine model transcribes `tell me the time` correctly on all native platforms. Any later real-ASR/manual/scheduled gate must state that broader scope explicitly.

## Privacy

The evidence and tests use synthetic deterministic PCM sample markers only. No raw user microphone audio, private transcript, credential, or path is persisted in this evidence.
