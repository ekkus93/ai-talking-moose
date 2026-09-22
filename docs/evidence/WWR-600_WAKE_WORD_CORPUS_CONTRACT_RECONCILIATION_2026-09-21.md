# WWR-600 — Wake Word Corpus Contract Reconciliation Evidence

**Date:** 2026-09-21
**Scope:** Align the legacy Python corpus contract gate with the current deterministic Wake Word V1 corpus manifest.

## Change

The Python corpus validator now reads `docs/wake-word-corpus.json`, validates the current schema, and remains fixture-content-free: it checks manifest metadata, fixture provenance/license/hash/path fields, expected-detection booleans, and pending acceptance criteria without reading audio bytes.

The corpus contract workflow path filters now target the same manifest used by the active Node corpus validation gate, so schema drift in `docs/wake-word-corpus.json` triggers both corpus gates.

## Regression coverage

`python scripts/validate_wake_word_corpus.py` and `PYTHONPATH=scripts python -m unittest scripts/test_validate_wake_word_corpus.py` validate:

- the current empty-but-calibration-pending repository manifest;
- complete synthetic fixture metadata covering all required labels;
- duplicate fixture IDs;
- fixture path traversal/normalization rejection;
- policy drift rejection.

## TODO reconciliation note

This advances WWR-600 harness determinism and WWR-800 corpus gate coverage. It does not claim real positive/negative KWS acceptance; fixture collection, measured recall/false-accept thresholds, and platform inference runs remain open.
