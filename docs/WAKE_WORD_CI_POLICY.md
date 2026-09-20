# Wake Word specialized CI and qualification policy

Wake Word V1 cannot qualify from ordinary CI alone. Final qualification is an **exact-head** decision over ordinary CI plus every Wake-specific gate required by the final diff and the remediation TODO. A skipped required gate is not a pass.

## Gate matrix

| Gate | Purpose | Execution class | Current status |
| --- | --- | --- | --- |
| Deterministic corpus manifest | Validate corpus schema, phrase/audio policy, provenance/license fields, fixture containment/hashes/outcomes, and versioned acceptance policy | GitHub-hosted Ubuntu; automatic for relevant paths and manually dispatchable | Implemented in `.github/workflows/wake-word-corpus.yml` |
| Wake artifact/model/runtime identity | Verify pinned identities, hashes, safe preparation, and native architecture contracts | Existing Wake artifact/identity workflows | Implemented; retain exact-run evidence |
| Linux x86_64 real KWS | Execute the production sherpa KWS path with real positive and negative fixtures, CPU-only and offline after preparation | Specialized Linux x86_64 acceptance runner/job | Required before Linux acoustic-support claim; not yet closed |
| macOS arm64 real KWS | Execute the production sherpa KWS path with real positive and negative fixtures, CPU-only and offline after preparation | Specialized macOS arm64 acceptance runner/job | Required before macOS arm64 acoustic-support claim; not yet closed |
| Native packaging/architecture | Prove the packaged application locates the pinned native runtime and rejects wrong/corrupt architecture | Platform packaging/acceptance jobs | Required for final supported-platform qualification |
| Integrated lifecycle stability | Repeated wake→ASR→Thinking→Talking→wake cycles, disable/enable, cancellation/failure, shutdown, bounded ring/session/stream state | Integration acceptance environment with audio/runtime support | Required; not yet closed |
| Performance evidence | CPU, memory, inference, handoff/pre-roll latency, repeated-cycle behavior, and comparison with continuous full ASR | Representative Linux and macOS acceptance environments | Evidence/report gate; not yet closed |
| Privacy/security audit | Verify ownership, cancellation, ring clearing, provider separation, exact artifacts, architecture, offline idle inference, and no raw-audio/secret/path leakage | Source + acceptance evidence review | Required at final feature head |
| Documentation audit | Verify user/developer docs state only behavior and platform claims proven on the final head | Source/evidence review | Required at final feature head |

## Exact-head rule

Every qualification record must name the exact 40-character commit SHA it tested. A result from an ancestor does not qualify a later head unless the gate's documented policy proves that the later diff cannot affect that gate and the final evidence explicitly records that determination. For final Wake Word V1 qualification, prefer rerunning required gates on the exact final head rather than relying on ancestry.

Before merge, the PR head SHA must equal the SHA associated with the required successful runs/reports. After merge, required exact-master validation must be rerun according to the remediation closeout policy.

## Skipped, cancelled, timed-out, or missing runs

A required gate is successful only when its required jobs/report conclude successfully for the qualifying SHA. `skipped`, `cancelled`, `timed_out`, missing, or merely queued/in-progress results are **not** successes. Ordinary CI success must never be used to substitute for a missing specialized Wake gate.

A path-filtered workflow being skipped can be acceptable only when that workflow is not required by the final diff/qualification matrix. Final closeout must record that decision explicitly; it must not silently translate a skipped workflow into a pass.

## Runner and hardware requirements

The deterministic corpus-manifest gate needs no microphone or model inference hardware and runs on GitHub-hosted Ubuntu.

Real KWS gates require the target OS/architecture, the pinned model/runtime preparation path, and real redistributable corpus fixtures. They must exercise CPU-only production inference with the frozen one-thread policy and prove inference remains offline after artifact preparation. Linux acceptance requires x86_64; macOS acceptance requires arm64.

Integrated lifecycle acceptance additionally requires an environment capable of exercising the application's production audio/lifecycle path. If deterministic virtual/injected audio is used for repeatability, the evidence must distinguish that from any physical-device claim.

Performance evidence requires representative target environments and must record enough runner/platform detail to make measurements interpretable. Performance measurements from one machine are evidence for that environment, not universal guarantees.

## Merge eligibility

A Wake Word feature head is merge-eligible for **final V1 closeout** only when:

1. ordinary CI succeeds on the exact head;
2. every required Wake-specific gate in the matrix succeeds on that exact head or has an explicitly justified unaffected-diff determination permitted by the final policy;
3. real KWS gates contain both positive and negative fixture evidence rather than component-only tests;
4. privacy/security and documentation audits are complete;
5. no required result is skipped, cancelled, timed out, missing, or still running;
6. the corpus/model/runtime identities referenced by the evidence match the exact head; and
7. the remediation and original Wake Word TODOs are reconciled without converting design-only evidence into implementation completion.

Ordinary feature PRs may continue to merge incrementally under repository policy while remediation is in progress. Such incremental merges are not a declaration that Wake Word V1 has passed final qualification.
