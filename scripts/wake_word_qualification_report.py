#!/usr/bin/env python3
"""Fail-closed validator for Wake Word V1 qualification reports.

The native/corpus/lifecycle runners emit JSON; this script validates identity,
coverage, and explicit pass/fail criteria without retaining audio. It is stdlib
only so Linux and macOS acceptance jobs can use the same reporting boundary.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

SHA256 = re.compile(r"^[0-9a-f]{64}$")
GIT_SHA = re.compile(r"^[0-9a-f]{40}$")
REQUIRED_TOP = {
    "schema_version", "commit_sha", "platform", "model", "runtime", "corpus",
    "performance", "lifecycle", "privacy", "offline_inference", "cpu_only",
    "thread_count",
}


def fail(message: str) -> None:
    raise ValueError(message)


def require_dict(value, name: str) -> dict:
    if not isinstance(value, dict):
        fail(f"{name} must be an object")
    return value


def require_number(value, name: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        fail(f"{name} must be numeric")
    return float(value)


def validate_identity(obj: dict, name: str) -> None:
    if not obj.get("name") or not isinstance(obj["name"], str):
        fail(f"{name}.name must be non-empty")
    digest = obj.get("sha256")
    if not isinstance(digest, str) or not SHA256.fullmatch(digest):
        fail(f"{name}.sha256 must be a lowercase SHA-256")


def validate(report: dict, criteria: dict) -> list[str]:
    missing = sorted(REQUIRED_TOP - report.keys())
    if missing:
        fail("missing fields: " + ", ".join(missing))
    if report["schema_version"] != criteria["schema_version"]:
        fail("schema_version does not match criteria")
    if not isinstance(report["commit_sha"], str) or not GIT_SHA.fullmatch(report["commit_sha"]):
        fail("commit_sha must be an exact lowercase 40-character git SHA")
    if report["platform"] not in criteria["allowed_platforms"]:
        fail("platform is not an allowed acceptance platform")
    validate_identity(require_dict(report["model"], "model"), "model")
    validate_identity(require_dict(report["runtime"], "runtime"), "runtime")
    if report["cpu_only"] is not True:
        fail("cpu_only must be true")
    if report["offline_inference"] is not True:
        fail("offline_inference must be true")
    if report["thread_count"] != criteria["thread_count"]:
        fail("thread_count violates the V1 one-thread policy")

    corpus = require_dict(report["corpus"], "corpus")
    positive = int(require_number(corpus.get("positive_total"), "corpus.positive_total"))
    detected = int(require_number(corpus.get("positive_detected"), "corpus.positive_detected"))
    negative = int(require_number(corpus.get("negative_total"), "corpus.negative_total"))
    false_accepts = int(require_number(corpus.get("negative_false_accepts"), "corpus.negative_false_accepts"))
    if positive < criteria["corpus"]["min_positive"] or negative < criteria["corpus"]["min_negative"]:
        fail("corpus does not meet minimum positive/negative fixture counts")
    if not (0 <= detected <= positive and 0 <= false_accepts <= negative):
        fail("corpus counts are internally inconsistent")
    recall = detected / positive
    false_accept_rate = false_accepts / negative
    if recall < criteria["corpus"]["min_recall"]:
        fail(f"recall {recall:.6f} is below threshold")
    if false_accept_rate > criteria["corpus"]["max_false_accept_rate"]:
        fail(f"false-accept rate {false_accept_rate:.6f} exceeds threshold")

    perf = require_dict(report["performance"], "performance")
    for key in criteria["required_performance_metrics"]:
        value = require_number(perf.get(key), f"performance.{key}")
        if value < 0:
            fail(f"performance.{key} must be non-negative")

    lifecycle = require_dict(report["lifecycle"], "lifecycle")
    if int(require_number(lifecycle.get("cycles"), "lifecycle.cycles")) < criteria["lifecycle"]["min_cycles"]:
        fail("lifecycle cycle count is below threshold")
    for key in criteria["lifecycle"]["required_zero_counters"]:
        if require_number(lifecycle.get(key), f"lifecycle.{key}") != 0:
            fail(f"lifecycle.{key} must be zero")

    privacy = require_dict(report["privacy"], "privacy")
    for key in criteria["privacy_required_true"]:
        if privacy.get(key) is not True:
            fail(f"privacy.{key} must be true")

    return [
        f"commit={report['commit_sha']}",
        f"platform={report['platform']}",
        f"recall={recall:.6f}",
        f"false_accept_rate={false_accept_rate:.6f}",
        f"cycles={lifecycle['cycles']}",
    ]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path)
    parser.add_argument("--criteria", type=Path, default=Path("docs/wake_word_qualification_criteria_v1.json"))
    args = parser.parse_args()
    try:
        report = require_dict(json.loads(args.report.read_text()), "report")
        criteria = require_dict(json.loads(args.criteria.read_text()), "criteria")
        summary = validate(report, criteria)
    except (OSError, json.JSONDecodeError, ValueError, KeyError) as exc:
        print(f"WAKE_QUALIFICATION_FAIL: {exc}", file=sys.stderr)
        return 1
    print("WAKE_QUALIFICATION_PASS " + " ".join(summary))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
