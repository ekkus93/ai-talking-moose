# WWR-310 command-ASR payload reconciliation — 2026-09-20

This evidence records the bounded WWR-310 implementation state on `master` after PRs #289, #290, and #291. It intentionally does **not** claim the production capture-consumer/orchestration loop is complete.

## Implemented component boundary

`CanonicalWakePcmRouter` already owns the deterministic trigger handoff state. On an accepted trigger it takes the runtime's chronological pre-roll snapshot into `WakeAsrHandoff`; while command ASR is starting, subsequent canonical chunks are appended to that handoff rather than being re-fed to KWS. Transfer is single-use and removes the handoff from the router.

PR #290 added `WakeCommandHandoffAudio`, a provider-neutral command-ASR payload that accepts only non-empty canonical 16-kHz PCM, preserves the exact chronological `i16` vector, performs no V1 acoustic wake-phrase trimming, and converts the same samples to little-endian PCM bytes without reordering or duplication.

PR #291 connected the router's single-use transfer directly to that validated payload boundary through `transfer_handoff_audio_to_asr`. Its regression coverage proves the trigger chunk and post-trigger live chunk emerge in exact chronological order, the byte representation preserves those samples, and a second transfer returns no payload.

## Qualification

- PR #290 exact head `519630d3136993c33231c357a5dd4d309b02b172`: ordinary CI `35530719539` passed; P21-P23 skipped by path policy.
- PR #290 merged master `13f993e9455dab785af5783d8e325a38239ef13c`: ordinary CI `35531149753` passed.
- PR #291 exact head `99f93cef16aa507ffb1a186644da9d85a579814c`: ordinary CI `35531914570` passed; P21-P23 skipped by path policy.
- PR #291 merged master `84e827baef8bf576690820feac4f5743f47b9d93`: ordinary CI `35532377178` passed.

## What this objectively closes at component level

The component implementation now demonstrates chronological pre-roll snapshot ownership, post-trigger live preservation, a no-trim V1 payload, exact sample-order preservation across the snapshot/live boundary, no duplicate transfer, and a concrete provider-neutral payload seam for normal command ASR.

## Still open — do not overclaim

WWR-310 is **not** production-complete. The application still needs the production capture-consumer/orchestration layer that continuously routes the one authoritative canonical microphone stream into the native KWS router while idle, reacts to an accepted trigger exactly once, starts the selected normal command-ASR path, primes that path with the transferred `WakeCommandHandoffAudio`, continues live PCM without a gap, and returns ownership to Wake listening on every terminal success/failure/cancellation path.

Real `Hey Moose, tell me the time` downstream-ASR acceptance and first-command-word acceptance also remain open. Therefore the WWR-310 production acceptance checkboxes must remain unchecked until that integration and acceptance evidence exists.
