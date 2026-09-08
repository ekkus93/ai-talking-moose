# Local LLM Remediation P7 Reconciliation — 2026-09-08

## Status

**P7 adversarial/concurrency/UI regression matrix is prepared for exact-head validation on the accepted P6-closure generation.**

This record covers `LLMR-700` through `LLMR-703`. It does not close the authoritative TODO until focused/exact-head validation, expected-head merge, and exact merged-master CI are all accepted.

Implementation base: `52637770d1636582f915c0098bce50c8a899eded`, the P6 tracker-closure `master` produced by guarded squash merge of PR #60. Its post-merge master CI is `34274316120` and must succeed before P7 is merged.

The P7 frontend regression was originally authored and focused-validated in run `34055131603`. The final old P7 tree was compared against the original P4 base and contains exactly two intended files: this reconciliation record and `src/test/LocalLlmSettingsPanel.test.tsx`. The pre-P7 test blob on current `master` is still `0facb262e570dc64124f9c3023b3c321cff5b1fd`, byte-identical to the original P7 base, so the strengthened test is transplanted unchanged from validated blob `fff5d748959e27325f6f00a2ead5ec2eda440230`.

## LLMR-700 — Installer adversarial matrix

P1/P2 production-path tests already cover the complete matrix required by P7:

- wrong/same-size-wrong SHA and byte-count failures;
- truncated and oversized responses;
- cancellation during download, verification, after verification/before promotion, and the promotion/marker boundary;
- stale staging cleanup and successful reinstall after cancellation;
- traversal and symlink/path escape defenses;
- atomic promotion failure and duplicate concurrent install handling;
- HTTPS-to-HTTP redirect rejection with a finite hop limit;
- same-size post-install mutation rejected before runtime load.

Those tests use the production installer/runtime paths and do not authorize Google, Fake, or another Local model on failure.

## LLMR-701 — Frontend cancellation end-state proof

The previous `LocalLlmSettingsPanel` test only proved that the cancel IPC command was sent and then explicitly resolved the same cancelled operation as `installed`. P7 replaces that false-positive shape with a deterministic lifecycle proof:

1. start an install and enter the `verifying` phase;
2. issue Cancel and prove the backend cancel command is called;
3. reject the same install operation with the cancellation result;
4. prove the UI returns to a non-installed state with `Download & Verify` available and no installed-success message;
5. start a **new** install operation and prove that retry can then succeed.

This test would fail if the UI again treated an acknowledged cancellation as allowing the same operation to become installed.

## LLMR-702 — Request-snapshot concurrency matrix

P4 added barrier-controlled production-path tests for typed and ambient requests. The typed test captures Local/provider/model/memory/transcript/personality settings A, mutates them to Google/settings B while paused, proves the in-flight request remains internally A, then proves the next request observes B. The ambient test similarly covers provider, Local model identity, memory, privacy observation, and behavior/talkativeness while preserving the post-generation current-state privacy suppression boundary.

P4 exact-head, guarded merge, post-merge master validation, and tracker closure are already accepted. No timing sleeps are used for the A/B consistency proof.

## LLMR-703 — Diagnostics truthfulness matrix

Merged P3 diagnostics tests and frontend coverage prove production telemetry across unloaded/idle, loaded+generating, successful generation metrics, safe error categories, installer phases (including verifying), and unload/delete. The barrier-controlled runtime diagnostics test also proves prompt/system-private-context/output sentinels never enter the serialized diagnostics payload while positive controls prove the production telemetry path is live.

## Required closure evidence

- focused frontend regression validation remains represented by authoring run `34055131603` and must also pass in ordinary exact-head CI on the rebased P7 head;
- P6 closure master CI `34274316120` succeeds on exact base `52637770d1636582f915c0098bce50c8a899eded` before P7 merge;
- ordinary P7 exact-head CI passes, including frontend quality, Rust quality, dependency/release gates, Local LLM compile proofs, both macOS bundles, and canonical `npm run check:all`;
- expected-head merge and exact merged-master CI pass before P7 tracker closure.
