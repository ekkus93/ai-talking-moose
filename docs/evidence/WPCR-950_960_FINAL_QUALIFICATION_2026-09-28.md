# WPCR-950 / WPCR-960 — Final qualification and exact-master verification

Date: 2026-09-28

## Exact tested master

Exact final qualification and exact-master verification head: `bba5ea44a668f67baf32f0471dff2d75bb72cf22`

This head is on `master` and is the post-closeout final-qualification trigger/re-run head. It changes no Wake product behavior, Wake policy, corpus recipes, artifact identity, runtime identity, KWS threshold, KWS score, or acceptance criteria. The commit content is limited to validation-script trigger comments, the narrow privacy-audit false-positive exception described below, and a lifecycle trigger note so all required Wake workflows run on the same exact SHA.

Because this remediation is being completed directly on `master`, there is no final PR merge step to perform. WPCR-960 is therefore interpreted as exact tested `master` verification: the exact `master` head above passed ordinary CI and all required Wake-specific final gates.

## Privacy-audit false-positive refinement

The first exact-head trigger `0ecc73d6c0607206c94aed98abe9c67075742351` selected the full gate set. It surfaced one failure in Wake Word privacy audit run `36391312118`: the audit flagged `format!("{:?}", final_snapshot.phase)` in `src-tauri/src/app/wake_word_acceptance.rs` as a generic `{:?}` formatting risk.

The flagged expression formats only the internal Wake runtime phase enum into the production-listener performance acceptance report. It does not serialize filesystem paths, credentials, raw audio, transcripts, private audio content, or user data. The final head `bba5ea44a668f67baf32f0471dff2d75bb72cf22` therefore adds a narrow audit allow-list entry for that exact expression and no broader exemption.

## WPCR-950 exact-head final qualification evidence

| Required gate | Workflow/run | Result | Scope |
| --- | ---: | --- | --- |
| Ordinary CI | `CI` run `36391804709` | Passed | Frontend quality, Rust quality, Rust tests, and ordinary selected checks for the exact head. |
| Settings/listener lifecycle acceptance | `CI` run `36391804709` | Passed | Ordinary CI Rust/frontend tests covering Settings enable/disable, persistence/listener rollback, diagnostics, selected ASR policy, manual transfer, and downstream first-command-word deterministic paths. |
| Manual shared-capture transfer acceptance | `CI` run `36391804709` | Passed | Ordinary CI Rust coverage for command ownership transfer, listener stop/resume boundaries, and duplicate microphone stream prevention. |
| Selected ASR policy acceptance | `CI` run `36391804709` | Passed | Ordinary CI and disclosure tests proving Wake-triggered command ASR remains local-Moonshine-only and unsupported modes fail before listening. |
| Downstream first-command-word acceptance | `CI` run `36391804709` | Passed | Deterministic router/local-ASR ingress tests proving wake pre-roll and first command word reach downstream command boundary in order. |
| Wake Artifact Verification | `Wake Artifact Verification` run `36391804721` | Passed | Artifact/manifest verification on the exact head. |
| Deterministic corpus manifest | `Wake Word corpus validation` run `36391804786` | Passed | Corpus manifest, provenance, license, privacy, and active acceptance criteria. |
| Deterministic corpus contract | `Wake Word corpus contract` run `36391804740` | Passed | Python companion validation for `docs/wake-word-corpus.json`. |
| Native packaging / clean-install artifact provisioning | `Wake Word native packaging architecture` run `36391804824` | Passed | Linux x86_64 and macOS arm64 artifact manifest, architecture, runtime preparation, and clean-install fail-closed policy checks. |
| Lifecycle stability / integrated production lifecycle | `Wake Word lifecycle stability` run `36391804798` | Passed | Deterministic runtime-manager and listener lifecycle stability tests on exact head. |
| Performance evidence policy | `Wake Word performance evidence` run `36391804722` | Passed | Performance-report schema, claim scope, and evidence policy checks. |
| Production listener performance evidence | `Wake Word real KWS acceptance` run `36391804703` | Passed | Production listener performance acceptance was executed in the real-KWS workflow on Linux x86_64 and macOS arm64. |
| Privacy audit | `Wake Word privacy audit` run `36391804693` | Passed | Diagnostics/log/error/privacy boundaries, active corpus criteria, and private-audio guardrails. |
| Source-security audit | `Wake Word source security audit` run `36391804817` | Passed | Runtime ownership, capture routing, memory-only PCM retention, command activation/ASR ingress, artifact/runtime verification, native architecture, and offline/provider separation. |
| Documentation audit | `Wake Word documentation audit` run `36391804809` | Passed | Behavior, architecture, UI disclosure, README boundary, performance status, and gate documentation truthfulness. |
| Required-gates audit | `Wake Word required gates audit` run `36391804741` | Passed | Required-gate inventory, exact-head requirement, workflow existence, skipped-gate policy, and post-closeout gate truthfulness. |
| Linux x86_64 real KWS acceptance | `Wake Word real KWS acceptance` run `36391804703` | Passed | Real pinned sherpa KWS inference on Linux x86_64, including command fixture detection and production-listener performance acceptance. |
| macOS arm64 real KWS acceptance | `Wake Word real KWS acceptance` run `36391804703` | Passed | Real pinned sherpa KWS inference on macOS arm64, including command fixture detection and production-listener performance acceptance. |
| Measured performance acceptance | `Wake Word real KWS acceptance` run `36391804703` | Passed | Platform report extraction plus production-listener performance acceptance reports for Linux x86_64 and macOS arm64. |

All required gates were selected on the exact same `master` SHA. No skipped required gate was counted as passing.

## Report artifacts

`Wake Word real KWS acceptance` run `36391804703` uploaded these exact-head artifacts:

- `wake-word-v1-corpus` — artifact `10955834039`
- `wake-word-real-kws-linux-x86_64` — artifact `10956920657`
- `wake-word-real-kws-macos-arm64` — artifact `10956920850`

The Linux and macOS artifacts include the real KWS acceptance reports and `*-production-listener.json` production-listener performance reports produced by the exact-head workflow.

## Diff and regression boundary

The exact final qualification head is a validation/evidence trigger head. It does not alter:

- Wake Word runtime behavior;
- listener control-plane behavior;
- Settings persistence or rollback behavior;
- command handoff, ASR ingress, or manual conversation transfer behavior;
- KWS model/runtime identities;
- corpus fixture definitions or acceptance criteria;
- KWS score, threshold, sample rate, channel count, or inference-thread policy;
- TTS, ASR, local-LLM, memory, or unrelated application runtime behavior.

The only executable checker change at final head is the privacy-audit narrowing for an internal enum formatting expression in an acceptance report path, described above.

## WPCR-960 exact-master verification

The exact tested head `bba5ea44a668f67baf32f0471dff2d75bb72cf22` was already on `master` when the gates above completed. Therefore, the guarded-merge portions of WPCR-960 are not applicable for this direct-to-master remediation run; no PR merge was performed or required.

Exact-master verification is satisfied by the same run set above because the exact head was `master` at execution time:

- ordinary CI passed on exact `master`;
- every required Wake-specific final gate passed on exact `master`;
- Linux and macOS real KWS acceptance passed on exact `master`;
- production-listener performance acceptance passed on exact `master`;
- privacy, source-security, documentation, and required-gates audits passed on exact `master`;
- no skipped required gate was counted as passing.

This evidence closes WPCR-950 and WPCR-960 subject only to canonical TODO reconciliation.