# WWR-600 — Wake Word Corpus Identity Policy Evidence

**Date:** 2026-09-21
**Scope:** Record KWS policy and exact artifact/runtime identity in the deterministic Wake Word corpus manifest.

`docs/wake-word-corpus.json` now records KWS score `1.0`, threshold `0.25`, source artifact manifest, exact model/archive/keyword identities, sherpa runtime ID/version, and Linux x86_64/macOS arm64 C API runtime SHA-256 values. Both corpus validators compare these values against `wake-word-artifacts.json`; regression tests cover policy and identity drift.

This satisfies the WWR-600 harness requirement to record exact model/runtime identity and score/threshold policy. Positive/negative fixture collection, measured detections, false rejects/accepts, calibrated pass/fail criteria, and real platform KWS acceptance remain open.
