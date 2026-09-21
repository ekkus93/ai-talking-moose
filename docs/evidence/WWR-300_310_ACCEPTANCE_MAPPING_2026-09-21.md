# WWR-300/WWR-310 acceptance evidence mapping — 2026-09-21

This file reconciles the objective evidence now present on master after PR #317 and PR #318. It is intentionally conservative: it records which remediation TODO requirements have source/test evidence and which requirements remain open.

## Qualified commits and CI

- Ownership reconciliation evidence merged via PR #317 at master `8b69451e7edfbcb717da55e1b247238081d61b45`; exact merged-master CI `35619133665` passed.
- Command handoff audio boundary tests merged via PR #318 at master `bcf29ce6b1a55bb11cdd211c518526b490dbafdf`; exact PR-head CI `35623229571` passed on `ac291483e5419f863dff66443e65da5dbdf3e987`.

## WWR-300 evidence-backed items

The PR #317 evidence file `docs/evidence/WWR-300_OWNERSHIP_ACCEPTANCE_RECONCILIATION_2026-09-21.md` supports the following WWR-300 requirements:

- Wake Word does not open a competing continuous microphone stream at the implemented ownership boundary.
- Wake start/disable, command handoff, command return, restart, disable, cancellation, unavailable-device startup failure, and recoverable restart operate through the shared application capture owner.
- Ownership transfer to command ASR stops the shared capture before command ASR takes ownership.
- Ownership return to Wake uses the same shared capture owner.
- Reconnect from a failed command-return path restores Wake listening through the same owner.
- Unavailable-device startup failure fails closed by leaving capture stopped and clearing stale orchestration state.
- Cancellation does not orphan or multiply capture streams in the covered ownership paths.
- Wake disable releases capture so manual listen can subsequently acquire it.

The following WWR-300 items are still not claimed complete by this mapping:

- Complete end-to-end production lifecycle wiring beyond the covered ownership boundary.
- Single canonical microphone resampling/canonicalization at the physical capture boundary.
- Real device-disconnect handling distinct from startup/unavailable-device failure.
- A repeated wake→ASR→wake stream-count invariant beyond pointer-identity and active-state tests.

## WWR-310 evidence-backed items

The PR #318 tests in `src-tauri/src/app/wake_word_pcm_router.rs` support the following WWR-310 requirements:

- Accepted trigger snapshots the chronological ring contents through the existing `WakeWordRuntimeManager::accept_trigger` and `take_triggered_pre_roll` path.
- Subsequent live canonical samples are preserved while command ASR startup is pending.
- Pre-roll and live PCM are transferred as one validated `WakeCommandHandoffAudio` payload.
- The handoff transfer prevents gaps, duplicate sample ranges, and ordering inversion in the synthetic exact-boundary acceptance.
- The untrimmed wake phrase tail remains present in the transferred payload.
- The immediate first command word remains present in the transferred payload.
- V1 does not acoustically trim the Wake Word before command ASR handoff.
- Repeated positive PCM after the accepted trigger is treated as live handoff audio rather than a second command activation.

The following WWR-310 items remain open:

- Activation of the existing normal command ASR path exactly once from the integrated production runtime.
- Real/reproducible `Hey Moose, tell me the time` audio acceptance.
- Downstream ASR acceptance proving the first command word survives beyond the provider-neutral audio payload boundary.
- End-to-end production acceptance that the existing command ASR receives the payload through its normal path.

## WWR-410 evidence-backed items

The PR #318 repeated-positive test supports the deterministic debounce invariant at the PCM-router boundary:

- One accepted wake event creates one trigger count.
- Positive-looking post-trigger PCM does not re-feed KWS or create a duplicate activation.
- No cooldown is needed for this correctness property at the covered boundary.

WWR-410 still needs later lifecycle-level evidence that a second phrase after a completed resume yields a later command interaction in the integrated production runtime.

## Closeout note

This mapping exists so a future TODO reconciliation can update `docs/WAKE_WORD_V1_REMEDIATION_TODO_2026-09-17.md` with precise evidence instead of marking broad sections complete from memory. It should not be used to claim real corpus, real Linux/macOS KWS, or full end-to-end lifecycle acceptance.
