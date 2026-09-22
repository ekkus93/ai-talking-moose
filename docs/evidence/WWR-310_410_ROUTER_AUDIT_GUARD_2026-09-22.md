# WWR-310 / WWR-410 Router Audit Guard

Date: 2026-09-22

This evidence records an additional source-security audit guard for deterministic Wake Word router invariants that are already covered by Rust tests.

## Scope

The audit now requires source-level presence of the deterministic router tests that cover:

- chronological wake phrase tail plus immediate first command word preservation;
- repeated positive KWS frames after a trigger not producing duplicate command activation;
- a later phrase after return to `Listening` producing a later trigger without adding a cooldown requirement; and
- disabled Wake runtime never feeding KWS.

## Boundaries

This is source-level regression evidence for WWR-310 and WWR-410. It does not claim real/reproducible audio acceptance, downstream ASR transcription accuracy, Linux/macOS real KWS acceptance, or final WWR-950/960 closeout.

Those remain governed by the corpus, real-platform KWS, performance, integrated lifecycle, and final exact-master acceptance tasks.
