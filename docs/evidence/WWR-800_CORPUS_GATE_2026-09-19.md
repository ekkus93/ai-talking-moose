# WWR-800 deterministic corpus gate evidence

Date: 2026-09-19

This evidence covers only the deterministic corpus manifest CI gate. It does not claim that real positive/negative fixtures, Linux/macOS real KWS acceptance, lifecycle stability, or performance gates are complete.

## Implementation

PR #240 added the versioned corpus contract in `docs/wake-word-corpus.json` and the deterministic validator in `scripts/check_wake_word_corpus_manifest.mjs`.

PR #242 added `.github/workflows/wake-word-corpus.yml`. The workflow is path-bound to the corpus manifest, Wake Word fixture tree, validator, and workflow itself, and also supports explicit dispatch. Its job runs the repository validator directly on the exact checked-out commit.

The validator freezes the V1 corpus schema and canonical audio policy, requires fixture provenance and redistributable SPDX evidence, constrains fixture paths to `docs/fixtures/wake-word-v1`, requires immutable byte count/SHA-256 identities, requires positive/negative labels when fixtures exist, and deliberately keeps recall/false-accept thresholds in `pending_real_fixture_calibration` until real fixtures are committed.

## Exact-head qualification

PR #242 exact head: `7a22173f72ba00e223544d7d79646498b788a6a2`.

- Ordinary CI run `35482024389`: success.
- Wake Word corpus validation run `35482024413`: success.

PR #242 merged as `38633b272d54dbe007d09b81f27ceb048391e376`.

## Exact-master verification

For merged master `38633b272d54dbe007d09b81f27ceb048391e376`:

- Ordinary CI run `35482086483`: success.
- Wake Word corpus validation run `35482086461`: success.

## Scope conclusion

Objective evidence now exists for the WWR-800 item `Define deterministic corpus CI/validation gate`, and the gate is exact-commit bound by GitHub Actions checkout/run identity. Corpus quality/recall acceptance remains open until real licensed fixtures and calibrated criteria exist.
