# ASR-015 Supported-Mac Evidence — 2026-08-23

This directory permanently preserves the compact acceptance evidence from GitHub Actions run `32665369708` (`ASR-015 Native Acceptance`) at commit `f181a1d18ab65eecfddf875f6706d9bce5f136fb`.

The original workflow artifact is `asr015-supported-mac-f181a1d18ab65eecfddf875f6706d9bce5f136fb`, artifact ID `9500044584`, recorded digest `sha256:b8964a048667fb638fd9e4fc39dc3825a5fac68afd7458e1e5b6ca68c2c9764d`. GitHub reports that artifact expires on `2026-08-30T20:45:04Z`, so the non-redundant evidence needed to audit P3A is checked into the repository here.

Preserved files:

- `ASR015_SUPPORTED_MAC_ACCEPTANCE.md` — rendered acceptance report;
- `asr015-records.jsonl` — all 12 raw benchmark records (Tiny and Small: one warm-up + five measured runs each);
- `asr015-hardware.json` — reference hardware/macOS metadata;
- `asr015-corpus.json` — immutable corpus provenance and derived PCM identity.

The per-run Cargo logs remain in the time-limited Actions artifact because their acceptance-relevant measurements are already represented in the raw JSONL and rendered report.

SHA-256 of the preserved files as downloaded from artifact `9500044584`:

- `ASR015_SUPPORTED_MAC_ACCEPTANCE.md`: `c1cc02d95632d69387f7410c0d397bc02da9c59954fdab35b1e092fedb21f4a0`
- `asr015-corpus.json`: `bf9cc6f80e573c984b41d0b58c839e161733547031f372789426057ac8f92a13`
- `asr015-hardware.json`: `d58bbbd4f93426ed1448cd0d9557432cc970b8c0f79cc0c648ed754ab304f3df`
- `asr015-records.jsonl`: `2924391fe6764ed19e6ebd29aed67dfd5350451526f9ad041d10e291906444a0`

Gate disposition is documented in `docs/RECONCILIATION_P3A_20260823.md`.