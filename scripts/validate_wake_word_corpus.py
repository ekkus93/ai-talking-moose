#!/usr/bin/env python3
"""Guard the compatibility pointer to the canonical Wake Word corpus manifest."""
from __future__ import annotations
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
POINTER = ROOT / "wake-word-corpus.json"
CANONICAL = ROOT / "docs" / "wake-word-corpus.json"


def validate(data: dict) -> None:
    assert data == {
        "schema_version": 1,
        "deprecated": True,
        "canonical_manifest": "docs/wake-word-corpus.json",
        "note": "Compatibility pointer only. Wake Word corpus policy, fixture metadata, provenance, licensing, and acceptance criteria are authoritative only in the canonical manifest.",
    }
    canonical = json.loads(CANONICAL.read_text(encoding="utf-8"))
    assert canonical["schema_version"] == 1
    assert canonical["corpus_id"] == "wake-word-v1-deterministic-corpus"
    assert canonical["wake_phrase"] == "Hey, Moose"


def main() -> None:
    validate(json.loads(POINTER.read_text(encoding="utf-8")))
    print(f"validated compatibility pointer -> {CANONICAL.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
