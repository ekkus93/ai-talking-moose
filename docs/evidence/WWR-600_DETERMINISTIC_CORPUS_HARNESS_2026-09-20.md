# WWR-600 — Deterministic Wake Word corpus harness evidence

Evidence head reviewed: `beee25b095185b36fb8c17e819073851dc7f4cd9`.

## Authoritative manifest

`docs/wake-word-corpus.json` defines the deterministic Wake Word V1 corpus contract:

- corpus id: `wake-word-v1-deterministic-corpus`;
- fixed wake phrase: `Hey, Moose`;
- sample policy: 16 kHz, mono, `pcm_s16le`;
- fixture root: `docs/fixtures/wake-word-v1`;
- fixture schema version: `1`;
- acceptance criteria version: `1`;
- required labels:
  - `positive_wake_phrase`;
  - `positive_wake_phrase_with_command`;
  - `negative_ordinary_speech`;
  - `negative_near_miss`.

The manifest intentionally keeps `positive_recall_minimum` and `negative_false_accepts_maximum` as `null` with `criteria_status: pending_real_fixture_calibration` until real redistributable fixtures are added and measured. That keeps claims limited to tested conditions and prevents the corpus contract from implying real KWS acceptance before audio exists.

## Deterministic checker

`scripts/check_wake_word_corpus_manifest.mjs` validates the manifest and fails closed on drift. It enforces:

- required top-level manifest keys;
- schema version `1`;
- fixed phrase `Hey, Moose`;
- required positive/negative label set;
- sample rate, channel count, and PCM sample format;
- fixture schema version `1`;
- explicit privacy policy forbidding private room audio;
- acceptance criteria version `1`;
- `pending_real_fixture_calibration` status while thresholds are uncalibrated;
- `null` recall/false-accept thresholds until real fixtures exist;
- per-fixture required provenance, license, bytes, SHA-256, sample policy, path, and expected detection fields;
- redistributable SPDX license evidence for every fixture;
- normalized relative POSIX paths under `docs/fixtures/wake-word-v1`;
- full required-label coverage once any fixture is added.

This closes the WWR-600 harness-level tasks for versioned pass/fail criteria, CI/report-friendly deterministic validation, and non-redistributable fixture leakage prevention. It does not close the positive/negative audio corpus items because the manifest currently has `fixtures: []`.

## CI evidence

`.github/workflows/wake-word-corpus.yml` runs the manifest checker for pull requests, master pushes, and manual dispatch when corpus-related paths change. Recent master evidence includes:

- `38633b272d54dbe007d09b81f27ceb048391e376`: Wake Word corpus validation run `35482086461` passed.
- `1a49663add48280811f8a749e1928abcbce8cdd9`: Wake Word corpus validation run `35482458468` passed after fixture/criteria versioning was added.

## Remaining open scope

The following WWR-600/610/620 work remains open and must not be inferred from this harness evidence:

- real redistributable positive fixtures;
- real redistributable negative/near-miss fixtures;
- measured positive detections, false rejects, and false accepts;
- calibrated recall/false-accept thresholds;
- Linux x86_64 real KWS inference acceptance;
- macOS arm64 real KWS inference acceptance.
