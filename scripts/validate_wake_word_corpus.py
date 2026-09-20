#!/usr/bin/env python3
"""Validate the deterministic Wake Word V1 corpus contract without reading audio."""
from __future__ import annotations
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "wake-word-corpus.json"
REQUIRED_NEGATIVES = {"Moose", "Hey Bruce", "Hey Moosey"}


def validate(data: dict) -> None:
    assert data["schema_version"] == 1
    assert data["wake_phrase"] == "Hey, Moose"
    assert data["sample_rate_hz"] == 16000
    assert data["channels"] == 1
    assert data["score"] == 1.0
    assert data["threshold"] == 0.25
    fixtures = data["fixtures"]
    ids = [item["id"] for item in fixtures]
    assert len(ids) == len(set(ids)), "fixture ids must be unique"
    assert any(item["class"] == "positive" for item in fixtures)
    assert any(item["class"] == "positive" and item["variant"] == "immediate-command" for item in fixtures)
    negatives = {item["text"] for item in fixtures if item["class"] == "negative"}
    assert REQUIRED_NEGATIVES <= negatives
    for item in fixtures:
        assert item["class"] in {"positive", "negative"}
        assert item["path"].startswith("fixtures/wake-word/") and item["path"].endswith(".wav")
        assert item["provenance"]
        assert item["license"]
    acceptance = data["acceptance"]
    assert 0.0 <= acceptance["minimum_positive_recall"] <= 1.0
    assert 0.0 <= acceptance["maximum_negative_false_accept_rate"] <= 1.0


def main() -> None:
    validate(json.loads(MANIFEST.read_text(encoding="utf-8")))
    print(f"validated {MANIFEST.name}")


if __name__ == "__main__":
    main()
