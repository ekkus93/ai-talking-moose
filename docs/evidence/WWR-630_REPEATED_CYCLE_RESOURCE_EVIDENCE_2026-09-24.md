# WWR-630 repeated-cycle resource evidence

**Date:** 2026-09-24
**Scope:** Partial WWR-630 performance evidence for repeated-cycle resource behavior.
**Status:** Partial evidence recorded; WWR-630 remains open.

## Exact evidence

- Exact merged `master` commit: `755d02b738742513419778043b524e8b94f9c340`.
- Source implementation merge: PR #448, merge commit `755d02b738742513419778043b524e8b94f9c340`.
- Exact-master Wake Word lifecycle stability run: `36090294124`.
- Exact-master lifecycle job: `107931090203`.

The lifecycle job emitted the following privacy-safe metric line from the repeated-cycle stability test:

```text
WWR630_REPEATED_CYCLE_RESOURCE_DELTA cycles=100 initial_ring_buffer_samples=0 final_ring_buffer_samples=0 ring_buffer_delta=0 initial_handoff_pre_roll_samples=0 final_handoff_pre_roll_samples=0 handoff_pre_roll_delta=0 trigger_count_delta=100 final_phase=Listening capacity_samples=32000
```

## Interpretation

The repeated wake lifecycle scenario completed 100 cycles without growth in retained ring-buffer samples or handoff pre-roll samples. The runtime returned to `Listening`, and the retained-audio capacity remained bounded at 32,000 samples.

This evidence supports the WWR-630 repeated-cycle resource-behavior requirement. It does **not** complete WWR-630 by itself. The accepted performance baseline still requires the remaining representative performance measurements and the continuous-ASR comparison described in `docs/wake-word-performance-evidence.json`.

## Privacy and policy notes

The metric reports counts, phase, and bounded capacity only. It does not include raw PCM, transcripts, model inputs, device identifiers, credentials, or filesystem paths.

The one-thread KWS policy is unchanged.
