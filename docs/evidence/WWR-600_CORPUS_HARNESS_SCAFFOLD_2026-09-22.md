# WWR-600 corpus harness scaffold evidence — 2026-09-22

## Scope

This evidence records the deterministic Wake Word corpus contract and validation harness currently present in the repository. It does not claim real positive/negative audio fixture acceptance.

## Source evidence

`docs/wake-word-corpus.json` defines the Wake Word V1 corpus contract:

- fixed phrase `Hey, Moose`;
- canonical 16 kHz mono `pcm_s16le` policy;
- V1 score `1.0` and threshold `0.25`;
- fixture root `docs/fixtures/wake-word-v1`;
- required fixture labels for positive wake phrase, positive wake phrase with command, ordinary-speech negative, and near-miss negative;
- exact model/runtime identity fields tied to `wake-word-artifacts.json`;
- privacy policy forbidding private room audio, incidental user recordings, credentials, and unredistributable fixture leakage;
- pending acceptance thresholds until real fixture calibration exists.

`scripts/validate_wake_word_corpus.py` validates the corpus contract without reading audio. It fails closed on policy drift, artifact/runtime identity drift, path traversal, missing provenance/license/hash/byte-size metadata, non-redistributable licenses, malformed SHA-256 values, and incomplete non-empty fixture label coverage.

`scripts/test_validate_wake_word_corpus.py` exercises the validator with synthetic manifest entries and verifies repository validity before real fixtures exist.

## Current corpus state

The current manifest intentionally contains an empty `fixtures` array. This means the harness/contract exists, but no recall or false-trigger behavior is accepted yet.

## Validation

Exact merged-master validation for `69679d67b64f4f4eed9fc709760056331495f440` passed ordinary CI `35765499838`.

## Non-claims

This evidence does not claim multiple reproducible/licensable speakers, natural pronunciation variants, background-noise variants, or near-miss audio files are present. Those corpus population items remain open.

This evidence also does not claim real Linux or macOS KWS inference against positive/negative fixtures. WWR-610 and WWR-620 remain open until platform acceptance jobs run pinned native KWS inference against real redistributable fixtures.