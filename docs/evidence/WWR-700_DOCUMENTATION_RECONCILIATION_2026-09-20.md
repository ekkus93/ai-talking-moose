# WWR-700 — Documentation reconciliation evidence

Evidence head reviewed: `4450feaebc9218fb22bf5e9e38de9714977568b2`.

## Current authoritative documentation

`docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md` is the user/developer-facing current-behavior boundary for Wake Word V1. It documents:

- one authoritative application Wake Word runtime and explicitly rejects a second command-ASR provider/runtime stack;
- the fixed phrase `Hey, Moose` and disabled-by-default policy;
- local/offline keyword spotting and the fact that the microphone may remain locally active while listening;
- the privacy boundary that raw Wake PCM is retained only in bounded in-memory ring/pre-roll buffers and is not represented by diagnostics;
- the wake phrase plus immediate command may enter the normal command ASR path after a trigger;
- Talking suspension and the V1 no-barge-in limitation;
- exact model/runtime identity and license evidence paths;
- privacy-safe diagnostics/troubleshooting fields;
- corpus and acceptance status, including explicit statements that real redistributable fixtures, Linux/macOS real KWS acceptance, integrated one-stream routing/handoff, lifecycle acceptance, and measured performance are not yet complete.

This prevents implementation scaffolding or component tests from being described as completed production acceptance.

## Truthfulness gate

`.github/workflows/wake-word-documentation-audit.yml` and its checker enforce the documentation boundary. Merged-master Wake Word documentation audit run `35487357681` passed on `01a45ce95720eac25f7fa8f22b788eaadba1786a`. Ordinary CI also passed on the current reviewed master lineage, including run `35488572188` on `4450feaebc9218fb22bf5e9e38de9714977568b2`.

The documentation audit is intentionally conservative: it requires current docs to retain explicit pending-acceptance language while WWR-300/310, WWR-600/610/620, WWR-630, and WWR-640 remain open.

## WWR-700 reconciliation

Objective source evidence supports the WWR-700 items for authoritative architecture, fixed phrase, disabled-by-default policy, local/offline KWS description, active-local-microphone disclosure, command-ASR boundary, Talking suspension, no-barge-in, memory-only pre-roll, exact artifact/runtime provenance and licenses, diagnostics/troubleshooting, correction of planned-vs-functional claims, and avoidance of unmeasured accuracy claims.

Two closeout qualifications remain deliberately open:

1. Supported-platform statements must remain limited to packaging/architecture preparation until WWR-610 and WWR-620 real KWS acceptance pass.
2. README/user-facing feature promotion must not describe Wake Word V1 as generally usable until production routing/handoff and real acceptance are complete.

Accordingly this evidence records completed documentation substance without claiming the final WWR-700 acceptance checkbox or feature-ready README promotion prematurely.
