# WWR-310 command handoff audio boundary evidence — 2026-09-21

This evidence records an additional deterministic acceptance slice for the Wake Word V1 wake→command-ASR handoff boundary.

## Scope

The test-only `RecordingEngine` in `src-tauri/src/app/wake_word_pcm_router.rs` models a KWS detection on a selected canonical PCM chunk without using raw real-user audio. The production router is still the subject under test: it routes the same canonical stream to ring retention and KWS, snapshots the retained pre-roll when a trigger is accepted, preserves live post-trigger samples while command ASR starts, and transfers a provider-neutral `WakeCommandHandoffAudio` payload exactly once.

## Added acceptance

`handoff_audio_preserves_wake_phrase_tail_and_first_command_word_contiguously` proves that a synthetic wake phrase tail plus the immediate first command word become one chronological command-ASR audio payload:

- pre-trigger samples are retained in order;
- the detected trigger frame remains present in the returned payload;
- post-trigger live samples are appended after the trigger frame;
- the returned `WakeCommandHandoffAudio` has canonical 16 kHz metadata;
- adjacent synthetic sample windows remain contiguous, proving no gap, duplicate range, or inversion at the snapshot/live boundary;
- post-trigger live PCM is not fed back into KWS.

`repeated_positive_frames_after_trigger_do_not_duplicate_command_activation` proves that a second positive-like PCM frame after trigger acceptance is treated as live handoff audio rather than a second wake activation. The engine receives only the original accepted trigger frame, the transferred payload remains chronological, and the runtime trigger count remains one.

## TODO mapping

This evidence supports reconciliation of the deterministic portions of WWR-310 and WWR-410 after exact-head CI passes:

- preserve the full wake phrase material present in the two-second window;
- preserve the immediate first command word;
- prevent gaps, duplicates, and inversion at the handoff boundary;
- keep V1 acoustic trimming disabled;
- preserve the one wake event → one command activation invariant for repeated positive-like frames after trigger acceptance.

This evidence does not claim real fixture acceptance, Linux/macOS native sherpa acceptance, performance acceptance, or end-to-end UI/runtime lifecycle completion.
