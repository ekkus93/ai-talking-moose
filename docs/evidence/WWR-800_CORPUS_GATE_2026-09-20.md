# WWR-800 — deterministic corpus CI gate evidence

Date: 2026-09-20
Baseline master: `276494f4342c639e6fe292f8c3e702a72e9117a4`

## Implemented gate

PR #282 added `.github/workflows/wake-word-corpus-contract.yml`. The workflow is path-bound to the authoritative corpus manifest, validator, validator tests, and the workflow itself. It runs `python scripts/validate_wake_word_corpus.py` plus the focused unittest suite on Ubuntu with Python 3.12.

The validator freezes the V1 corpus contract at 16 kHz mono, score `1.0`, threshold `0.25`, unique fixture IDs, at least one positive fixture, an immediate-command positive, required `Moose` / `Hey Bruce` / `Hey Moosey` near misses, fixture provenance/license metadata, and explicit recall/false-accept thresholds. Focused tests prove duplicate IDs, required-near-miss removal, and policy drift fail deterministically.

## Exact-master qualification

On exact master `276494f4342c639e6fe292f8c3e702a72e9117a4`:

- Wake Word corpus contract run `35522895158` completed successfully.
- Ordinary CI run `35522895150` completed successfully.

This objectively closes the WWR-800 requirement to define a deterministic corpus CI/validation gate. It also establishes a deterministic, CI-friendly manifest/contract portion of WWR-600.

## Limits intentionally not claimed

The manifest currently describes fixture paths and acceptance policy; this evidence does not claim the referenced WAV fixtures exist or that real KWS inference has run over them. Therefore WWR-600 corpus-content/measurement acceptance, WWR-610 Linux real KWS acceptance, WWR-620 macOS real KWS acceptance, and the WWR-800 real-platform/performance/lifecycle gates remain open.
