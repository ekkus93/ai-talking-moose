from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "wake_word_qualification_report.py"
CRITERIA = ROOT / "docs" / "wake_word_qualification_criteria_v1.json"
SPEC = importlib.util.spec_from_file_location("wake_word_qualification_report", SCRIPT)
wake_report = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(wake_report)

MODEL_SHA = "0" * 64
RUNTIME_SHA = "1" * 64
COMMIT_SHA = "2" * 40


def criteria() -> dict:
    return json.loads(CRITERIA.read_text())


def valid_report() -> dict:
    return {
        "schema_version": 1,
        "commit_sha": COMMIT_SHA,
        "platform": "linux-x86_64",
        "model": {
            "name": "sherpa-onnx-kws-zipformer-gigaspeech-3.3M-2024-01-01",
            "sha256": MODEL_SHA,
        },
        "runtime": {
            "name": "sherpa-onnx-native-v1.13.8-linux-x86_64",
            "sha256": RUNTIME_SHA,
        },
        "cpu_only": True,
        "offline_inference": True,
        "thread_count": 1,
        "corpus": {
            "positive_total": 1,
            "positive_detected": 1,
            "negative_total": 1,
            "negative_false_accepts": 0,
        },
        "performance": {
            "idle_cpu_percent": 0.1,
            "runtime_memory_bytes": 123456,
            "inference_ms": 12.5,
            "wake_to_asr_activation_ms": 25.0,
            "preroll_replay_ms": 2.0,
        },
        "lifecycle": {
            "cycles": 1,
            "native_session_growth": 0,
            "capture_stream_growth": 0,
            "ring_buffer_growth": 0,
            "stuck_state_count": 0,
        },
        "privacy": {
            "no_raw_audio_in_report": True,
            "no_secrets_in_report": True,
            "no_unnecessary_paths_in_report": True,
        },
    }


class WakeWordQualificationReportTests(unittest.TestCase):
    def assert_rejects(self, report: dict, expected: str) -> None:
        with self.assertRaisesRegex(ValueError, expected):
            wake_report.validate(report, criteria())

    def test_valid_report_summarizes_exact_identity_and_acceptance_rates(self):
        summary = wake_report.validate(valid_report(), criteria())
        self.assertEqual(
            summary,
            [
                f"commit={COMMIT_SHA}",
                "platform=linux-x86_64",
                "recall=1.000000",
                "false_accept_rate=0.000000",
                "cycles=1",
            ],
        )

    def test_missing_required_top_level_field_fails_closed(self):
        report = valid_report()
        report.pop("privacy")
        self.assert_rejects(report, "missing fields: privacy")

    def test_commit_sha_must_be_exact_lowercase_git_sha(self):
        report = valid_report()
        report["commit_sha"] = "ABC"
        self.assert_rejects(report, "commit_sha must be an exact lowercase")

    def test_unsupported_platform_is_rejected(self):
        report = valid_report()
        report["platform"] = "windows-x86_64"
        self.assert_rejects(report, "platform is not an allowed")

    def test_wrong_thread_count_rejects_report(self):
        report = valid_report()
        report["thread_count"] = 2
        self.assert_rejects(report, "one-thread policy")

    def test_recall_threshold_is_enforced(self):
        report = valid_report()
        report["corpus"]["positive_detected"] = 0
        self.assert_rejects(report, "recall 0.000000 is below threshold")

    def test_false_accept_rate_threshold_is_enforced(self):
        report = valid_report()
        report["corpus"]["negative_false_accepts"] = 1
        self.assert_rejects(report, "false-accept rate 1.000000 exceeds threshold")

    def test_missing_performance_metric_is_rejected(self):
        report = valid_report()
        report["performance"].pop("wake_to_asr_activation_ms")
        self.assert_rejects(report, "performance.wake_to_asr_activation_ms must be numeric")

    def test_negative_performance_metric_is_rejected(self):
        report = valid_report()
        report["performance"]["inference_ms"] = -1
        self.assert_rejects(report, "performance.inference_ms must be non-negative")

    def test_lifecycle_growth_counter_must_be_zero(self):
        report = valid_report()
        report["lifecycle"]["capture_stream_growth"] = 1
        self.assert_rejects(report, "lifecycle.capture_stream_growth must be zero")

    def test_privacy_flags_must_be_true(self):
        report = valid_report()
        report["privacy"]["no_raw_audio_in_report"] = False
        self.assert_rejects(report, "privacy.no_raw_audio_in_report must be true")

    def test_cli_returns_success_for_valid_report(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "report.json"
            path.write_text(json.dumps(valid_report()))
            self.assertEqual(wake_report.main_for_test([str(path), "--criteria", str(CRITERIA)]), 0)


if __name__ == "__main__":
    unittest.main()
