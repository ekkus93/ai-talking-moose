#!/usr/bin/env python3
"""Validate the deterministic Wake Word V1 corpus contract without reading audio."""
from __future__ import annotations

import json
import posixpath
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs" / "wake-word-corpus.json"
ARTIFACT_MANIFEST = ROOT / "wake-word-artifacts.json"
REQUIRED_LABELS = {
    "positive_wake_phrase",
    "positive_wake_phrase_with_command",
    "negative_ordinary_speech",
    "negative_near_miss",
}
REQUIRED_FIXTURE_FIELDS = {
    "id",
    "label",
    "path",
    "speaker_or_source",
    "provenance",
    "license",
    "bytes",
    "sha256",
    "sample_rate_hz",
    "channels",
    "sample_format",
    "expected_detection",
}


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def _require_object(value: Any, name: str) -> dict[str, Any]:
    _require(isinstance(value, dict), f"{name} must be an object")
    return value


def _require_sha256(value: Any, name: str) -> None:
    _require(
        isinstance(value, str)
        and len(value) == 64
        and value == value.lower()
        and all(char in "0123456789abcdef" for char in value),
        f"{name} must be lowercase hex sha256",
    )


def _runtime_c_api_sha256(artifact_manifest: dict[str, Any], platform_key: str) -> str:
    platform = artifact_manifest["runtime"]["platforms"][platform_key]
    for file in platform["files"]:
        if "sherpa-onnx-c-api" in file["path"]:
            return file["sha256"]
    raise AssertionError(f"artifact manifest missing {platform_key} C API file")


def validate(data: dict[str, Any]) -> None:
    artifact_manifest = json.loads(ARTIFACT_MANIFEST.read_text(encoding="utf-8"))
    model = next(
        artifact for artifact in artifact_manifest["artifacts"] if artifact["kind"] == "kws-model"
    )

    _require(data.get("schema_version") == 1, "schema_version must be 1")
    _require(data.get("corpus_id") == "wake-word-v1-deterministic-corpus", "unexpected corpus_id")
    _require(data.get("wake_phrase") == "Hey, Moose", "wake_phrase must be Hey, Moose")

    policy = _require_object(data.get("policy"), "policy")
    _require(policy.get("sample_rate_hz") == 16_000, "policy.sample_rate_hz must be 16000")
    _require(policy.get("channels") == 1, "policy.channels must be 1")
    _require(policy.get("sample_format") == "pcm_s16le", "policy.sample_format must be pcm_s16le")
    _require(policy.get("score") == 1.0, "policy.score must be 1.0")
    _require(policy.get("threshold") == 0.25, "policy.threshold must be 0.25")
    _require(
        policy.get("fixture_root") == "docs/fixtures/wake-word-v1",
        "policy.fixture_root must be docs/fixtures/wake-word-v1",
    )
    _require(policy.get("fixture_schema_version") == 1, "fixture_schema_version must be 1")
    _require(
        "Do not commit private room audio" in str(policy.get("privacy", "")),
        "privacy policy must forbid private room audio",
    )

    identity = _require_object(data.get("model_runtime_identity"), "model_runtime_identity")
    _require(identity.get("source_manifest") == "wake-word-artifacts.json", "identity source drift")
    _require(identity.get("model_id") == model["id"], "identity model_id must match artifact manifest")
    _require(
        identity.get("model_archive_sha256") == model["archive"]["sha256"],
        "identity model_archive_sha256 must match artifact manifest",
    )
    _require(
        identity.get("keyword_sha256") == model["keyword"]["sha256"],
        "identity keyword_sha256 must match artifact manifest",
    )
    _require(identity.get("runtime_id") == artifact_manifest["runtime"]["id"], "runtime_id drift")
    _require(
        identity.get("runtime_version") == artifact_manifest["runtime"]["version"],
        "runtime_version drift",
    )
    _require(
        identity.get("linux_x86_64_c_api_sha256")
        == _runtime_c_api_sha256(artifact_manifest, "linux-x86_64"),
        "linux C API identity drift",
    )
    _require(
        identity.get("macos_arm64_c_api_sha256")
        == _runtime_c_api_sha256(artifact_manifest, "macos-arm64"),
        "macOS C API identity drift",
    )
    for key, value in identity.items():
        if key.endswith("sha256"):
            _require_sha256(value, f"identity.{key}")

    required_labels = data.get("required_labels")
    _require(isinstance(required_labels, list), "required_labels must be a list")
    _require(REQUIRED_LABELS <= set(required_labels), "required_labels is incomplete")

    criteria = _require_object(data.get("acceptance_criteria"), "acceptance_criteria")
    _require(criteria.get("criteria_version") == 1, "criteria_version must be 1")
    _require(
        criteria.get("criteria_status") == "pending_real_fixture_calibration",
        "criteria_status must remain pending until real fixture calibration",
    )
    _require(criteria.get("positive_recall_minimum") is None, "positive recall must remain uncalibrated")
    _require(
        criteria.get("negative_false_accepts_maximum") is None,
        "negative false-accept threshold must remain uncalibrated",
    )

    fixtures = data.get("fixtures")
    _require(isinstance(fixtures, list), "fixtures must be a list")
    ids: set[str] = set()
    labels_seen: set[str] = set()
    for fixture in fixtures:
        fixture = _require_object(fixture, "fixture")
        missing = REQUIRED_FIXTURE_FIELDS - set(fixture)
        _require(not missing, f"fixture is missing fields: {sorted(missing)}")
        fixture_id = fixture["id"]
        _require(isinstance(fixture_id, str) and fixture_id, "fixture id must be non-empty")
        _require(fixture_id not in ids, f"duplicate fixture id {fixture_id}")
        ids.add(fixture_id)

        label = fixture["label"]
        _require(label in REQUIRED_LABELS, f"unknown fixture label {label}")
        labels_seen.add(label)
        _require(fixture["sample_rate_hz"] == policy["sample_rate_hz"], f"{fixture_id}: sample rate drift")
        _require(fixture["channels"] == policy["channels"], f"{fixture_id}: channel drift")
        _require(fixture["sample_format"] == policy["sample_format"], f"{fixture_id}: sample format drift")
        _require(isinstance(fixture["bytes"], int) and fixture["bytes"] > 0, f"{fixture_id}: bytes must be positive")
        _require_sha256(fixture["sha256"], f"{fixture_id}: sha256")
        _require(isinstance(fixture["expected_detection"], bool), f"{fixture_id}: expected_detection must be boolean")
        _require_object(fixture["provenance"], f"{fixture_id}.provenance")
        license_info = _require_object(fixture["license"], f"{fixture_id}.license")
        _require(license_info.get("spdx"), f"{fixture_id}: license.spdx is required")
        _require(license_info.get("redistributable") is True, f"{fixture_id}: license must be redistributable")

        fixture_path = fixture["path"]
        _require(isinstance(fixture_path, str), f"{fixture_id}: path must be a string")
        normalized = posixpath.normpath(fixture_path)
        _require(normalized == fixture_path, f"{fixture_id}: path must be normalized")
        _require(not normalized.startswith("../") and not posixpath.isabs(normalized), f"{fixture_id}: path must be relative")
        _require(normalized.startswith(f"{policy['fixture_root']}/"), f"{fixture_id}: path escapes fixture root")

    if fixtures:
        _require(REQUIRED_LABELS <= labels_seen, "non-empty corpus must cover all required labels")


def main() -> None:
    validate(json.loads(MANIFEST.read_text(encoding="utf-8")))
    print(f"validated {MANIFEST.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
