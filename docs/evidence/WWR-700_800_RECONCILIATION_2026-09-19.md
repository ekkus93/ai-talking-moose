# WWR-700 / WWR-800 Reconciliation Evidence

**Evidence head:** `b6b5ffa3af8be0227f9d3efeef2f02d1c8e130d6`

This note reconciles the documentation and CI-policy requirements that have objective evidence on current `master`. It deliberately does not claim real corpus/platform/performance acceptance that has not run.

## WWR-700 — documentation

Objective evidence exists for the following requirements in `docs/WAKE_WORD_V1_CURRENT_BEHAVIOR.md`:

- fixed phrase `Hey, Moose`;
- disabled-by-default policy;
- local/offline keyword-spotting intent and explicit statement that idle Wake Word is not full-time cloud transcription;
- active local microphone disclosure;
- disclosure that the wake phrase and immediate command may enter normal command ASR after a trigger;
- Talking suspension;
- no Wake Word barge-in in V1;
- bounded memory-only ring/pre-roll behavior;
- model/runtime provenance and separate license references;
- diagnostics/troubleshooting fields and privacy constraints;
- explicit limitation of platform, real-fixture, performance, and user-readiness claims to evidence actually obtained;
- no subjective accuracy claim.

The same document identifies the authoritative application runtime and states that Wake Word is not a second command-ASR provider or an independent always-on full-ASR stream.

Still open and not claimed complete here:

- README/user-facing quick-start integration when the feature is fully usable;
- final whole-repository documentation audit after real platform/lifecycle/performance acceptance;
- any supported-platform statement beyond the real acceptance ultimately obtained.

## WWR-800 — specialized Wake gates

`docs/WAKE_WORD_V1_CI_GATES.md` defines the required gate inventory and final qualification policy.

Implemented now:

- ordinary CI remains required for Wake changes;
- `.github/workflows/wake-word-corpus.yml` is a deterministic corpus-manifest validation gate;
- corpus input/checker/workflow changes select that gate by path;
- skipped workflows are explicitly not acceptance evidence;
- final Wake Word V1 qualification cannot use ordinary CI alone.

The corpus workflow passed on exact merged master `38633b272d54dbe007d09b81f27ceb048391e376` as run `35482086461`, and again after criteria versioning on exact merged master `1a49663add48280811f8a749e1928abcbce8cdd9` as run `35482458468`.

Defined but still pending implementation/acceptance:

- Linux x86_64 real KWS gate;
- macOS arm64 real KWS gate;
- final packaged native architecture gate;
- integrated repeated lifecycle stability gate;
- performance evidence gate;
- final privacy/security audit gate.

The policy records the evidence each pending gate must produce. A component/unit test is not substituted for a missing real acceptance run.

## WWR-600 prerequisites now objectively complete

The deterministic corpus infrastructure has these completed prerequisites:

- `docs/wake-word-corpus.json` has explicit manifest, fixture-schema, and acceptance-criteria versions;
- `scripts/check_wake_word_corpus_manifest.mjs` fails closed on schema/version drift;
- any fixture entry must record provenance, redistributable SPDX license evidence, byte size, SHA-256, canonical audio policy, and expected detection outcome;
- fixture paths are constrained to `docs/fixtures/wake-word-v1`;
- the manifest privacy policy explicitly forbids private room audio;
- real recall/false-accept thresholds remain unset until real redistributable fixtures exist.

This is deterministic and CI/report friendly infrastructure, not corpus acceptance. The fixture array is still empty, so positive/negative recall and false-accept acceptance remain open.

## Qualification evidence

- `38633b272d54dbe007d09b81f27ceb048391e376`: ordinary CI `35482086483`, Wake corpus `35482086461` — both passed.
- `1a49663add48280811f8a749e1928abcbce8cdd9`: ordinary CI `35482458483`, Wake corpus `35482458468` — both passed.
- `4772f506b9a525176da3efdac812ef74f2c076c8`: current-behavior documentation merged; ordinary CI `35482845997` passed.
- `e0500df0e7a7e15edf959783287af6e493ef7be8`: CI-gate policy merged; ordinary CI `35483145701` passed.
- `b6b5ffa3af8be0227f9d3efeef2f02d1c8e130d6`: specialized qualification policy merged; ordinary CI `35483383618` passed.

This evidence is intended to support later checkbox reconciliation without converting pending real acceptance into documentation-only completion.
